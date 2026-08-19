#!/bin/sh
# Reject product and test policy from Nex's reusable assemblies.
set -eu

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

check_pattern() {
    file=$1
    label=$2
    pattern=$3

    if matches=$(LC_ALL=C grep -n -E "$pattern" "$file"); then
        printf '%s: %s\n' "$file" "$label" >&2
        printf '%s\n' "$matches" >&2
        return 1
    fi
}

scan_assembly() {
    file=$1
    failed=0

    [ -f "$file" ] || die "assembly not found: $file"

    check_pattern "$file" 'built-in test identity or home' \
        'testuser|/home/[[:alnum:]_.-]+' || failed=1
    check_pattern "$file" 'site-specific network policy' \
        'wegelnet|192\.168\.3\.|psk[[:space:]]*=|address1=|ipv4\.addresses|ipv4\.gateway|ipv4\.dns' || failed=1
    check_pattern "$file" 'SSH authentication bypass' \
        'accept-any-key|AuthorizedKeysCommand' || failed=1
    check_pattern "$file" 'permissive SSH policy' \
        'PermitRootLogin[[:space:]]+yes|PermitEmptyPasswords[[:space:]]+yes|PasswordAuthentication[[:space:]]+yes' || failed=1
    check_pattern "$file" 'enabled remote access' \
        'multi-user\.target\.wants/sshd(-keygen)?\.service' || failed=1
    check_pattern "$file" 'test-only service or helper' \
        'nm-autoconnect|nex-boot-dump' || failed=1
    check_pattern "$file" 'unlocked empty root password' \
        '^[[:space:]]+root::[0-9:]' || failed=1

    if ! LC_ALL=C grep -q -E '^[[:space:]]+root:!\*:' "$file"; then
        printf '%s: root does not have the required locked shadow field\n' "$file" >&2
        failed=1
    fi

    return "$failed"
}

scan_assemblies() {
    failed=0

    for file in "$@"; do
        scan_assembly "$file" || failed=1
    done

    [ "$failed" -eq 0 ] || return 1
    printf 'PASS: reusable assemblies contain only generic access and network policy\n'
}

self_test() {
    test_root=$(mktemp -d)
    trap 'rm -rf "$test_root"' EXIT HUP INT TERM

    cat > "$test_root/safe.yaml" <<'EOF'
system:
  name: safe
  slug: safe
  version: 1.0

files:
- path: /etc/passwd
  content: |
    root:x:0:0:root:/root:/bin/bash
    sshd:x:74:74:SSH:/var/lib/sshd:/usr/bin/nologin
- path: /etc/shadow
  content: |
    root:!*:19735:0:99999:7:::
EOF
    scan_assemblies "$test_root/safe.yaml" > "$test_root/safe.out" ||
        die 'self-test rejected a locked generic assembly'

    cat > "$test_root/unsafe.yaml" <<'EOF'
system:
  name: unsafe
  slug: unsafe
  version: 1.0

files:
  testuser:x:1000:1000:Test:/home/testuser:/bin/bash
  root::19735:0:99999:7:::
  PermitRootLogin yes
  PermitEmptyPasswords yes
  PasswordAuthentication yes
  AuthorizedKeysCommand /usr/local/bin/accept-any-key
  psk=tracked-secret
  address1=192.168.3.110/24,192.168.3.2
  nm-autoconnect nex-boot-dump
  /etc/systemd/system/multi-user.target.wants/sshd.service
EOF
    if scan_assemblies "$test_root/unsafe.yaml" > "$test_root/unsafe.out" 2>&1; then
        die 'self-test accepted unsafe reusable assembly policy'
    fi
    for label in \
        'built-in test identity or home' \
        'site-specific network policy' \
        'SSH authentication bypass' \
        'permissive SSH policy' \
        'enabled remote access' \
        'test-only service or helper' \
        'unlocked empty root password' \
        'root does not have the required locked shadow field'
    do
        grep -F "$label" "$test_root/unsafe.out" >/dev/null ||
            die "self-test did not report: $label"
    done

    printf 'PASS: generic assembly policy checker self-test\n'
}

case "${1:-}" in
    --self-test)
        [ "$#" -eq 1 ] || die 'usage: check-generic-assembly-policy.sh [--self-test | ASSEMBLY...]'
        self_test
        ;;
    '')
        scan_assemblies \
            base/nex-systemd.yaml \
            examples/desktop-vwl/desktop-vwl.yaml
        ;;
    *)
        scan_assemblies "$@"
        ;;
esac
