#!/usr/bin/env bash
# Exercise the desktop's XDG search path with the installed Systemd generators.
set -euo pipefail

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

need_tool() {
    command -v "$1" >/dev/null 2>&1 || die "$1 not found"
}

if [[ "${DESKTOP_XDG_TEST_IN_NAMESPACE:-0}" != 1 ]]; then
    [[ $# -eq 1 ]] || die "usage: $0 ROOTFS"
    need_tool realpath
    need_tool unshare

    rootfs_under_test="$(realpath "$1")"
    [[ -d "$rootfs_under_test" ]] || die "rootfs is not a directory: $rootfs_under_test"
    [[ -x "$rootfs_under_test/usr/bin/bash" || -L "$rootfs_under_test/usr/bin/bash" ]] ||
        die "rootfs has no /usr/bin/bash"

    export DESKTOP_XDG_TEST_IN_NAMESPACE=1
    export DESKTOP_XDG_TEST_ROOT="$rootfs_under_test"
    export LC_ALL=C
    exec unshare --user --map-root-user --mount --pid --fork "$(realpath "$0")"
fi

need_tool chroot

chroot "$DESKTOP_XDG_TEST_ROOT" /usr/bin/bash -s <<'ROOTFS_TEST'
set -euo pipefail

export HOME=/tmp/xdg-autostart-home
export LC_ALL=C
export PATH=/usr/bin:/usr/sbin:/bin:/sbin

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

test_root=/tmp/xdg-autostart-test
vendor_test=/usr/share/xdg/autostart/tier-test.desktop
transient_test=/run/xdg/autostart/tier-test.desktop
administrator_test=/etc/xdg/autostart/tier-test.desktop

cleanup() {
    rm -rf "$HOME" "$test_root"
    rm -f "$vendor_test" "$transient_test" "$administrator_test"
}
trap cleanup EXIT
cleanup
mkdir -p "$HOME" "$test_root" /usr/share/xdg/autostart \
    /run/xdg/autostart /etc/xdg/autostart

environment_output=$(
    XDG_CONFIG_HOME="$HOME/.config" \
      /usr/lib/systemd/user-environment-generators/30-systemd-environment-d-generator
)
printf '%s\n' "$environment_output" |
    grep -qx 'XDG_CONFIG_DIRS=/etc/xdg:/run/xdg:/usr/share/xdg' ||
    fail "Systemd did not publish the assembly XDG search path"

export XDG_CONFIG_HOME="$HOME/.config"
export XDG_CONFIG_DIRS=/etc/xdg:/run/xdg:/usr/share/xdg
export XDG_CURRENT_DESKTOP=Unity

reset_generator_output() {
    rm -rf "$test_root/normal" "$test_root/early" "$test_root/late"
    mkdir -p "$test_root/normal" "$test_root/early" "$test_root/late"
}

run_generator() {
    reset_generator_output
    /usr/lib/systemd/user-generators/systemd-xdg-autostart-generator \
      "$test_root/normal" "$test_root/early" "$test_root/late"
}

assert_generated_source() {
    local source_path=$1
    grep -RlF "SourcePath=$source_path" "$test_root/late" >/dev/null ||
        fail "Systemd did not generate an autostart unit for $source_path"
}

run_generator
assert_generated_source /usr/share/xdg/autostart/gnome-keyring-pkcs11.desktop
assert_generated_source /usr/share/xdg/autostart/gnome-keyring-secrets.desktop
assert_generated_source /usr/share/xdg/autostart/at-spi-dbus-bus.desktop

cat > "$vendor_test" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Vendor tier
Exec=/usr/bin/true vendor
DESKTOP
run_generator
assert_generated_source "$vendor_test"

cat > "$transient_test" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Transient tier
Exec=/usr/bin/true transient
DESKTOP
run_generator
assert_generated_source "$transient_test"

cat > "$administrator_test" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Administrator tier
Exec=/usr/bin/true administrator
DESKTOP
run_generator
assert_generated_source "$administrator_test"

cat > "$administrator_test" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Administrator mask
Hidden=true
DESKTOP
run_generator
if grep -RF 'tier-test.desktop' "$test_root/late" >/dev/null; then
    fail "the administrator Hidden=true entry did not mask lower tiers"
fi

printf 'PASS: layered XDG autostart discovery\n'
ROOTFS_TEST
