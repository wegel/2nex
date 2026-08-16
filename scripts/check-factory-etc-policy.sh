#!/usr/bin/env bash
# Inventory factory /etc and reject files that do not have a machine owner.
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
policy_file=${FACTORY_ETC_POLICY:-$script_dir/factory-etc-policy.tsv}

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

file_kind() {
    local path=$1

    if [[ -L "$path" ]]; then
        printf 'l\n'
    elif [[ -f "$path" ]]; then
        printf 'f\n'
    else
        printf 'unsupported\n'
    fi
}

find_policy() {
    local relative=$1
    local kind=$2
    local link_target=$3
    local pattern
    local types

    policy_class=
    policy_owner=
    policy_consumer=
    policy_source=
    policy_target=
    policy_reason=

    while IFS=$'\t' read -r pattern types policy_class policy_owner \
        policy_consumer policy_source policy_target policy_reason; do
        [[ "$pattern" != pattern ]] || continue
        case "$relative" in
            $pattern)
                [[ "$types" == *"$kind"* ]] || return 2
                if [[ "$kind" == l && "$policy_target" != '*' && \
                    "$link_target" != "$policy_target" ]]; then
                    return 3
                fi
                return 0
                ;;
        esac
    done < "$policy_file"

    return 1
}

check_link() {
    local root=$1
    local factory=$2
    local path=$3
    local relative=$4
    local target=$5
    local resolved

    if [[ "$target" =~ ^/nex/pkg/.*/[0-9a-f]{8,}/ ]]; then
        printf 'error: factory link pins a package checksum: %s -> %s\n' \
            "$relative" "$target" >&2
        return 1
    fi

    case "$target" in
        /usr/*)
            if [[ ! -e "$root$target" && ! -L "$root$target" ]]; then
                printf 'error: factory link has no immutable target: %s -> %s\n' \
                    "$relative" "$target" >&2
                return 1
            fi
            ;;
        /*)
            ;;
        *)
            resolved=$(realpath -m "$(dirname "$path")/$target")
            case "$resolved" in
                "$factory"/*) ;;
                *)
                    printf 'error: relative factory link escapes its tree: %s -> %s\n' \
                        "$relative" "$target" >&2
                    return 1
                    ;;
            esac
            if [[ ! -e "$resolved" && ! -L "$resolved" ]]; then
                printf 'error: relative factory link is dangling: %s -> %s\n' \
                    "$relative" "$target" >&2
                return 1
            fi
            ;;
    esac
}

scan_root() {
    local root=$1
    local factory
    local path
    local relative
    local kind
    local kind_name
    local link_target
    local bytes
    local policy_status
    local failed=0

    root=$(realpath "$root")
    factory=$root/usr/share/factory/etc
    [[ -d "$factory" ]] || {
        printf 'error: factory /etc is missing: %s\n' "$factory" >&2
        return 1
    }
    [[ -r "$policy_file" ]] || {
        printf 'error: policy file is missing: %s\n' "$policy_file" >&2
        return 1
    }

    printf 'path\ttype\ttarget\tbytes\tclass\towner\tconsumer\tsource\treason\n'
    while IFS= read -r -d '' path; do
        relative=${path#"$factory"/}
        kind=$(file_kind "$path")
        link_target=-
        [[ "$kind" != l ]] || link_target=$(readlink "$path")

        if [[ "$kind" == unsupported ]]; then
            printf 'error: factory path has an unsupported type: %s\n' \
                "$relative" >&2
            failed=1
            continue
        fi

        if [[ "$kind" == l ]] && \
            ! check_link "$root" "$factory" "$path" "$relative" "$link_target"; then
            failed=1
            continue
        fi

        if find_policy "$relative" "$kind" "$link_target"; then
            :
        else
            policy_status=$?
            case "$policy_status" in
                2)
                    printf 'error: factory path has a forbidden type: %s\n' \
                        "$relative" >&2
                    ;;
                3)
                    printf 'error: factory adapter has the wrong target: %s -> %s\n' \
                        "$relative" "$link_target" >&2
                    ;;
                *)
                    printf 'error: unclassified factory path: %s\n' \
                        "$relative" >&2
                    ;;
            esac
            failed=1
            continue
        fi

        [[ "$kind" == f ]] && kind_name=file || kind_name=symlink
        bytes=$(stat -c '%s' "$path")
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$relative" "$kind_name" "$link_target" "$bytes" \
            "$policy_class" "$policy_owner" "$policy_consumer" \
            "$policy_source" "$policy_reason"
    done < <(find "$factory" -mindepth 1 ! -type d -print0 | LC_ALL=C sort -z)

    [[ "$failed" -eq 0 ]]
}

expect_failure() {
    local root=$1
    local message=$2
    local output=$3

    if scan_root "$root" >"$output" 2>&1; then
        die "self-test accepted $message"
    fi
}

self_test() {
    local test_root
    local root
    local factory

    test_root=$(mktemp -d)
    trap "rm -rf '$test_root'" EXIT HUP INT TERM
    root=$test_root/root
    factory=$root/usr/share/factory/etc

    mkdir -p \
        "$factory/systemd/system/multi-user.target.wants" \
        "$root/usr/lib/systemd/system"
    printf 'root:x:0:0:root:/root:/bin/sh\n' > "$factory/passwd"
    : > "$root/usr/lib/systemd/system/systemd-networkd.service"
    ln -s /usr/lib/systemd/system/systemd-networkd.service \
        "$factory/systemd/system/multi-user.target.wants/systemd-networkd.service"
    scan_root "$root" > "$test_root/positive.tsv" ||
        die 'self-test rejected machine state and documented unit enablement'

    printf 'NAME=stale\n' > "$factory/os-release"
    expect_failure "$root" 'a copied vendor default' "$test_root/vendor-default.out"
    grep -q 'unclassified factory path: os-release' "$test_root/vendor-default.out" ||
        die 'self-test did not report the copied vendor default'
    rm "$factory/os-release"

    ln -s /usr/lib/unknown.conf "$factory/unknown.conf"
    expect_failure "$root" 'an undocumented compatibility link' "$test_root/adapter.out"
    grep -Eq 'unclassified factory path|no immutable target' "$test_root/adapter.out" ||
        die 'self-test did not report the undocumented compatibility link'
    rm "$factory/unknown.conf"

    rm "$factory/systemd/system/multi-user.target.wants/systemd-networkd.service"
    ln -s /nex/pkg/core/init/systemd/257.5/01234567/usr/lib/systemd/system/systemd-networkd.service \
        "$factory/systemd/system/multi-user.target.wants/systemd-networkd.service"
    expect_failure "$root" 'a checksum-pinned package link' "$test_root/checksum.out"
    grep -q 'factory link pins a package checksum' "$test_root/checksum.out" ||
        die 'self-test did not report the checksum-pinned package link'

    printf 'PASS: factory /etc policy self-test\n'
}

usage() {
    printf 'usage: %s --self-test | ROOT [ROOT ...]\n' "$0" >&2
    exit 2
}

[[ "$#" -gt 0 ]] || usage
if [[ "$1" == --self-test ]]; then
    [[ "$#" -eq 1 ]] || usage
    self_test
    exit 0
fi

status=0
for root in "$@"; do
    scan_root "$root" || status=1
done
exit "$status"
