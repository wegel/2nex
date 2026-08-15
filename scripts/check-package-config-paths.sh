#!/bin/sh
# Reject package outputs that place vendor files below /etc or /usr/etc.
set -eu

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

scan_manifests() {
    manifest_root=$1

    [ -d "$manifest_root" ] || die "manifest directory not found: $manifest_root"

    if matches=$(LC_ALL=C grep -R -n -E --include='*.yaml' \
        '^[[:space:]]*-[[:space:]]+path:[[:space:]]+(/etc|/usr/etc)(/[^[:space:]#]+)?([[:space:]]+#.*)?[[:space:]]*$' \
        "$manifest_root"); then
        printf '%s\n' "$matches" >&2
        printf 'error: package manifests must not declare outputs below /etc or /usr/etc\n' >&2
        return 1
    fi

    printf 'PASS: package manifests declare no outputs below /etc or /usr/etc\n'
}

self_test() {
    test_root=$(mktemp -d)
    trap 'rm -rf "$test_root"' EXIT HUP INT TERM
    mkdir -p "$test_root/pkg"

    printf '%s\n' \
        'build:' \
        '  script: echo /etc/example.conf' \
        'outputs:' \
        '  conf:' \
        '    files:' \
        '    - path: /usr/lib/example.conf' \
        > "$test_root/pkg/allowed.yaml"
    printf '%s\n' \
        'outputs:' \
        '  conf:' \
        '    files:' \
        '    - path: /etc/example.conf' \
        '    - path: /usr/etc/example.conf' \
        > "$test_root/pkg/rejected.yaml"

    if scan_manifests "$test_root/pkg" > "$test_root/output" 2>&1; then
        die 'self-test accepted forbidden package paths'
    fi
    grep -F "$test_root/pkg/rejected.yaml:4:" "$test_root/output" >/dev/null \
        || die 'self-test did not report /etc output'
    grep -F "$test_root/pkg/rejected.yaml:5:" "$test_root/output" >/dev/null \
        || die 'self-test did not report /usr/etc output'
    grep -F 'package manifests must not declare outputs' "$test_root/output" >/dev/null \
        || die 'self-test did not explain the failure'

    rm "$test_root/pkg/rejected.yaml"
    scan_manifests "$test_root/pkg" > "$test_root/output"
    grep -F 'PASS: package manifests declare no outputs' "$test_root/output" >/dev/null \
        || die 'self-test did not accept vendor paths below /usr/lib'

    printf 'PASS: package configuration path checker self-test\n'
}

case "${1:-}" in
    --self-test)
        [ "$#" -eq 1 ] || die 'usage: check-package-config-paths.sh [--self-test | MANIFEST_DIR]'
        self_test
        ;;
    '')
        scan_manifests pkg
        ;;
    *)
        [ "$#" -eq 1 ] || die 'usage: check-package-config-paths.sh [--self-test | MANIFEST_DIR]'
        scan_manifests "$1"
        ;;
esac
