#!/bin/bash
# Verify factory seeding and the nex-install provision-file interface.
set -eu

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
SYSTEM_ROOT=${1:-}

if [ "$(id -u)" -ne 0 ] && [ "${NEX_PROVISION_TEST_IN_NAMESPACE:-0}" != 1 ]; then
    export NEX_PROVISION_TEST_IN_NAMESPACE=1
    exec unshare --user --map-root-user "$0" "$@"
fi

TEST_DIR=$(mktemp -d)

cleanup() {
    if [ -n "$SYSTEM_ROOT" ]; then
        rm -f "$SYSTEM_ROOT/etc/hosts"
    fi
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

DEPLOYMENT="$TEST_DIR/deployment"
FACTORY="$DEPLOYMENT/usr/share/factory/etc"
PROVISION="$TEST_DIR/provision"
TARGET_ETC="$TEST_DIR/target-etc"

mkdir -p "$FACTORY/pam.d" "$PROVISION/NetworkManager/system-connections" \
    "$PROVISION/systemd/network" "$TARGET_ETC"
printf '%s\n' 'root:x:0:0:root:/root:/bin/bash' > "$FACTORY/passwd"
printf '%s\n' 'root:x:0:' > "$FACTORY/group"
printf '%s\n' 'root:!*:19735:0:99999:7:::' > "$FACTORY/shadow"
printf '%s\n' 'factory policy' > "$FACTORY/pam.d/system-auth"

printf '%s\n' \
    'root:x:0:0:root:/root:/bin/bash' \
    'admin:x:1000:1000:Administrator:/home/admin:/bin/bash' \
    > "$PROVISION/passwd"
printf '%s\n' 'root:x:0:' 'admin:x:1000:' > "$PROVISION/group"
printf '%s\n' \
    'root:!*:19735:0:99999:7:::' \
    'admin:!*:19735:0:99999:7:::' \
    > "$PROVISION/shadow"
printf '%s\n' 'provisioned-host' > "$PROVISION/hostname"
chmod 777 "$PROVISION/hostname"
printf '%s\n' \
    '127.0.0.1 localhost' \
    '::1 localhost' \
    '127.0.1.1 provisioned-host' \
    > "$PROVISION/hosts"
ln -s /usr/share/zoneinfo/UTC "$PROVISION/localtime"
printf '%s\n' 'LANG=en_CA.UTF-8' > "$PROVISION/locale.conf"
printf '%s\n' 'KEYMAP=us' > "$PROVISION/vconsole.conf"
printf '%s\n' 'LABEL=nex / ext4 defaults 0 1' > "$PROVISION/fstab"
printf '%s\n' '[connection]' 'id=provisioned' 'type=ethernet' \
    > "$PROVISION/NetworkManager/system-connections/provisioned.nmconnection"
printf '%s\n' '[Match]' 'Name=en*' '[Network]' 'DHCP=yes' \
    > "$PROVISION/systemd/network/20-provisioned.network"

NEX_INSTALL_SOURCE_ONLY=1 . "$SCRIPT_DIR/nex-install"
seed_factory_etc "$DEPLOYMENT" "$TARGET_ETC"
copy_provision_tree "$PROVISION" "$TARGET_ETC"

grep -Fq 'admin:x:1000:1000:' "$TARGET_ETC/passwd"
grep -Fxq 'factory policy' "$TARGET_ETC/pam.d/system-auth"
grep -Fxq 'provisioned-host' "$TARGET_ETC/hostname"
grep -Fxq '127.0.0.1 localhost' "$TARGET_ETC/hosts"
test "$(readlink "$TARGET_ETC/localtime")" = /usr/share/zoneinfo/UTC
grep -Fxq 'LANG=en_CA.UTF-8' "$TARGET_ETC/locale.conf"
grep -Fxq 'KEYMAP=us' "$TARGET_ETC/vconsole.conf"
grep -Fxq 'LABEL=nex / ext4 defaults 0 1' "$TARGET_ETC/fstab"
test "$(stat -c %a "$TARGET_ETC/shadow")" = 600
test "$(stat -c %a "$TARGET_ETC/NetworkManager/system-connections/provisioned.nmconnection")" = 600
test "$(stat -c %a "$TARGET_ETC/hostname")" = 644
test "$(stat -c %u:%g "$TARGET_ETC/hostname")" = 0:0
test "$(stat -c %u:%g "$TARGET_ETC/localtime")" = 0:0
test -s "$TARGET_ETC/systemd/network/20-provisioned.network"

expect_rejected() {
    message=$1
    bad_tree=$2
    if (copy_provision_tree "$bad_tree" "$TARGET_ETC") >/dev/null 2>&1; then
        printf 'FAIL: installer accepted %s\n' "$message" >&2
        exit 1
    fi
}

BAD_FILE="$TEST_DIR/bad-file"
mkdir -p "$BAD_FILE"
printf '%s\n' must-not-replace > "$BAD_FILE/hostname"
printf '%s\n' forbidden > "$BAD_FILE/arbitrary.conf"
expect_rejected 'an unlisted provision path' "$BAD_FILE"
grep -Fxq 'provisioned-host' "$TARGET_ETC/hostname"

BAD_DIRECTORY="$TEST_DIR/bad-directory"
mkdir -p "$BAD_DIRECTORY/arbitrary"
expect_rejected 'an unlisted empty directory' "$BAD_DIRECTORY"

BAD_SYMLINK="$TEST_DIR/bad-symlink"
mkdir -p "$BAD_SYMLINK"
ln -s /tmp/hosts "$BAD_SYMLINK/hosts"
expect_rejected 'a non-timezone symlink' "$BAD_SYMLINK"

BAD_DIRECTORY_SYMLINK="$TEST_DIR/bad-directory-symlink"
mkdir -p "$BAD_DIRECTORY_SYMLINK"
ln -s /tmp "$BAD_DIRECTORY_SYMLINK/systemd"
expect_rejected 'a directory symlink' "$BAD_DIRECTORY_SYMLINK"

BAD_TIMEZONE="$TEST_DIR/bad-timezone"
mkdir -p "$BAD_TIMEZONE"
ln -s /usr/share/zoneinfo/Etc/../../etc/shadow "$BAD_TIMEZONE/localtime"
expect_rejected 'a timezone path containing parent traversal' "$BAD_TIMEZONE"

BAD_FIFO="$TEST_DIR/bad-fifo"
mkdir -p "$BAD_FIFO"
mkfifo "$BAD_FIFO/hostname"
expect_rejected 'a special file' "$BAD_FIFO"

if [ -n "$SYSTEM_ROOT" ]; then
    [ -d "$SYSTEM_ROOT" ] || {
        printf 'FAIL: system root not found: %s\n' "$SYSTEM_ROOT" >&2
        exit 1
    }
    [ ! -e "$SYSTEM_ROOT/etc/hosts" ] || {
        printf 'FAIL: test system root already has /etc/hosts\n' >&2
        exit 1
    }
    cp "$TARGET_ETC/hosts" "$SYSTEM_ROOT/etc/hosts"

    localhost_output=$(unshare --user --map-root-user --root "$SYSTEM_ROOT" \
        /usr/bin/getent ahostsv4 localhost)
    printf '%s\n' "$localhost_output" | grep -Fq '127.0.0.1'
    hostname_output=$(unshare --user --map-root-user --root "$SYSTEM_ROOT" \
        /usr/bin/getent ahostsv4 provisioned-host)
    printf '%s\n' "$hostname_output" | grep -Fq '127.0.1.1'
fi

printf 'PASS: nex-install factory and provision files\n'
