#!/usr/bin/env bash
# Exercise the assembled Edgebox rootfs without changing the host mounts.
set -euo pipefail

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

need_tool() {
    command -v "$1" >/dev/null 2>&1 || die "$1 not found"
}

if [[ "${EDGEBOX_ROOTFS_TEST_IN_NAMESPACE:-0}" != 1 ]]; then
    [[ $# -eq 1 ]] || die "usage: $0 ROOTFS"
    need_tool realpath
    need_tool unshare

    rootfs_under_test="$(realpath "$1")"
    [[ -d "$rootfs_under_test" ]] || die "rootfs is not a directory: $rootfs_under_test"
    [[ -x "$rootfs_under_test/usr/bin/bash" ]] || die "rootfs has no executable /usr/bin/bash"

    export EDGEBOX_ROOTFS_TEST_IN_NAMESPACE=1
    export EDGEBOX_ROOTFS_TEST_ROOT="$rootfs_under_test"
    export LC_ALL=C
    exec unshare --user --map-root-user --mount --pid --fork "$(realpath "$0")"
fi

need_tool chroot
need_tool mount
mount --make-rprivate /
mount --rbind /dev "$EDGEBOX_ROOTFS_TEST_ROOT/dev"
mount -t proc proc "$EDGEBOX_ROOTFS_TEST_ROOT/proc"

chroot "$EDGEBOX_ROOTFS_TEST_ROOT" /usr/bin/bash -s <<'ROOTFS_TEST'
set -euo pipefail

export HOME=/root
export LANG=en_GB.UTF-8
export LC_ALL=en_GB.UTF-8
export PATH=/usr/bin:/usr/sbin:/bin:/sbin

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

assert_contains() {
    local label="$1"
    local expected="$2"
    shift 2
    local output
    if ! output=$("$@" 2>&1); then
        printf '%s\n' "$output" >&2
        fail "$label command failed"
    fi
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output" >&2
        fail "$label output did not contain: $expected"
    fi
    printf 'PASS: %s\n' "$label"
}

assert_contains acpid 2.0.34 acpid --version
assert_contains alsa-utils 1.2.14 aplay --version
assert_contains bridge-utils 1.7 brctl --version
assert_contains containerd 2.2.2 containerd --version
assert_contains runc 1.2.4 runc --version
assert_contains cryptsetup 2.8.6 cryptsetup --version
assert_contains curl 8.17.0 curl --version
assert_contains docker 29.3.0 docker --version
assert_contains docker-compose 5.1.0 docker compose version
assert_contains file 5.45 file --version
assert_contains i2c-tools 4.4 i2cdetect -V
assert_contains iproute2 6.18.0 ip -Version
assert_contains iproute2-ss 6.18.0 ss -V
assert_contains jq 1.8.2 jq --version
assert_contains nftables 1.1.1 nft --version
assert_contains ostree 2026.1 ostree --version
assert_contains pipewire 1.4.9 pipewire --version
assert_contains pipewire-cli 1.4.9 pw-cli --version
assert_contains procps-ng 4.0.5 ps --version
assert_contains openssh OpenSSH_9.9 ssh -V
assert_contains smartmontools 7.5 smartctl --version
assert_contains usbutils 019 lsusb --version
assert_contains dmidecode 3.7 dmidecode --version
assert_contains e2fsprogs 1.47.0 e2fsck -V
assert_contains kbd 2.9.0 loadkeys -V
assert_contains vim 9.2 vim --version
assert_contains wireplumber 0.5.12 wireplumber --version
assert_contains wireplumber-cli Usage wpctl --help
assert_contains systemd-analyze 257.5 systemd-analyze --version
assert_contains python 3.12.2 python3 --version
assert_contains lsusb-python Usage /usr/bin/lsusb.py --help

[[ "$(locale charmap)" == UTF-8 ]] || fail "locale charmap is not UTF-8"
locale -a | grep -Fx en_GB.UTF-8 >/dev/null || fail "en_GB.UTF-8 is not generated"
printf 'PASS: en_GB locale\n'

[[ "$(readlink /etc/localtime)" == /usr/share/zoneinfo/Universal ]] ||
    fail "/etc/localtime does not select Universal"
[[ "$(readlink /etc/resolv.conf)" == /run/systemd/resolve/stub-resolv.conf ]] ||
    fail "/etc/resolv.conf does not use systemd-resolved"
grep -Fx 'DHCP=yes' /etc/systemd/network/80-dhcp.network >/dev/null ||
    fail "networkd DHCP policy is missing"
printf 'PASS: locale, clock, and network policy\n'

for enabled_unit in \
    acpid.service \
    containerd.service \
    docker.service \
    pipewire-system.service \
    pipewire-pulse-system.service \
    sshd.service \
    systemd-networkd.service \
    systemd-resolved.service \
    wireplumber-system.service; do
    enabled_path="/etc/systemd/system/multi-user.target.wants/$enabled_unit"
    [[ -L "$enabled_path" ]] || fail "$enabled_unit is not enabled"
done
[[ -L /etc/systemd/system/sockets.target.wants/dbus.socket ]] || fail "dbus.socket is not enabled"
[[ -L /etc/systemd/system/sockets.target.wants/docker.socket ]] || fail "docker.socket is not enabled"
[[ -L /etc/systemd/system/sysinit.target.wants/systemd-timesyncd.service ]] ||
    fail "systemd-timesyncd.service is not enabled"
printf 'PASS: service links\n'

systemd-analyze verify \
    /usr/lib/systemd/system/pipewire-system.service \
    /usr/lib/systemd/system/wireplumber-system.service \
    /usr/lib/systemd/system/pipewire-pulse-system.service
printf 'PASS: appliance audio units\n'

module_firmware=$(modinfo -k 6.18.24 -F firmware r8169)
[[ "$module_firmware" == *"rtl_nic/rtl8168h-2.fw"* ]] ||
    fail "r8169 does not name the expected Realtek firmware"
modinfo -k 6.18.24 amdgpu >/dev/null || fail "amdgpu module metadata cannot be read"
printf 'PASS: kernel modules and firmware metadata\n'

ostree_test_dir=$(mktemp -d /tmp/ostree-rootfs-test.XXXXXX)
trap 'rm -rf "$ostree_test_dir"' EXIT
mkdir -p "$ostree_test_dir/tree"
printf 'rootfs smoke\n' > "$ostree_test_dir/tree/probe"
ostree init --repo="$ostree_test_dir/repo" --mode=bare-user-only
ostree commit \
    --repo="$ostree_test_dir/repo" \
    --branch=smoke \
    --tree="dir=$ostree_test_dir/tree" \
    --subject='rootfs smoke' >/dev/null
ostree fsck --repo="$ostree_test_dir/repo" >/dev/null
rm -rf "$ostree_test_dir"
trap - EXIT
printf 'PASS: OSTree repository commit and fsck\n'

printf 'PASS: Edgebox rootfs smoke test\n'
ROOTFS_TEST
