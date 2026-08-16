#!/bin/sh
# qemu-test-installer.sh: test the nex installer in QEMU
# usage: qemu-test-installer.sh [--boot-target] [--direct-initramfs] [--rebuild] [--autoinstall] [--headless] [--timeout <secs>] [--extra-nex-var] [--assert-boot]
#        qemu-test-installer.sh --self-test-image-sizing
#   default: boots from installer.img with empty target disk
#   --boot-target: boots from the installed target disk
#   --rebuild: force rebuild of installer image
#   --autoinstall: add installer.autoinstall=/dev/sdb to kcmdline and run non-interactive install
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TMP_DIR="$ROOT_DIR/.nex/tmp"
NEX_BIN="${NEX_BIN:-$ROOT_DIR/src/cli/target/debug/nex}"
ZUB_BIN="${ZUB_BIN:-zub}"

mkdir -p "$TMP_DIR"

INSTALLER_IMG="${INSTALLER_IMG:-}"
TARGET_IMG="${TARGET_IMG:-}"
TARGET_SIZE_MB=16384
BOOT_TARGET=false
DIRECT_INITRAMFS=false
REBUILD=false
AUTOINSTALL=false
TIMEOUT_SECS=""
HEADLESS=false
EXTRA_NEX_VAR=false
EXTRA_NEX_VAR_IMG="$TMP_DIR/extra-nex-var.img"
ASSERT_BOOT=false
SELF_TEST_IMAGE_SIZING=false
ASSERT_PORT="${ASSERT_PORT:-10022}"
ASSERT_KEY="$TMP_DIR/qemu-assert-ed25519"
ASSERT_SERIAL_LOG="$TMP_DIR/qemu-assert-boot.serial.log"
ASSERT_PROBE_LOG="$TMP_DIR/qemu-assert-boot.probe.log"
INSTALL_SERIAL_LOG="$TMP_DIR/qemu-installer.serial.log"
DIRECT_ROOT="$TMP_DIR/direct-initramfs-root"
LINUX_BOOT_REF="${LINUX_BOOT_REF:-x86_64/pkg/core/kernel/linux/6.18.24/outputs/boot}"
INITRAMFS_BOOT_REF="${INITRAMFS_BOOT_REF:-x86_64/pkg/core/kernel/initramfs/1.0.0/outputs/boot}"
TARGET_REF="${TARGET_REF:-systems/desktop-vwl/0.0.1}"
TARGET_SLUG=$(printf "%s" "$TARGET_REF" | tr '/:' '__')
INSTALLER_IMG="${INSTALLER_IMG:-$TMP_DIR/installer-${TARGET_SLUG}.img}"
TARGET_IMG="${TARGET_IMG:-$TMP_DIR/installer-target-${TARGET_SLUG}.img}"
DIRECT_ROOT_MIN_MB="${DIRECT_ROOT_MIN_MB:-6144}"
DIRECT_VAR_MIN_MB="${DIRECT_VAR_MIN_MB:-1024}"
ESP_SIZE_MB=64

die() {
    echo "error: $*" >&2
    exit 1
}

filesystem_image_size_mb() {
    content_kib=$1
    minimum_mb=$2
    content_mb=$(((content_kib + 1023) / 1024))
    sized_mb=$((content_mb + content_mb / 4 + 512))
    if [ "$sized_mb" -lt "$minimum_mb" ]; then
        sized_mb=$minimum_mb
    fi
    printf '%s\n' "$sized_mb"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --boot-target) BOOT_TARGET=true; shift ;;
        --direct-initramfs) DIRECT_INITRAMFS=true; shift ;;
        --rebuild) REBUILD=true; shift ;;
        --autoinstall) AUTOINSTALL=true; shift ;;
        --headless) HEADLESS=true; shift ;;
        --extra-nex-var) EXTRA_NEX_VAR=true; shift ;;
        --assert-boot) ASSERT_BOOT=true; shift ;;
        --self-test-image-sizing) SELF_TEST_IMAGE_SIZING=true; shift ;;
        --timeout)
            [ $# -ge 2 ] || die "--timeout requires seconds"
            TIMEOUT_SECS="${2:-}"
            shift 2
            ;;
        *) die "unknown option: $1" ;;
    esac
done

if [ "$SELF_TEST_IMAGE_SIZING" = "true" ]; then
    [ "$(filesystem_image_size_mb 1024 6144)" -eq 6144 ] ||
        die "small root did not keep the minimum image size"
    [ "$(filesystem_image_size_mb 8388608 6144)" -eq 10752 ] ||
        die "large root did not receive 25 percent plus 512 MiB headroom"
    [ "$(filesystem_image_size_mb 1048576 256)" -eq 1792 ] ||
        die "content-based size did not supersede a smaller minimum"
    echo "PASS: installer direct image sizing"
    exit 0
fi

if [ "$AUTOINSTALL" = "true" ] && [ "$INSTALLER_IMG" = "$TMP_DIR/installer-${TARGET_SLUG}.img" ]; then
    INSTALLER_IMG="$TMP_DIR/installer-autoinstall-${TARGET_SLUG}.img"
fi

OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd
if [ "$DIRECT_INITRAMFS" != "true" ]; then
    [ -f "$OVMF_CODE" ] || die "OVMF not found at $OVMF_CODE"
fi

print_assert_logs() {
    echo ""
    echo "=== QEMU serial log ==="
    if [ -f "$ASSERT_SERIAL_LOG" ]; then
        cat "$ASSERT_SERIAL_LOG"
    else
        echo "(missing $ASSERT_SERIAL_LOG)"
    fi
    echo ""
    echo "=== guest probe log ==="
    if [ -f "$ASSERT_PROBE_LOG" ]; then
        cat "$ASSERT_PROBE_LOG"
    else
        echo "(missing $ASSERT_PROBE_LOG)"
    fi
}

ensure_assert_key() {
    command -v ssh >/dev/null 2>&1 || die "ssh not found"
    command -v ssh-keygen >/dev/null 2>&1 || die "ssh-keygen not found"
    if [ ! -f "$ASSERT_KEY" ]; then
        ssh-keygen -q -t ed25519 -N "" -f "$ASSERT_KEY"
    fi
}

ssh_probe() {
    ssh \
        -i "$ASSERT_KEY" \
        -p "$ASSERT_PORT" \
        -o BatchMode=yes \
        -o ConnectTimeout=5 \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        root@127.0.0.1 "$@"
}

wait_for_ssh() {
    timeout_secs=$1
    end_time=$(($(date +%s) + timeout_secs))
    while [ "$(date +%s)" -lt "$end_time" ]; do
        if ssh_probe "true" >/dev/null 2>&1; then
            return 0
        fi
        sleep 2
    done
    return 1
}

write_guest_assert_script() {
    ASSERT_GUEST_SCRIPT="$TMP_DIR/qemu-assert-boot.guest.sh"
    cat > "$ASSERT_GUEST_SCRIPT" <<'GUEST_ASSERT'
#!/bin/sh
set -eu

fail() {
    echo "ASSERT-FAIL: $*" >&2
    exit 1
}

source_for() {
    target=$1
    while read -r line; do
        set -- $line
        [ "$5" = "$target" ] || continue
        while [ "$1" != "-" ]; do
            shift
        done
        shift
        shift
        printf "%s\n" "$1"
        return 0
    done < /proc/self/mountinfo
}

mount_options_for() {
    target=$1
    while read -r line; do
        set -- $line
        [ "$5" = "$target" ] || continue
        printf "%s\n" "$6"
        return 0
    done < /proc/self/mountinfo
}

source_contains() {
    target=$1
    expected=$2
    src=$(source_for "$target")
    case "$src" in
        *"$expected"*) return 0 ;;
        *) fail "$target is mounted from $src, expected $expected" ;;
    esac
}

assert_writable_dir() {
    dir=$1
    [ -d "$dir" ] || fail "$dir is missing"
    probe="$dir/.nex-assert-boot.$$"
    : > "$probe" || fail "$dir is not writable"
    rm -f "$probe"
}

wait_for_systemd_ready() {
    deadline=$(($(date +%s) + 120))
    state=""
    while [ "$(date +%s)" -lt "$deadline" ]; do
        state=$(systemctl is-system-running --no-pager 2>/dev/null || true)
        case "$state" in
            running|degraded)
                echo "systemd-state=$state"
                return 0
                ;;
        esac
        sleep 2
    done
    fail "systemd did not settle, last state: $state"
}

cmdline=$(cat /proc/cmdline)
echo "cmdline=$cmdline"
case " $cmdline " in
    *" zub="*) ;;
    *) fail "kernel command line does not contain zub=" ;;
esac

deploy_path=""
for parameter in $cmdline; do
    case "$parameter" in
        zub=*) deploy_path=${parameter#zub=} ;;
    esac
done
[ -n "$deploy_path" ] || fail "kernel command line has an empty zub= value"
deploy_name=${deploy_path##*/}
root_mount=$(findmnt -n -o SOURCE / 2>/dev/null || true)
case "$root_mount" in
    *"$deploy_name"*) ;;
    *) fail "/ is mounted from $root_mount, expected deployment $deploy_name" ;;
esac

sysroot_opts=$(mount_options_for /sysroot)
case ",$sysroot_opts," in
    *,ro,*) ;;
    *) fail "/sysroot is not read-only: $sysroot_opts" ;;
esac

root_src=$(source_for /sysroot)
case "$root_src" in
    /dev/nvme*n*p2) expected_var="${root_src%p2}p3" ;;
    /dev/*2) expected_var="${root_src%2}3" ;;
    *) fail "cannot derive expected var partition from $root_src" ;;
esac

var_src=$(source_for /var)
[ "$var_src" = "$expected_var" ] || fail "/var is mounted from $var_src, expected $expected_var"

source_contains /etc "$expected_var"
source_contains /home "$expected_var"
source_contains /root "$expected_var"
source_contains /nex/repo "$root_src"
source_contains /nex/staging "$root_src"
source_contains /nex/users "$expected_var"
source_contains /nex/manifests "$expected_var"

assert_writable_dir /etc
assert_writable_dir /home
assert_writable_dir /root
assert_writable_dir /nex/repo
assert_writable_dir /nex/staging
assert_writable_dir /nex/users
assert_writable_dir /nex/manifests

wait_for_systemd_ready
if [ -x /usr/bin/update-ca-certificates ]; then
    systemctl is-active --quiet update-ca-certificates.service ||
        fail "update-ca-certificates.service is not active"
    [ -s /etc/ssl/certs/ca-certificates.crt ] ||
        fail "generated CA bundle is missing after boot"
    if [ -f /usr/lib/systemd/system/update-ca-certificates.service.d/runtime-store.conf ]; then
        [ -L /etc/ssl/certs ] ||
            fail "/etc/ssl/certs is not the runtime-cache compatibility link"
        [ "$(readlink /etc/ssl/certs)" = /run/ssl/certs ] ||
            fail "/etc/ssl/certs does not target /run/ssl/certs"
        [ -d /run/ssl/certs ] && [ ! -L /run/ssl/certs ] ||
            fail "/run/ssl/certs is not a generated directory"
        echo "system-ca-runtime=ready"
    fi
    echo "system-ca=ready"
fi
echo "ASSERT-BOOT-PASS"
GUEST_ASSERT
}

write_serial_assert_script() {
    serial_script=$1
    cat > "$serial_script" <<'GUEST_ASSERT'
#!/bin/sh
set -eu

say() {
    echo "$*" >/dev/console
    echo "$*"
}

fail() {
    say "ASSERT-FAIL: $*"
    poweroff -f
    exit 1
}

source_for() {
    target=$1
    while read -r line; do
        set -- $line
        [ "$5" = "$target" ] || continue
        while [ "$1" != "-" ]; do
            shift
        done
        shift
        shift
        printf "%s\n" "$1"
        return 0
    done < /proc/self/mountinfo
}

mount_options_for() {
    target=$1
    while read -r line; do
        set -- $line
        [ "$5" = "$target" ] || continue
        printf "%s\n" "$6"
        return 0
    done < /proc/self/mountinfo
}

source_contains() {
    target=$1
    expected=$2
    src=$(source_for "$target")
    case "$src" in
        *"$expected"*) return 0 ;;
        *) fail "$target is mounted from $src, expected $expected" ;;
    esac
}

assert_writable_dir() {
    dir=$1
    [ -d "$dir" ] || fail "$dir is missing"
    probe="$dir/.nex-assert-boot.$$"
    : > "$probe" || fail "$dir is not writable"
    rm -f "$probe"
}

cmdline=$(cat /proc/cmdline)
say "cmdline=$cmdline"
case " $cmdline " in
    *" zub="*) ;;
    *) fail "kernel command line does not contain zub=" ;;
esac

deploy_path=""
for parameter in $cmdline; do
    case "$parameter" in
        zub=*) deploy_path=${parameter#zub=} ;;
    esac
done
[ -n "$deploy_path" ] || fail "kernel command line has an empty zub= value"
deploy_name=${deploy_path##*/}
root_mount=$(findmnt -n -o SOURCE / 2>/dev/null || true)
case "$root_mount" in
    *"$deploy_name"*) ;;
    *) fail "/ is mounted from $root_mount, expected deployment $deploy_name" ;;
esac

sysroot_opts=$(mount_options_for /sysroot)
case ",$sysroot_opts," in
    *,ro,*) ;;
    *) fail "/sysroot is not read-only: $sysroot_opts" ;;
esac

root_src=$(source_for /sysroot)
case "$root_src" in
    /dev/nvme*n*p2) expected_var="${root_src%p2}p3" ;;
    /dev/*2) expected_var="${root_src%2}3" ;;
    *) fail "cannot derive expected var partition from $root_src" ;;
esac

var_src=$(source_for /var)
[ "$var_src" = "$expected_var" ] || fail "/var is mounted from $var_src, expected $expected_var"

source_contains /etc "$expected_var"
source_contains /home "$expected_var"
source_contains /root "$expected_var"
source_contains /nex/repo "$root_src"
source_contains /nex/staging "$root_src"
source_contains /nex/users "$expected_var"
source_contains /nex/manifests "$expected_var"

assert_writable_dir /etc
assert_writable_dir /home
assert_writable_dir /root
assert_writable_dir /nex/repo
assert_writable_dir /nex/staging
assert_writable_dir /nex/users
assert_writable_dir /nex/manifests

[ -s /etc/passwd ] || fail "/etc/passwd was not populated from package defaults"
root_user_found=false
while IFS=: read -r user_name _; do
    if [ "$user_name" = root ]; then
        root_user_found=true
        break
    fi
done < /etc/passwd
[ "$root_user_found" = true ] || fail "/etc/passwd has no root user"

grep -q '^root:!\*:' /etc/shadow || fail "root account is not locked"
while IFS=: read -r user_name _ user_id _; do
    case "$user_id" in
        ''|*[!0-9]*) continue ;;
    esac
    if [ "$user_id" -ge 1000 ] && [ "$user_id" -lt 65534 ]; then
        fail "generic system contains interactive account $user_name with UID $user_id"
    fi
done < /etc/passwd

if systemctl is-enabled --quiet sshd.service 2>/dev/null; then
    fail "sshd.service is enabled before provisioning"
fi
if systemctl is-active --quiet sshd.service 2>/dev/null; then
    fail "sshd.service is active before provisioning"
fi
if /usr/bin/timeout 1 /usr/bin/bash -c \
    'exec 3<>/dev/tcp/127.0.0.1/22' >/dev/null 2>&1; then
    fail "TCP port 22 accepts connections before provisioning"
fi
if printf '\n' | /usr/bin/setpriv --reuid=65534 --regid=65534 --clear-groups \
    /usr/bin/su root -c /usr/bin/true >/dev/null 2>&1; then
    fail "root accepts a blank password"
fi
say "generic-access-policy=ready"

IFS= read -r hosts_line < /etc/hosts || fail "/etc/hosts is missing"
[ "$hosts_line" = "host-owned hosts" ] || fail "/etc/hosts was overwritten: $hosts_line"

if [ -x /usr/bin/update-ca-certificates ]; then
    systemctl is-active --quiet update-ca-certificates.service ||
        fail "update-ca-certificates.service is not active"
    [ -s /etc/ssl/certs/ca-certificates.crt ] ||
        fail "generated CA bundle is missing after boot"
    if [ -f /usr/lib/systemd/system/update-ca-certificates.service.d/runtime-store.conf ]; then
        [ -L /etc/ssl/certs ] ||
            fail "/etc/ssl/certs is not the runtime-cache compatibility link"
        [ "$(readlink /etc/ssl/certs)" = /run/ssl/certs ] ||
            fail "/etc/ssl/certs does not target /run/ssl/certs"
        [ -d /run/ssl/certs ] && [ ! -L /run/ssl/certs ] ||
            fail "/run/ssl/certs is not a generated directory"
        say "system-ca-runtime=ready"
    fi
    say "system-ca=ready"
fi

state=$(systemctl is-system-running --no-pager 2>/dev/null || true)
say "systemd-state=$state"
say "ASSERT-BOOT-PASS"
poweroff -f
GUEST_ASSERT
}

resolve_deploy_file() {
    root_dir=$1
    deploy_dir=$2
    path=$3
    candidate="$deploy_dir/$path"

    if [ -s "$candidate" ]; then
        printf "%s\n" "$candidate"
        return 0
    fi

    if [ -L "$candidate" ]; then
        target=$(readlink "$candidate")
        case "$target" in
            /*) candidate="$root_dir$target" ;;
            *) candidate="$(dirname "$candidate")/$target" ;;
        esac

        if [ -s "$candidate" ]; then
            printf "%s\n" "$candidate"
            return 0
        fi
    fi

    return 1
}

build_combined_initramfs() {
    root_dir=$1
    deploy_dir=$2
    base_initramfs=$3
    output_initramfs=$4

    : > "$output_initramfs"

    for microcode_name in amd-ucode.cpio intel-ucode.cpio; do
        if microcode_initrd=$(resolve_deploy_file "$root_dir" "$deploy_dir" "boot/$microcode_name"); then
            echo "including early microcode initrd: $microcode_initrd"
            microcode_offset=$(wc -c < "$output_initramfs")
            cat "$microcode_initrd" >> "$output_initramfs"
            microcode_size=$(wc -c < "$microcode_initrd")
            cmp -n "$microcode_size" -i "0:$microcode_offset" "$microcode_initrd" "$output_initramfs" \
                || die "combined initramfs has $microcode_name at the wrong offset"
        fi
    done

    cat "$base_initramfs" >> "$output_initramfs"
}

build_direct_initramfs_disk() {
    command -v sfdisk >/dev/null 2>&1 || die "sfdisk not found"
    command -v mke2fs >/dev/null 2>&1 || die "mke2fs not found"
    command -v truncate >/dev/null 2>&1 || die "truncate not found"
    command -v "$ZUB_BIN" >/dev/null 2>&1 || die "zub not found: $ZUB_BIN"
    command -v qemu-system-x86_64 >/dev/null 2>&1 || die "qemu-system-x86_64 not found"

    rm -rf "$DIRECT_ROOT"
    mkdir -p "$DIRECT_ROOT"/{boot,initramfs,root-content,var-content}

    "$ZUB_BIN" checkout --copy "$LINUX_BOOT_REF" "$DIRECT_ROOT/boot"
    "$ZUB_BIN" checkout --copy "$INITRAMFS_BOOT_REF" "$DIRECT_ROOT/initramfs"

    TARGET_CHECKSUM=$("$ZUB_BIN" show "$TARGET_REF" 2>/dev/null | awk '/nex.system.checksum:/ {print $2; exit}')
    if [ -z "$TARGET_CHECKSUM" ]; then
        TARGET_CHECKSUM=$("$ZUB_BIN" rev-parse "$TARGET_REF" 2>/dev/null)
    fi
    [ -n "$TARGET_CHECKSUM" ] || die "could not get checksum for $TARGET_REF"

    DEPLOY_DIR="$DIRECT_ROOT/root-content/nex/deployments/${TARGET_CHECKSUM}.0"
    mkdir -p "$DEPLOY_DIR"
    "$ZUB_BIN" checkout --copy "$TARGET_REF" "$DEPLOY_DIR"

    mkdir -p \
        "$DIRECT_ROOT/root-content/nex/repo" \
        "$DIRECT_ROOT/root-content/nex/staging" \
        "$DIRECT_ROOT/root-content/proc" \
        "$DIRECT_ROOT/root-content/sys" \
        "$DIRECT_ROOT/root-content/dev" \
        "$DIRECT_ROOT/root-content/run" \
        "$DIRECT_ROOT/root-content/tmp"
    ln -sfn "deployments/${TARGET_CHECKSUM}.0" "$DIRECT_ROOT/root-content/nex/current"
    ln -sfn "current/nex/pkg" "$DIRECT_ROOT/root-content/nex/pkg"
    ln -sfn "current/nex/db" "$DIRECT_ROOT/root-content/nex/db"

    mkdir -p \
        "$DEPLOY_DIR/usr/lib" \
        "$DEPLOY_DIR/usr/lib/systemd/system" \
        "$DEPLOY_DIR/sysroot" \
        "$DEPLOY_DIR/var" \
        "$DEPLOY_DIR/etc" \
        "$DEPLOY_DIR/home" \
        "$DEPLOY_DIR/root" \
        "$DEPLOY_DIR/proc" \
        "$DEPLOY_DIR/sys" \
        "$DEPLOY_DIR/dev" \
        "$DEPLOY_DIR/run" \
        "$DEPLOY_DIR/tmp" \
        "$DEPLOY_DIR/nex/repo" \
        "$DEPLOY_DIR/nex/deployments" \
        "$DEPLOY_DIR/nex/staging" \
        "$DEPLOY_DIR/nex/users" \
        "$DEPLOY_DIR/nex/manifests"

    ln -sf "nex/deployments/${TARGET_CHECKSUM}.0/usr" "$DIRECT_ROOT/root-content/usr"
    ln -sf /usr/lib "$DIRECT_ROOT/root-content/lib"
    ln -sf "nex/deployments/${TARGET_CHECKSUM}.0/lib64" "$DIRECT_ROOT/root-content/lib64"
    ln -sf /usr/bin "$DIRECT_ROOT/root-content/bin"
    ln -sf /usr/bin "$DIRECT_ROOT/root-content/sbin"
    ln -sf /var/etc "$DIRECT_ROOT/root-content/etc"
    ln -sf /var/home "$DIRECT_ROOT/root-content/home"
    ln -sf /var/root "$DIRECT_ROOT/root-content/root"
    ln -sf usr/bin/init "$DIRECT_ROOT/root-content/init"

    mkdir -p \
        "$DIRECT_ROOT/var-content/etc/systemd/system/basic.target.wants" \
        "$DIRECT_ROOT/var-content/home" \
        "$DIRECT_ROOT/var-content/root" \
        "$DIRECT_ROOT/var-content/log/journal" \
        "$DIRECT_ROOT/var-content/lib/systemd/random-seed" \
        "$DIRECT_ROOT/var-content/lib/systemd/timers" \
        "$DIRECT_ROOT/var-content/lib/systemd/coredump" \
        "$DIRECT_ROOT/var-content/cache/fontconfig" \
        "$DIRECT_ROOT/var-content/tmp" \
        "$DIRECT_ROOT/var-content/nex/users" \
        "$DIRECT_ROOT/var-content/nex/manifests"
    chmod 1777 "$DIRECT_ROOT/var-content/tmp"
    printf 'host-owned hosts\n' > "$DIRECT_ROOT/var-content/etc/hosts"
    write_serial_assert_script "$DIRECT_ROOT/var-content/etc/nex-assert-boot.sh"
    chmod +x "$DIRECT_ROOT/var-content/etc/nex-assert-boot.sh"
    cp "$DIRECT_ROOT/var-content/etc/nex-assert-boot.sh" "$DEPLOY_DIR/usr/lib/nex-assert-boot.sh"
    cat > "$DIRECT_ROOT/var-content/etc/systemd/system/nex-assert-boot.service" <<'SERVICE'
[Unit]
Description=Nex boot assertion

[Service]
Type=oneshot
ExecStart=/bin/sh /usr/lib/nex-assert-boot.sh
StandardOutput=journal+console
StandardError=journal+console

[Install]
WantedBy=basic.target
SERVICE
    cp "$DIRECT_ROOT/var-content/etc/systemd/system/nex-assert-boot.service" "$DEPLOY_DIR/usr/lib/systemd/system/nex-assert-boot.service"
    ln -sfn ../nex-assert-boot.service "$DIRECT_ROOT/var-content/etc/systemd/system/basic.target.wants/nex-assert-boot.service"

    ROOT_IMG="$DIRECT_ROOT/root.img"
    VAR_IMG="$DIRECT_ROOT/var.img"
    root_content_kib=$(du -sk "$DIRECT_ROOT/root-content" | awk '{print $1}')
    var_content_kib=$(du -sk "$DIRECT_ROOT/var-content" | awk '{print $1}')
    root_size_mb=$(filesystem_image_size_mb "$root_content_kib" "$DIRECT_ROOT_MIN_MB")
    var_size_mb=$(filesystem_image_size_mb "$var_content_kib" "$DIRECT_VAR_MIN_MB")
    echo "creating direct images: root=${root_size_mb}MiB var=${var_size_mb}MiB"
    if command -v fakeroot >/dev/null 2>&1; then
        fakeroot -- sh -c "chown -R 0:0 '$DIRECT_ROOT/root-content' '$DIRECT_ROOT/var-content' && mke2fs -q -t ext4 -L nex-root -d '$DIRECT_ROOT/root-content' '$ROOT_IMG' ${root_size_mb}M && mke2fs -q -t ext4 -L nex-var -d '$DIRECT_ROOT/var-content' '$VAR_IMG' ${var_size_mb}M"
    else
        mke2fs -q -t ext4 -L nex-root -d "$DIRECT_ROOT/root-content" "$ROOT_IMG" "${root_size_mb}M"
        mke2fs -q -t ext4 -L nex-var -d "$DIRECT_ROOT/var-content" "$VAR_IMG" "${var_size_mb}M"
    fi

    ROOT_START=$((ESP_SIZE_MB * 2048 + 2048))
    ROOT_SECTORS=$((root_size_mb * 2048))
    VAR_START=$((ROOT_START + ROOT_SECTORS))
    disk_size_mb=$((ESP_SIZE_MB + root_size_mb + var_size_mb + 64))
    truncate -s "${disk_size_mb}M" "$TARGET_IMG"
    sfdisk "$TARGET_IMG" >/dev/null <<EOF
label: gpt
unit: sectors

start=2048, size=$((ESP_SIZE_MB * 2048)), type=uefi, name="EFI"
start=${ROOT_START}, size=${ROOT_SECTORS}, type=linux, name="nex-root"
start=${VAR_START}, type=linux, name="nex-var"
EOF
    dd if="$ROOT_IMG" of="$TARGET_IMG" bs=1M seek="$((ROOT_START / 2048))" \
      conv=notrunc,sparse status=none
    dd if="$VAR_IMG" of="$TARGET_IMG" bs=1M seek="$((VAR_START / 2048))" \
      conv=notrunc,sparse status=none

    DIRECT_KERNEL="$DIRECT_ROOT/boot/boot/vmlinuz-6.18.24"
    DIRECT_BASE_INITRAMFS="$DIRECT_ROOT/initramfs/boot/initramfs.cpio"
    DIRECT_INITRAMFS="$DIRECT_ROOT/initramfs/boot/combined-initramfs.cpio"
    build_combined_initramfs "$DIRECT_ROOT/root-content" "$DEPLOY_DIR" "$DIRECT_BASE_INITRAMFS" "$DIRECT_INITRAMFS"
    DIRECT_DEPLOY="/nex/deployments/${TARGET_CHECKSUM}.0"
}

assert_direct_initramfs_boot() {
    build_direct_initramfs_disk
    : > "$ASSERT_SERIAL_LOG"
    timeout_secs="${TIMEOUT_SECS:-180}"
    extra_drives=""
    if [ "$EXTRA_NEX_VAR" = "true" ]; then
        extra_drives="-drive file=$EXTRA_NEX_VAR_IMG,format=raw,if=none,id=extravardisk -device ide-hd,drive=extravardisk,bus=ahci.1"
    fi

    # shellcheck disable=SC2086
    qemu-system-x86_64 \
        -enable-kvm \
        -machine q35 \
        -cpu host \
        -m 4G \
        -kernel "$DIRECT_KERNEL" \
        -initrd "$DIRECT_INITRAMFS" \
        -append "console=ttyS0 root=/dev/sda2 zub=$DIRECT_DEPLOY systemd.unit=nex-assert-boot.service" \
        -drive file="$TARGET_IMG",format=raw,if=none,id=disk \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk,bus=ahci.0 \
        $extra_drives \
        -serial "file:$ASSERT_SERIAL_LOG" \
        -display none \
        -no-reboot &
    qemu_pid=$!

    end_time=$(($(date +%s) + timeout_secs))
    while [ "$(date +%s)" -lt "$end_time" ]; do
        if grep -q "ASSERT-BOOT-PASS" "$ASSERT_SERIAL_LOG" 2>/dev/null; then
            wait "$qemu_pid" 2>/dev/null || true
            cat "$ASSERT_SERIAL_LOG"
            return 0
        fi
        if grep -q "ASSERT-FAIL:" "$ASSERT_SERIAL_LOG" 2>/dev/null; then
            kill "$qemu_pid" 2>/dev/null || true
            wait "$qemu_pid" 2>/dev/null || true
            cat "$ASSERT_SERIAL_LOG"
            die "direct initramfs boot assertions failed"
        fi
        if ! kill -0 "$qemu_pid" 2>/dev/null; then
            wait "$qemu_pid" 2>/dev/null || true
            cat "$ASSERT_SERIAL_LOG"
            die "direct initramfs boot exited before assertions passed"
        fi
        sleep 2
    done

    kill "$qemu_pid" 2>/dev/null || true
    wait "$qemu_pid" 2>/dev/null || true
    cat "$ASSERT_SERIAL_LOG"
    die "direct initramfs boot assertions timed out"
}

assert_boot_target() {
    [ -f "$TARGET_IMG" ] || die "target image not found: $TARGET_IMG"
    ensure_assert_key
    write_guest_assert_script
    : > "$ASSERT_SERIAL_LOG"
    : > "$ASSERT_PROBE_LOG"

    timeout_secs="${TIMEOUT_SECS:-180}"
    extra_drives=""
    if [ "$EXTRA_NEX_VAR" = "true" ]; then
        extra_drives="-drive file=$EXTRA_NEX_VAR_IMG,format=raw,if=none,id=extravardisk -device ide-hd,drive=extravardisk,bus=ahci.1"
    fi

    # shellcheck disable=SC2086
    qemu-system-x86_64 \
        -enable-kvm \
        -machine q35 \
        -cpu host \
        -m 4G \
        -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
        -drive file="$TARGET_IMG",format=raw,if=none,id=disk \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk,bus=ahci.0 \
        $extra_drives \
        -netdev user,id=net0,hostfwd=tcp::"$ASSERT_PORT"-:22 \
        -device virtio-net-pci,netdev=net0 \
        -serial "file:$ASSERT_SERIAL_LOG" \
        -display none \
        -no-reboot &
    qemu_pid=$!

    if ! wait_for_ssh "$timeout_secs"; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
        print_assert_logs
        die "guest SSH probe did not become ready"
    fi

    if ! ssh_probe "cat >/tmp/nex-assert-boot.sh && sh /tmp/nex-assert-boot.sh" \
        < "$ASSERT_GUEST_SCRIPT" > "$ASSERT_PROBE_LOG" 2>&1; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
        print_assert_logs
        die "guest boot assertions failed"
    fi

    kill "$qemu_pid" 2>/dev/null || true
    wait "$qemu_pid" 2>/dev/null || true
    cat "$ASSERT_PROBE_LOG"
}

if [ "$EXTRA_NEX_VAR" = "true" ]; then
    # extra disk that looks like an already-installed system's var partition
    if [ ! -f "$EXTRA_NEX_VAR_IMG" ]; then
        echo "creating extra nex-var disk image..."
        dd if=/dev/zero of="$EXTRA_NEX_VAR_IMG" bs=1M count=128 status=none
        mke2fs -q -t ext4 -L nex-var "$EXTRA_NEX_VAR_IMG" 128M
    fi
fi

if [ "$DIRECT_INITRAMFS" = "true" ]; then
    [ "$ASSERT_BOOT" = "true" ] || die "--direct-initramfs requires --assert-boot"
    assert_direct_initramfs_boot
    exit 0
fi

if [ "$BOOT_TARGET" = "true" ]; then
    [ -f "$TARGET_IMG" ] || { echo "error: target image not found: $TARGET_IMG (run installer first)"; exit 1; }
    if [ "$ASSERT_BOOT" = "true" ]; then
        assert_boot_target
        exit 0
    fi

    echo "booting installed system from: $TARGET_IMG"
    echo ""

    DISPLAY_MODE=gtk
    if [ "$HEADLESS" = "true" ]; then
        DISPLAY_MODE=none
    fi

    QEMU_PREFIX=""
    if [ -n "$TIMEOUT_SECS" ] && command -v timeout >/dev/null 2>&1; then
        QEMU_PREFIX="timeout $TIMEOUT_SECS"
    fi

    EXTRA_DRIVES=""
    if [ "$EXTRA_NEX_VAR" = "true" ]; then
        EXTRA_DRIVES="-drive file=$EXTRA_NEX_VAR_IMG,format=raw,if=none,id=extravardisk -device ide-hd,drive=extravardisk,bus=ahci.1"
    fi

    # shellcheck disable=SC2086
    $QEMU_PREFIX qemu-system-x86_64 \
        -enable-kvm \
        -machine q35 \
        -cpu host \
        -m 4G \
        -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
        -drive file="$TARGET_IMG",format=raw,if=none,id=disk \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk,bus=ahci.0 \
        $EXTRA_DRIVES \
        -serial stdio \
        -display "$DISPLAY_MODE" \
        -no-reboot
else
    # build installer image if missing or --rebuild
    if [ ! -f "$INSTALLER_IMG" ] || [ "$REBUILD" = "true" ]; then
        [ -x "$NEX_BIN" ] || die "nex binary not found or not executable: $NEX_BIN"
        "$NEX_BIN" build asm/installer/installer.yaml --verbose
        echo "building installer image..."
        if [ "$AUTOINSTALL" = "true" ]; then
            KCMDLINE_TMP="$TMP_DIR/kcmdline.qemu-autoinstall.txt"
            cp "$ROOT_DIR/src/bootloader/kcmdline.vm.txt" "$KCMDLINE_TMP"
            printf "\ninstaller.autoinstall=/dev/sdb\n" >> "$KCMDLINE_TMP"
            "$SCRIPT_DIR/create-installer-usb" --force --kcmdline "$KCMDLINE_TMP" "$TARGET_REF" "$INSTALLER_IMG"
        else
            "$SCRIPT_DIR/create-installer-usb" --force "$TARGET_REF" "$INSTALLER_IMG"
        fi
        # also reset target disk when rebuilding installer
        rm -f "$TARGET_IMG"
    fi

    # create empty target disk if needed
    if [ ! -f "$TARGET_IMG" ]; then
        echo "creating ${TARGET_SIZE_MB}MB target disk..."
        dd if=/dev/zero of="$TARGET_IMG" bs=1M count=$TARGET_SIZE_MB status=none
    fi

    echo "booting installer..."
    echo "  installer: $INSTALLER_IMG"
    echo "  target:    $TARGET_IMG"
    echo "  system:    $TARGET_REF"
    echo ""
    if [ "$AUTOINSTALL" != "true" ]; then
        echo "in the shell, run: nex-install /dev/sdb"
        echo ""
    fi

    DISPLAY_MODE=gtk
    if [ "$AUTOINSTALL" = "true" ] || [ "$HEADLESS" = "true" ]; then
        DISPLAY_MODE=none
    fi

    if [ "$AUTOINSTALL" = "true" ] && [ -z "$TIMEOUT_SECS" ]; then
        TIMEOUT_SECS=240
    fi

    SERIAL_ARG="-serial stdio"
    if [ "$ASSERT_BOOT" = "true" ]; then
        : > "$INSTALL_SERIAL_LOG"
        SERIAL_ARG="-serial file:$INSTALL_SERIAL_LOG"
    fi

    QEMU_PREFIX=""
    if [ -n "$TIMEOUT_SECS" ] && command -v timeout >/dev/null 2>&1; then
        QEMU_PREFIX="timeout $TIMEOUT_SECS"
    fi

    EXTRA_DRIVES=""
    if [ "$EXTRA_NEX_VAR" = "true" ]; then
        EXTRA_DRIVES="-drive file=$EXTRA_NEX_VAR_IMG,format=raw,if=none,id=extravardisk -device ide-hd,drive=extravardisk,bus=ahci.2"
    fi

    # shellcheck disable=SC2086
    $QEMU_PREFIX qemu-system-x86_64 \
        -enable-kvm \
        -machine q35 \
        -cpu host \
        -m 4G \
        -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
        -drive file="$INSTALLER_IMG",format=raw,if=none,id=installer,snapshot=on \
        -device ahci,id=ahci \
        -device ide-hd,drive=installer,bus=ahci.0 \
        -drive file="$TARGET_IMG",format=raw,if=none,id=target \
        -device ide-hd,drive=target,bus=ahci.1 \
        $EXTRA_DRIVES \
        $SERIAL_ARG \
        -display "$DISPLAY_MODE" \
        -no-reboot

    if [ "$ASSERT_BOOT" = "true" ]; then
        assert_boot_target
    fi
fi
