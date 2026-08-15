#!/usr/bin/env bash
# Exercise Tig and Wget's layered readers in a finished desktop root.
set -euo pipefail

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

need_tool() {
    command -v "$1" >/dev/null 2>&1 || die "$1 not found"
}

if [[ "${DESKTOP_COMMAND_CONFIG_TEST_IN_NAMESPACE:-0}" != 1 ]]; then
    [[ $# -eq 1 ]] || die "usage: $0 ROOTFS"
    need_tool realpath
    need_tool unshare

    rootfs_under_test="$(realpath "$1")"
    [[ -d "$rootfs_under_test" ]] ||
        die "rootfs is not a directory: $rootfs_under_test"
    [[ -x "$rootfs_under_test/usr/bin/bash" ||
        -L "$rootfs_under_test/usr/bin/bash" ]] ||
        die "rootfs has no /usr/bin/bash"

    export DESKTOP_COMMAND_CONFIG_TEST_IN_NAMESPACE=1
    export DESKTOP_COMMAND_CONFIG_TEST_ROOT="$rootfs_under_test"
    export LC_ALL=C
    exec unshare --user --map-root-user --mount --pid --fork \
        "$(realpath "$0")"
fi

need_tool chroot

chroot "$DESKTOP_COMMAND_CONFIG_TEST_ROOT" /usr/bin/bash -s <<'ROOTFS_TEST'
set -euo pipefail

export HOME=/tmp/nex-command-config-test-home
export LC_ALL=C
export TERM=xterm

cleanup() {
    rm -f /etc/tigrc /run/tigrc /etc/wgetrc /run/wgetrc
    rm -rf "$HOME"
}
trap cleanup EXIT HUP INT TERM

for path in /etc/tigrc /run/tigrc /etc/wgetrc /run/wgetrc; do
    [[ ! -e "$path" ]] || {
        printf 'test path already exists: %s\n' "$path" >&2
        exit 1
    }
done
mkdir -p "$HOME" /run

[[ -s /usr/lib/tigrc ]] || {
    printf 'desktop omitted Tig vendor configuration\n' >&2
    exit 1
}
[[ -s /usr/lib/wgetrc ]] || {
    printf 'desktop omitted Wget vendor configuration\n' >&2
    exit 1
}
env -u SYSTEM_WGETRC -u WGETRC wget --version >/dev/null

capture_tig() {
    output="$(printf '\n' | env -u TIGRC_SYSTEM \
        TIGRC_USER= TIG_NO_DISPLAY=1 tig 2>&1 || true)"
}

assert_mentions_only() {
    expected=$1
    shift
    printf '%s\n' "$output" | grep -Fq "$expected" || {
        printf '%s\n' "$output" >&2
        printf 'reader did not report %s\n' "$expected" >&2
        exit 1
    }
    for rejected in "$@"; do
        if printf '%s\n' "$output" | grep -Fq "$rejected"; then
            printf '%s\n' "$output" >&2
            printf 'reader also reported lower tier %s\n' "$rejected" >&2
            exit 1
        fi
    done
}

printf '%s\n' 'nex-invalid-transient-option' > /run/tigrc
capture_tig
assert_mentions_only /run/tigrc /usr/lib/tigrc

printf '%s\n' 'nex-invalid-administrator-option' > /etc/tigrc
capture_tig
assert_mentions_only /etc/tigrc /run/tigrc /usr/lib/tigrc

: > /etc/tigrc
capture_tig
if printf '%s\n' "$output" | grep -Eq '/run/tigrc|/usr/lib/tigrc'; then
    printf '%s\n' "$output" >&2
    printf 'empty Tig administrator file did not mask lower tiers\n' >&2
    exit 1
fi
rm -f /etc/tigrc /run/tigrc

capture_wget() {
    set +e
    output="$(env -u SYSTEM_WGETRC -u WGETRC wget --version 2>&1)"
    status=$?
    set -e
}

printf '%s\n' 'nex-invalid-transient-option = on' > /run/wgetrc
capture_wget
[[ $status -ne 0 ]] || exit 1
assert_mentions_only /run/wgetrc /usr/lib/wgetrc

printf '%s\n' 'nex-invalid-administrator-option = on' > /etc/wgetrc
capture_wget
[[ $status -ne 0 ]] || exit 1
assert_mentions_only /etc/wgetrc /run/wgetrc /usr/lib/wgetrc

: > /etc/wgetrc
capture_wget
[[ $status -eq 0 ]] || {
    printf '%s\n' "$output" >&2
    printf 'empty Wget administrator file did not mask lower tiers\n' >&2
    exit 1
}

printf 'PASS: finished desktop Tig and Wget layered readers\n'
ROOTFS_TEST
