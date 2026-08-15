#!/usr/bin/env bash
# Exercise Libvirt's OpenSSH and Logrotate fragments in a finished desktop root.
set -euo pipefail

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

need_tool() {
    command -v "$1" >/dev/null 2>&1 || die "$1 not found"
}

if [[ "${DESKTOP_LIBVIRT_TEST_IN_NAMESPACE:-0}" != 1 ]]; then
    [[ $# -eq 1 ]] || die "usage: $0 ROOTFS"
    need_tool realpath
    need_tool unshare

    rootfs_under_test="$(realpath "$1")"
    [[ -d "$rootfs_under_test" ]] || die "rootfs is not a directory: $rootfs_under_test"
    [[ -x "$rootfs_under_test/usr/bin/bash" || -L "$rootfs_under_test/usr/bin/bash" ]] ||
        die "rootfs has no /usr/bin/bash"

    export DESKTOP_LIBVIRT_TEST_IN_NAMESPACE=1
    export DESKTOP_LIBVIRT_TEST_ROOT="$rootfs_under_test"
    export LC_ALL=C
    exec unshare --user --map-root-user --mount --pid --fork "$(realpath "$0")"
fi

need_tool chroot

chroot "$DESKTOP_LIBVIRT_TEST_ROOT" /usr/bin/bash -s <<'ROOTFS_TEST'
set -euo pipefail

export HOME=/tmp/libvirt-integration-home
export LC_ALL=C
export PATH=/usr/bin:/usr/sbin:/bin:/sbin

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

ssh_fragment=/usr/lib/ssh/ssh_config.d/30-libvirt-ssh-proxy.conf
logrotate_fragment=/usr/lib/logrotate.d/libvirtd

[[ -s "$ssh_fragment" ]] || fail "Libvirt OpenSSH fragment is missing"
[[ -s "$logrotate_fragment" ]] || fail "Libvirt Logrotate fragment is missing"
[[ -f /usr/lib/logrotate.conf ]] || fail "Logrotate vendor main file is missing"
[[ -x /usr/bin/logrotate ]] || fail "Logrotate command is missing"

cleanup() {
    rm -rf "$HOME" /tmp/libvirt-integration-test
    rm -f \
        /etc/ssh/ssh_config.d/30-libvirt-ssh-proxy.conf \
        /run/ssh/ssh_config.d/30-libvirt-ssh-proxy.conf \
        /etc/logrotate.d/libvirtd \
        /run/logrotate.d/libvirtd \
        /var/log/libvirt/libvirtd.log \
        /var/log/libvirt/libvirtd.log.1
}
trap cleanup EXIT
cleanup
mkdir -p \
    "$HOME" \
    /tmp/libvirt-integration-test \
    /etc/ssh/ssh_config.d \
    /run/ssh/ssh_config.d \
    /etc/logrotate.d \
    /run/logrotate.d \
    /var/log/libvirt

ssh_config() {
    /usr/bin/ssh -G qemu/system 2>/dev/null
}

ssh_config | grep -qxF \
    'proxycommand /usr/libexec/libvirt-ssh-proxy %h %p' ||
    fail "OpenSSH did not read Libvirt's vendor proxy"

cat > /run/ssh/ssh_config.d/30-libvirt-ssh-proxy.conf <<'SSH_CONFIG'
Host qemu/*
    ProxyCommand /usr/bin/true transient
SSH_CONFIG
ssh_config | grep -qxF 'proxycommand /usr/bin/true transient' ||
    fail "OpenSSH did not select the transient same-name fragment"
if ssh_config | grep -qF '/usr/libexec/libvirt-ssh-proxy'; then
    fail "OpenSSH also read the replaced vendor fragment"
fi

cat > /etc/ssh/ssh_config.d/30-libvirt-ssh-proxy.conf <<'SSH_CONFIG'
Host qemu/*
    ProxyCommand /usr/bin/true administrator
SSH_CONFIG
ssh_config | grep -qxF 'proxycommand /usr/bin/true administrator' ||
    fail "OpenSSH did not select the administrator same-name fragment"
if ssh_config | grep -qF '/usr/bin/true transient'; then
    fail "OpenSSH also read the replaced transient fragment"
fi

write_logrotate_rule() {
    local rule_path=$1
    local log_path=$2
    cat > "$rule_path" <<RULE
$log_path {
    size 1
    rotate 1
    copytruncate
}
RULE
}

make_vendor_log() {
    local block
    local index

    printf -v block '%1024s' ''
    : > /var/log/libvirt/libvirtd.log
    for ((index = 0; index < 128; index++)); do
        printf '%s' "$block" >> /var/log/libvirt/libvirtd.log
    done
}

make_vendor_log
/usr/bin/logrotate --force \
    --state /tmp/libvirt-integration-test/vendor.state
[[ -f /var/log/libvirt/libvirtd.log.1 ]] ||
    fail "Logrotate did not apply Libvirt's vendor rule"
[[ ! -s /var/log/libvirt/libvirtd.log ]] ||
    fail "Libvirt's copytruncate rule did not empty the live log"

transient_log=/tmp/libvirt-integration-test/transient.log
write_logrotate_rule /run/logrotate.d/libvirtd "$transient_log"
make_vendor_log
printf '%s\n' transient > "$transient_log"
/usr/bin/logrotate --force \
    --state /tmp/libvirt-integration-test/transient.state
[[ -f "${transient_log}.1" ]] ||
    fail "Logrotate did not select the transient same-name fragment"
[[ ! -s "$transient_log" ]] ||
    fail "the transient rule did not rotate its log"
[[ -s /var/log/libvirt/libvirtd.log ]] ||
    fail "Logrotate also applied the replaced vendor rule"

: > /etc/logrotate.d/libvirtd
make_vendor_log
/usr/bin/logrotate --force \
    --state /tmp/libvirt-integration-test/mask.state
[[ -s /var/log/libvirt/libvirtd.log ]] ||
    fail "the empty administrator fragment did not mask the vendor rule"

/usr/bin/systemd-analyze verify \
    /usr/lib/systemd/system/logrotate.service \
    /usr/lib/systemd/system/logrotate.timer
[[ "$(readlink /usr/share/factory/etc/systemd/system/timers.target.wants/logrotate.timer)" == /usr/lib/systemd/system/logrotate.timer ]] ||
    fail "the desktop did not enable logrotate.timer"

printf 'PASS: Libvirt OpenSSH and Logrotate integrations\n'
ROOTFS_TEST
