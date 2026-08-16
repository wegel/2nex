#!/bin/sh
# Check that the package configuration audit ledger covers every tracked
# package manifest exactly once, files each record in the matching area file,
# and uses only the result states that the audit README defines.
#
# This checker proves ledger coverage and record shape. It cannot prove that a
# record describes the package's real configuration readers.
set -eu

AUDIT_DIR_DEFAULT=.agents/audits/package-configuration

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

# Print one line per audit record: PATH<TAB>AREA<TAB>RESULT<TAB>FIELD_COUNT.
# FIELD_COUNT counts how many of the seven required labels the record carries.
extract_records() {
    audit_dir=$1

    find "$audit_dir" -maxdepth 1 -name '*.md' ! -name 'README.md' \
        | LC_ALL=C sort \
        | while IFS= read -r area_file; do
            LC_ALL=C awk -v area_file="$area_file" '
                function flush() {
                    if (path == "") {
                        return
                    }
                    printf "%s\t%s\t%s\t%d\n", path, area, result, seen_count
                    path = ""
                }
                function note(label) {
                    if (path == "" || (label in seen)) {
                        return
                    }
                    seen[label] = 1
                    seen_count += 1
                }
                BEGIN {
                    area = area_file
                    sub(/.*\//, "", area)
                    sub(/\.md$/, "", area)
                    path = ""
                }
                /^### `pkg\/.*\.yaml`[ \t]*$/ {
                    flush()
                    path = $0
                    sub(/^### `/, "", path)
                    sub(/`[ \t]*$/, "", path)
                    result = "MISSING"
                    seen_count = 0
                    delete seen
                    next
                }
                /^### / {
                    flush()
                    next
                }
                /^Purpose:/ { note("Purpose"); next }
                /^Runtime configuration:/ { note("Runtime configuration"); next }
                /^Ownership class:/ { note("Ownership class"); next }
                /^Reader behavior:/ { note("Reader behavior"); next }
                /^Evidence:/ { note("Evidence"); next }
                /^Proof:/ { note("Proof"); next }
                /^Result:[ \t]*/ {
                    if (path == "") {
                        next
                    }
                    value = $0
                    sub(/^Result:[ \t]*/, "", value)
                    sub(/[ \t]*$/, "", value)
                    result = value
                    note("Result")
                    next
                }
                END { flush() }
            ' "$area_file"
        done
}

check_audit() {
    work=$(mktemp -d)
    check_audit_body "$1" "$2" "$3" "$work" && status=0 || status=$?
    rm -rf "$work"
    return "$status"
}

check_audit_body() {
    manifest_list=$1
    audit_dir=$2
    final=$3
    work=$4

    [ -f "$manifest_list" ] || die "manifest list not found: $manifest_list"
    [ -d "$audit_dir" ] || die "audit directory not found: $audit_dir"

    extract_records "$audit_dir" > "$work/records"
    LC_ALL=C cut -f1 "$work/records" | LC_ALL=C sort > "$work/audited"
    LC_ALL=C sort "$manifest_list" > "$work/tracked"

    failed=0

    LC_ALL=C uniq -d "$work/audited" > "$work/duplicates"
    if [ -s "$work/duplicates" ]; then
        printf 'error: duplicate audit heading:\n' >&2
        sed 's/^/  /' "$work/duplicates" >&2
        failed=1
    fi

    LC_ALL=C comm -23 "$work/tracked" "$work/audited" > "$work/missing"
    if [ -s "$work/missing" ]; then
        printf 'error: tracked manifest has no audit heading:\n' >&2
        sed 's/^/  /' "$work/missing" >&2
        failed=1
    fi

    LC_ALL=C comm -13 "$work/tracked" "$work/audited" > "$work/stale"
    if [ -s "$work/stale" ]; then
        printf 'error: audit heading names no tracked manifest:\n' >&2
        sed 's/^/  /' "$work/stale" >&2
        failed=1
    fi

    while IFS="$(printf '\t')" read -r path area result fields; do
        expected_area=$(printf '%s\n' "$path" | LC_ALL=C cut -d/ -f2)
        if [ "$area" != "$expected_area" ]; then
            printf 'error: %s belongs in %s.md but appears in %s.md\n' \
                "$path" "$expected_area" "$area" >&2
            failed=1
        fi
        case "$result" in
            'pass'|'gap fixed'|'gap'|'uncertain'|'not audited') ;;
            'MISSING')
                printf 'error: %s has no Result line\n' "$path" >&2
                failed=1
                ;;
            *)
                printf 'error: %s has unknown result "%s"\n' "$path" "$result" >&2
                failed=1
                ;;
        esac
        if [ "$final" = 'yes' ]; then
            case "$result" in
                'pass'|'gap fixed') ;;
                *)
                    printf 'error: %s is not finished: result "%s"\n' \
                        "$path" "$result" >&2
                    failed=1
                    ;;
            esac
            if [ "$fields" -ne 7 ]; then
                printf 'error: %s carries %s of 7 required fields\n' \
                    "$path" "$fields" >&2
                failed=1
            fi
        fi
    done < "$work/records"

    [ "$failed" -eq 0 ] || return 1

    tracked_count=$(LC_ALL=C wc -l < "$work/tracked" | tr -d ' ')
    printf 'PASS: %s audit records cover every tracked package manifest\n' \
        "$tracked_count"
    LC_ALL=C cut -f3 "$work/records" | LC_ALL=C sort | LC_ALL=C uniq -c \
        | while read -r count state; do
            printf '  %s: %s\n' "$state" "$count"
        done
}

write_record() {
    printf '### `%s`\n\n' "$1"
    shift
    for field in "$@"; do
        printf '%s\n\n' "$field"
    done
}

full_record() {
    write_record "$1" \
        'Purpose: example' \
        'Runtime configuration: none' \
        'Ownership class: no file-based runtime configuration' \
        'Reader behavior: not applicable' \
        'Evidence: the manifest' \
        'Proof: read the manifest' \
        "Result: $2"
}

self_test() {
    test_root=$(mktemp -d)
    trap 'rm -rf "$test_root"' EXIT HUP INT TERM
    mkdir -p "$test_root/audit"

    printf '%s\n' \
        'pkg/libs/system/attr.yaml' \
        'pkg/libs/system/glibc.yaml' \
        'pkg/net/wifi/iwd.yaml' \
        > "$test_root/manifests"

    printf '# libs\n\n' > "$test_root/audit/libs.md"
    full_record 'pkg/libs/system/attr.yaml' 'pass' >> "$test_root/audit/libs.md"
    full_record 'pkg/libs/system/glibc.yaml' 'gap fixed' >> "$test_root/audit/libs.md"
    printf '# net\n\n' > "$test_root/audit/net.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'pass' >> "$test_root/audit/net.md"
    printf '# README\n\n### `pkg/libs/system/attr.yaml`\n\nResult: pass\n' \
        > "$test_root/audit/README.md"

    check_audit "$test_root/manifests" "$test_root/audit" yes \
        > "$test_root/out" 2>&1 \
        || { cat "$test_root/out" >&2; die 'self-test rejected a complete audit'; }
    grep -F 'PASS: 3 audit records' "$test_root/out" >/dev/null \
        || die 'self-test did not report the covered manifest count'

    expect_failure() {
        message=$1
        needle=$2
        if check_audit "$test_root/manifests" "$test_root/audit" "$3" \
            > "$test_root/out" 2>&1; then
            die "self-test accepted $message"
        fi
        grep -F "$needle" "$test_root/out" >/dev/null \
            || { cat "$test_root/out" >&2; die "self-test did not explain $message"; }
    }

    # A tracked manifest with no heading must fail.
    printf '# net\n\n' > "$test_root/audit/net.md"
    expect_failure 'a missing heading' 'pkg/net/wifi/iwd.yaml' no
    full_record 'pkg/net/wifi/iwd.yaml' 'pass' >> "$test_root/audit/net.md"

    # A heading for an untracked manifest must fail.
    full_record 'pkg/net/wifi/removed.yaml' 'pass' >> "$test_root/audit/net.md"
    expect_failure 'a stale heading' 'audit heading names no tracked manifest' no
    printf '# net\n\n' > "$test_root/audit/net.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'pass' >> "$test_root/audit/net.md"

    # A repeated heading must fail.
    full_record 'pkg/libs/system/attr.yaml' 'pass' >> "$test_root/audit/net.md"
    expect_failure 'a duplicate heading' 'duplicate audit heading' no
    printf '# net\n\n' > "$test_root/audit/net.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'pass' >> "$test_root/audit/net.md"

    # A record filed under the wrong area must fail.
    printf '# libs\n\n' > "$test_root/audit/libs.md"
    full_record 'pkg/libs/system/attr.yaml' 'pass' >> "$test_root/audit/libs.md"
    full_record 'pkg/libs/system/glibc.yaml' 'gap fixed' >> "$test_root/audit/libs.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'pass' >> "$test_root/audit/libs.md"
    printf '# net\n\n' > "$test_root/audit/net.md"
    expect_failure 'a record in the wrong area file' 'belongs in net.md' no
    printf '# libs\n\n' > "$test_root/audit/libs.md"
    full_record 'pkg/libs/system/attr.yaml' 'pass' >> "$test_root/audit/libs.md"
    full_record 'pkg/libs/system/glibc.yaml' 'gap fixed' >> "$test_root/audit/libs.md"
    printf '# net\n\n' > "$test_root/audit/net.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'pass' >> "$test_root/audit/net.md"

    # An unknown result state must fail even outside the final check.
    printf '# net\n\n' > "$test_root/audit/net.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'probably fine' >> "$test_root/audit/net.md"
    expect_failure 'an unknown result state' 'unknown result "probably fine"' no

    # A heading with no Result line must fail.
    printf '# net\n\n### `pkg/net/wifi/iwd.yaml`\n\nPurpose: example\n' \
        > "$test_root/audit/net.md"
    expect_failure 'a record with no result' 'has no Result line' no

    # An open result must pass the coverage check and fail the final check.
    printf '# net\n\n' > "$test_root/audit/net.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'uncertain' >> "$test_root/audit/net.md"
    check_audit "$test_root/manifests" "$test_root/audit" no \
        > "$test_root/out" 2>&1 \
        || { cat "$test_root/out" >&2; die 'self-test rejected an open result'; }
    grep -F 'uncertain: 1' "$test_root/out" >/dev/null \
        || die 'self-test did not tally the open result'
    expect_failure 'an open result in the final check' 'is not finished' yes

    printf '# net\n\n' > "$test_root/audit/net.md"
    full_record 'pkg/net/wifi/iwd.yaml' 'gap' >> "$test_root/audit/net.md"
    expect_failure 'a gap in the final check' 'is not finished' yes

    # A finished record missing required fields must fail the final check.
    printf '# net\n\n' > "$test_root/audit/net.md"
    write_record 'pkg/net/wifi/iwd.yaml' \
        'Purpose: example' \
        'Result: pass' \
        >> "$test_root/audit/net.md"
    expect_failure 'a record missing required fields' 'carries 2 of 7 required fields' yes

    printf 'PASS: package configuration audit checker self-test\n'
}

final=no
self=no
manifest_list=
audit_dir=$AUDIT_DIR_DEFAULT

while [ "$#" -gt 0 ]; do
    case $1 in
        --self-test) self=yes; shift ;;
        --final) final=yes; shift ;;
        -*) die "unknown option: $1" ;;
        *) break ;;
    esac
done

if [ "$self" = yes ]; then
    [ "$#" -eq 0 ] || die 'usage: check-package-config-audit.sh --self-test'
    self_test
    exit 0
fi

case $# in
    0) ;;
    2) manifest_list=$1; audit_dir=$2 ;;
    *) die 'usage: check-package-config-audit.sh [--final] [MANIFEST_LIST AUDIT_DIR]' ;;
esac

if [ -z "$manifest_list" ]; then
    manifest_list=$(mktemp)
    trap 'rm -f "$manifest_list"' EXIT HUP INT TERM
    git ls-files 'pkg/**/*.yaml' | LC_ALL=C sort > "$manifest_list"
fi

check_audit "$manifest_list" "$audit_dir" "$final"
