#!/usr/bin/env bash
# Inventory flat-root /etc leaves and reject paths without a machine owner.
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
policy_file=${FLAT_ETC_POLICY:-$script_dir/flat-etc-policy.tsv}

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

scope_matches() {
    local scopes=$1
    local scope=$2

    case ",$scopes," in
        *,"$scope",*|*,\**,*) return 0 ;;
        *) return 1 ;;
    esac
}

find_policy() {
    local scope=$1
    local relative=$2
    local kind=$3
    local link_target=$4
    local scopes
    local pattern
    local types

    policy_class=
    policy_owner=
    policy_consumer=
    policy_source=
    policy_target=
    policy_reason=

    while IFS=$'\t' read -r scopes pattern types policy_class policy_owner \
        policy_consumer policy_source policy_target policy_reason; do
        [[ "$scopes" != scope ]] || continue
        scope_matches "$scopes" "$scope" || continue
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
    local etc=$2
    local path=$3
    local relative=$4
    local target=$5
    local resolved

    if [[ "$target" =~ ^/nex/pkg/.*/[0-9a-f]{8,}/ ]]; then
        printf 'error: flat /etc link pins a package checksum: %s -> %s\n' \
            "$relative" "$target" >&2
        return 1
    fi

    case "$target" in
        /usr/*)
            if [[ ! -e "$root$target" && ! -L "$root$target" ]]; then
                printf 'error: flat /etc link has no immutable target: %s -> %s\n' \
                    "$relative" "$target" >&2
                return 1
            fi
            ;;
        /*)
            ;;
        *)
            resolved=$(realpath -m "$(dirname "$path")/$target")
            case "$resolved" in
                "$etc"/*) ;;
                *)
                    printf 'error: relative flat /etc link escapes its tree: %s -> %s\n' \
                        "$relative" "$target" >&2
                    return 1
                    ;;
            esac
            if [[ ! -e "$resolved" && ! -L "$resolved" ]]; then
                printf 'error: relative flat /etc link is dangling: %s -> %s\n' \
                    "$relative" "$target" >&2
                return 1
            fi
            ;;
    esac
}

scan_root() {
    local scope=$1
    local root=$2
    local etc
    local path
    local relative
    local kind
    local kind_name
    local link_target
    local bytes
    local policy_status
    local failed=0

    root=$(realpath "$root")
    etc=$root/etc
    [[ -d "$etc" ]] || {
        printf 'error: flat /etc is missing: %s\n' "$etc" >&2
        return 1
    }
    [[ -r "$policy_file" ]] || {
        printf 'error: policy file is missing: %s\n' "$policy_file" >&2
        return 1
    }

    printf 'scope\tpath\ttype\ttarget\tbytes\tclass\towner\tconsumer\tsource\treason\n'
    while IFS= read -r -d '' path; do
        relative=${path#"$etc"/}
        kind=$(file_kind "$path")
        link_target=-
        [[ "$kind" != l ]] || link_target=$(readlink "$path")

        if [[ "$kind" == unsupported ]]; then
            printf 'error: flat /etc path has an unsupported type: %s:%s\n' \
                "$scope" "$relative" >&2
            failed=1
            continue
        fi

        if [[ "$kind" == l ]] && \
            ! check_link "$root" "$etc" "$path" "$relative" "$link_target"; then
            failed=1
            continue
        fi

        if find_policy "$scope" "$relative" "$kind" "$link_target"; then
            :
        else
            policy_status=$?
            case "$policy_status" in
                2)
                    printf 'error: flat /etc path has a forbidden type: %s:%s\n' \
                        "$scope" "$relative" >&2
                    ;;
                3)
                    printf 'error: flat /etc adapter has the wrong target: %s:%s -> %s\n' \
                        "$scope" "$relative" "$link_target" >&2
                    ;;
                *)
                    printf 'error: unclassified flat /etc path: %s:%s\n' \
                        "$scope" "$relative" >&2
                    ;;
            esac
            failed=1
            continue
        fi

        [[ "$kind" == f ]] && kind_name=file || kind_name=symlink
        bytes=$(stat -c '%s' "$path")
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$scope" "$relative" "$kind_name" "$link_target" "$bytes" \
            "$policy_class" "$policy_owner" "$policy_consumer" \
            "$policy_source" "$policy_reason"
    done < <(find "$etc" -mindepth 1 ! -type d -print0 | LC_ALL=C sort -z)

    [[ "$failed" -eq 0 ]]
}

expect_failure() {
    local scope=$1
    local root=$2
    local message=$3
    local output=$4

    if scan_root "$scope" "$root" >"$output" 2>&1; then
        die "self-test accepted $message"
    fi
}

self_test() {
    local test_root
    local root

    test_root=$(mktemp -d)
    trap "rm -rf '$test_root'" EXIT HUP INT TERM
    root=$test_root/root

    mkdir -p "$root/etc"
    printf 'root:x:0:0:root:/root:/bin/sh\n' > "$root/etc/passwd"
    ln -s /proc/self/mounts "$root/etc/mtab"
    scan_root edgebox-rootfs "$root" > "$test_root/positive.tsv" ||
        die 'self-test rejected machine state and a documented adapter'

    printf 'NAME=stale\n' > "$root/etc/os-release"
    expect_failure edgebox-rootfs "$root" 'a copied vendor default' \
        "$test_root/vendor-default.out"
    grep -q 'unclassified flat /etc path: edgebox-rootfs:os-release' \
        "$test_root/vendor-default.out" ||
        die 'self-test did not report the copied vendor default'
    rm "$root/etc/os-release"

    rm "$root/etc/mtab"
    ln -s /tmp/mtab "$root/etc/mtab"
    expect_failure edgebox-rootfs "$root" 'an adapter with the wrong target' \
        "$test_root/adapter.out"
    grep -q 'flat /etc adapter has the wrong target' "$test_root/adapter.out" ||
        die 'self-test did not report the wrong adapter target'

    printf 'PASS: flat /etc policy self-test\n'
}

usage() {
    printf 'usage: %s --self-test | SCOPE ROOT [SCOPE ROOT ...]\n' "$0" >&2
    exit 2
}

[[ "$#" -gt 0 ]] || usage
if [[ "$1" == --self-test ]]; then
    [[ "$#" -eq 1 ]] || usage
    self_test
    exit 0
fi

[[ "$#" -ge 2 && "$(( $# % 2 ))" -eq 0 ]] || usage
status=0
while [[ "$#" -gt 0 ]]; do
    scan_root "$1" "$2" || status=1
    shift 2
done
exit "$status"
