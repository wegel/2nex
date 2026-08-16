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
assert_contains e2scrub 1.47.0 e2scrub -V
assert_contains kbd 2.9.0 loadkeys -V
assert_contains vim 9.2 vim --version
assert_contains wireplumber 0.5.12 wireplumber --version
assert_contains wireplumber-cli Usage wpctl --help
assert_contains systemd-analyze 257.5 systemd-analyze --version
assert_contains python 3.12.2 python3 --version
assert_contains lsusb-python Usage /usr/bin/lsusb.py --help

[[ -f /usr/lib/login.defs ]] || fail "Shadow vendor login.defs is missing"
[[ -f /usr/lib/pam.d/login ]] || fail "Shadow vendor PAM policy is missing"
[[ -f /usr/lib/pam.d/sshd ]] || fail "OpenSSH vendor PAM policy is missing"
[[ ! -e /etc/login.defs ]] || fail "Shadow installed a package default in /etc"
[[ ! -e /etc/pam.d ]] || fail "a package installed PAM service policy in /etc"
printf 'PASS: vendor account and PAM policy\n'

[[ -f /usr/lib/profile ]] || fail "Bash vendor profile is missing"
[[ ! -e /etc/profile ]] || fail "the assembly installed its Bash profile in /etc"
[[ "$(env -u PS1 -u TERM HOME=/root bash --login -c 'printf "%s|%s" "$PS1" "$TERM"')" == '# |dumb' ]] ||
    fail "Bash did not read the vendor profile"
[[ "$(TERM=xterm-256color HOME=/root bash --login -c \
    'printf "%s|%s" "$PS1" "$TERM"')" == '# |xterm-256color' ]] ||
    fail "the vendor profile replaced the caller terminal setting"
[[ "$(
    PS1='custom$ '
    TERM=xterm-256color
    export PS1 TERM
    . /usr/lib/profile
    printf '%s|%s' "$PS1" "$TERM"
)" == 'custom$ |xterm-256color' ]] ||
    fail "the vendor profile replaced caller shell settings"

profile_test_name=zz-profile-layer-test.sh
mkdir -p /usr/lib/profile.d /run/profile.d /etc/profile.d
printf '%s\n' 'export PROFILE_LAYER=vendor' > "/usr/lib/profile.d/$profile_test_name"
printf '%s\n' 'PROFILE_NON_SH_SOURCED=yes' > /usr/lib/profile.d/zz-profile-layer-test.csh
[[ "$(env -u PROFILE_LAYER -u PROFILE_NON_SH_SOURCED HOME=/root bash --login -c \
    'printf "%s|%s" "$PROFILE_LAYER" "${PROFILE_NON_SH_SOURCED-unset}"')" == vendor\|unset ]] ||
    fail "Bash did not load the vendor profile fragment"

printf '%s\n' 'export PROFILE_LAYER=transient' > "/run/profile.d/$profile_test_name"
[[ "$(env -u PROFILE_LAYER HOME=/root bash --login -c 'printf %s "$PROFILE_LAYER"')" == transient ]] ||
    fail "the transient profile fragment did not replace the vendor fragment"

printf '%s\n' 'export PROFILE_LAYER=administrator' > "/etc/profile.d/$profile_test_name"
[[ "$(env -u PROFILE_LAYER HOME=/root bash --login -c 'printf %s "$PROFILE_LAYER"')" == administrator ]] ||
    fail "the administrator profile fragment did not replace lower fragments"

rm -f "/etc/profile.d/$profile_test_name"
ln -s /dev/null "/etc/profile.d/$profile_test_name"
[[ "$(env -u PROFILE_LAYER HOME=/root bash --login -c \
    'printf %s "${PROFILE_LAYER-unset}"')" == unset ]] ||
    fail "the administrator profile mask did not hide lower fragments"
rm -f "/etc/profile.d/$profile_test_name"

cat > /etc/profile << 'PROFILE_TEST'
export PROFILE_MAIN=administrator
PROFILE_TEST
[[ "$(env -u PROFILE_LAYER -u PROFILE_MAIN HOME=/root bash --login -c \
    'printf "%s|%s" "$PROFILE_MAIN" "${PROFILE_LAYER-unset}"')" == administrator\|unset ]] ||
    fail "the administrator main profile did not replace the vendor profile"
rm -f /etc/profile "/run/profile.d/$profile_test_name" \
    "/usr/lib/profile.d/$profile_test_name" /usr/lib/profile.d/zz-profile-layer-test.csh

completion_result=$(HOME=/root PS1='test$ ' bash --login -ic '
    declare -F _comp_complete_load >/dev/null
    complete -p -D | grep -q _comp_complete_load
    printf loaded
' 2>/dev/null)
[[ "$completion_result" == loaded ]] || fail "the Bash Completion vendor hook did not load"
printf 'PASS: layered Bash profile and vendor completion hook\n'

[[ -f /usr/lib/e2scrub.conf ]] || fail "e2scrub vendor policy is missing"
[[ -f /usr/lib/mke2fs.conf ]] || fail "mke2fs vendor policy is missing"
[[ ! -e /etc/e2scrub.conf ]] || fail "e2fsprogs installed e2scrub policy in /etc"
[[ ! -e /etc/mke2fs.conf ]] || fail "e2fsprogs installed mke2fs policy in /etc"
e2fs_image=$(mktemp /tmp/e2fsprogs-rootfs-test.XXXXXX)
truncate -s 32M "$e2fs_image"
mke2fs -q -F -t ext4 "$e2fs_image"
dumpe2fs -h "$e2fs_image" 2>/dev/null |
    grep -Fx 'Filesystem magic number:  0xEF53' >/dev/null ||
    fail "mke2fs did not create an ext4 filesystem"
rm -f "$e2fs_image"
printf 'PASS: e2fsprogs vendor policy and filesystem creation\n'

[[ -f /usr/lib/ssh/ssh_config ]] || fail "OpenSSH vendor client policy is missing"
[[ -f /usr/lib/ssh/sshd_config ]] || fail "OpenSSH vendor server policy is missing"
[[ -f /usr/share/ssh/moduli ]] || fail "OpenSSH moduli database is missing"
[[ ! -e /etc/ssh/ssh_config ]] || fail "OpenSSH installed client policy in /etc"
[[ ! -e /etc/ssh/sshd_config ]] || fail "OpenSSH installed server policy in /etc"
[[ ! -e /etc/ssh/moduli ]] || fail "OpenSSH installed moduli in /etc"
printf 'PASS: OpenSSH vendor policy\n'

[[ -f /usr/lib/nsswitch.conf ]] || fail "Glibc vendor NSS policy is missing"
[[ -f /usr/lib/rpc ]] || fail "Glibc vendor RPC database is missing"
[[ ! -e /etc/nsswitch.conf ]] || fail "the flat assembly copied vendor NSS policy into /etc"
[[ ! -e /etc/rpc ]] || fail "Glibc installed its RPC database in /etc"
getent rpc portmapper | grep -F '100000' >/dev/null ||
    fail "Glibc did not read the vendor RPC database"
printf 'PASS: Glibc vendor databases and RPC lookup\n'

[[ -f /usr/lib/os-release ]] || fail "immutable release identity is missing"
[[ ! -e /etc/os-release ]] || fail "release identity is stored in /etc"
grep -Fq 'PRETTY_NAME="Nex Edgebox Rootfs"' /usr/lib/os-release ||
    fail "immutable release identity has the wrong name"
printf 'PASS: immutable release identity\n'

[[ -f /usr/share/pki/trust/anchors/ca-certificates.crt ]] ||
    fail "vendor CA anchors are missing"
[[ -x /usr/bin/update-ca-certificates ]] || fail "CA updater is missing"
[[ -x /usr/bin/trust ]] || fail "p11-kit trust command is missing"
[[ -f /usr/lib/pkcs11/p11-kit-trust.so ]] || fail "p11-kit trust module is missing"
[[ -L /etc/ssl/certs ]] || fail "CA compatibility path is not a link"
[[ "$(readlink /etc/ssl/certs)" == /run/ssl/certs ]] ||
    fail "CA compatibility path does not target the runtime cache"
[[ ! -e /run/ssl/certs ]] || fail "the flat root contains a generated CA cache"
[[ -L /usr/lib/systemd/system/sysinit.target.wants/update-ca-certificates.service ]] ||
    fail "CA update service is not enabled"

mkdir -p /run/ssl
ln -sT /usr/lib/ssl/certs /run/ssl/certs
update-ca-certificates --output-dir /run/ssl/certs
[[ -s /etc/ssl/certs/ca-certificates.crt ]] ||
    fail "runtime CA compatibility bundle is missing"

vendor_ca_count=$(grep -c 'BEGIN CERTIFICATE' \
    /usr/share/pki/trust/anchors/ca-certificates.crt)
[[ "$(grep -c 'BEGIN CERTIFICATE' /etc/ssl/certs/ca-certificates.crt)" -eq \
    "$vendor_ca_count" ]] || fail "assembled CA bundle does not match vendor anchors"

ca_test_dir=$(mktemp -d /tmp/ca-certificates-rootfs-test.XXXXXX)
mkdir -p /etc/pki/trust/blocklist
csplit -s -n 3 -f "$ca_test_dir/vendor-" \
    /usr/share/pki/trust/anchors/ca-certificates.crt \
    '/-----BEGIN CERTIFICATE-----/' '{*}'
cp "$ca_test_dir/vendor-001" /etc/pki/trust/blocklist/vendor.pem
update-ca-certificates --output-dir /run/ssl/certs
[[ "$(grep -c 'BEGIN CERTIFICATE' /etc/ssl/certs/ca-certificates.crt)" -eq \
    "$((vendor_ca_count - 1))" ]] || fail "administrator CA blocklist did not mask vendor anchor"
rm -f /etc/pki/trust/blocklist/vendor.pem
rmdir /etc/pki/trust/blocklist
update-ca-certificates --output-dir /run/ssl/certs
[[ "$(grep -c 'BEGIN CERTIFICATE' /etc/ssl/certs/ca-certificates.crt)" -eq \
    "$vendor_ca_count" ]] || fail "CA bundle did not restore vendor anchors"
rm -rf "$ca_test_dir"
systemd-analyze verify /usr/lib/systemd/system/update-ca-certificates.service
printf 'PASS: layered CA trust and generated compatibility store\n'

[[ -f /usr/lib/nftables/osf/pf.os ]] ||
    fail "Nftables vendor OS fingerprint database is missing"
[[ ! -e /etc/nftables/osf/pf.os ]] ||
    fail "Nftables installed its OS fingerprint database in /etc"
nft_rule=$(mktemp /tmp/nftables-rootfs-test.XXXXXX)
nft_log=$(mktemp /tmp/nftables-rootfs-log.XXXXXX)
printf '%s\n' \
    'table inet rootfs_osf_test {' \
    '  chain input {' \
    '    type filter hook input priority filter;' \
    '    osf name "Linux" accept' \
    '  }' \
    '}' > "$nft_rule"
unshare --net nft --debug mnl --check --file "$nft_rule" > "$nft_log" 2>&1 || {
    cat "$nft_log" >&2
    fail "nft did not check the OS fingerprint rule"
}
grep -F "Opening OS signature file '/usr/lib/nftables/osf/pf.os'" \
    "$nft_log" >/dev/null || fail "nft did not select the vendor OS database"
grep -F '45046:64:0:44:M*:' "$nft_log" >/dev/null ||
    fail "nft did not load the vendor OS fingerprint records"
rm -f "$nft_rule" "$nft_log"
printf 'PASS: Nftables vendor database and installed reader\n'

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
