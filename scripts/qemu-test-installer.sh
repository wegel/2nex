#!/bin/sh
# qemu-test-installer.sh: test the nex installer in QEMU
# usage: qemu-test-installer.sh [--boot-target] [--direct-initramfs] [--rebuild] [--autoinstall] [--headless] [--timeout <secs>] [--extra-nex-var] [--assert-boot]
#   default: boots from installer.img with empty target disk
#   --boot-target: boots from the installed target disk
#   --rebuild: force rebuild of installer image
#   --autoinstall: add installer.autoinstall=/dev/sdb to kcmdline and run non-interactive install
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TMP_DIR="$ROOT_DIR/.nex/tmp"
NEX_BIN="${NEX_BIN:-$ROOT_DIR/src/cli/target/debug/nex}"

mkdir -p "$TMP_DIR"

INSTALLER_IMG="$TMP_DIR/installer.img"
TARGET_IMG="$TMP_DIR/installer-target.img"
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
ASSERT_PORT="${ASSERT_PORT:-10022}"
ASSERT_KEY="$TMP_DIR/qemu-assert-ed25519"
ASSERT_SERIAL_LOG="$TMP_DIR/qemu-assert-boot.serial.log"
ASSERT_PROBE_LOG="$TMP_DIR/qemu-assert-boot.probe.log"
INSTALL_SERIAL_LOG="$TMP_DIR/qemu-installer.serial.log"
DIRECT_ROOT="$TMP_DIR/direct-initramfs-root"
LINUX_BOOT_REF="${LINUX_BOOT_REF:-x86_64/pkg/core/kernel/linux/6.12.58/outputs/boot}"
INITRAMFS_BOOT_REF="${INITRAMFS_BOOT_REF:-x86_64/pkg/core/kernel/initramfs/1.0.0/outputs/boot}"
TARGET_REF="${TARGET_REF:-systems/desktop-vwl/0.0.1}"

die() {
    echo "error: $*" >&2
    exit 1
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
        --timeout)
            [ $# -ge 2 ] || die "--timeout requires seconds"
            TIMEOUT_SECS="${2:-}"
            shift 2
            ;;
        *) die "unknown option: $1" ;;
    esac
done

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

cmdline=$(cat /proc/cmdline)
echo "cmdline=$cmdline"
case " $cmdline " in
    *" zub="*) ;;
    *) fail "kernel command line does not contain zub=" ;;
esac

deploy_path=$(printf "%s\n" "$cmdline" | sed -n 's/.*zub=\([^ ]*\).*/\1/p')
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
source_contains /nex/repo "$expected_var"
source_contains /nex/staging "$expected_var"
source_contains /nex/users "$expected_var"
source_contains /nex/manifests "$expected_var"

assert_writable_dir /etc
assert_writable_dir /home
assert_writable_dir /root
assert_writable_dir /nex/repo
assert_writable_dir /nex/staging
assert_writable_dir /nex/users
assert_writable_dir /nex/manifests

systemctl is-active --quiet multi-user.target || fail "multi-user.target is not active"
systemctl is-system-running --no-pager || true
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

deploy_path=$(printf "%s\n" "$cmdline" | sed -n 's/.*zub=\([^ ]*\).*/\1/p')
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
source_contains /nex/repo "$expected_var"
source_contains /nex/staging "$expected_var"
source_contains /nex/users "$expected_var"
source_contains /nex/manifests "$expected_var"

assert_writable_dir /etc
assert_writable_dir /home
assert_writable_dir /root
assert_writable_dir /nex/repo
assert_writable_dir /nex/staging
assert_writable_dir /nex/users
assert_writable_dir /nex/manifests

systemctl is-system-running --no-pager || true
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

    if microcode_initrd=$(resolve_deploy_file "$root_dir" "$deploy_dir" boot/amd-ucode.cpio); then
        echo "including AMD early microcode initrd: $microcode_initrd"
        cat "$microcode_initrd" >> "$output_initramfs"
        microcode_size=$(wc -c < "$microcode_initrd")
        cmp -n "$microcode_size" "$microcode_initrd" "$output_initramfs" \
            || die "combined initramfs does not start with AMD microcode cpio"
    fi

    cat "$base_initramfs" >> "$output_initramfs"
}

build_direct_initramfs_disk() {
    command -v sfdisk >/dev/null 2>&1 || die "sfdisk not found"
    command -v mke2fs >/dev/null 2>&1 || die "mke2fs not found"
    command -v zub >/dev/null 2>&1 || die "zub not found"
    command -v qemu-system-x86_64 >/dev/null 2>&1 || die "qemu-system-x86_64 not found"

    rm -rf "$DIRECT_ROOT"
    mkdir -p "$DIRECT_ROOT"/{boot,initramfs,root-content,var-content}

    zub checkout --copy "$LINUX_BOOT_REF" "$DIRECT_ROOT/boot"
    zub checkout --copy "$INITRAMFS_BOOT_REF" "$DIRECT_ROOT/initramfs"

    TARGET_CHECKSUM=$(zub show "$TARGET_REF" 2>/dev/null | awk '/nex.system.checksum:/ {print $2; exit}')
    if [ -z "$TARGET_CHECKSUM" ]; then
        TARGET_CHECKSUM=$(zub rev-parse "$TARGET_REF" 2>/dev/null)
    fi
    [ -n "$TARGET_CHECKSUM" ] || die "could not get checksum for $TARGET_REF"

    DEPLOY_DIR="$DIRECT_ROOT/root-content/nex/deployments/${TARGET_CHECKSUM}.0"
    mkdir -p "$DEPLOY_DIR"
    zub checkout --copy "$TARGET_REF" "$DEPLOY_DIR"

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
        "$DIRECT_ROOT/var-content/nex/repo" \
        "$DIRECT_ROOT/var-content/nex/staging" \
        "$DIRECT_ROOT/var-content/nex/users" \
        "$DIRECT_ROOT/var-content/nex/manifests"
    chmod 1777 "$DIRECT_ROOT/var-content/tmp"
    cp -a "$DEPLOY_DIR/etc/." "$DIRECT_ROOT/var-content/etc/"
    touch "$DIRECT_ROOT/var-content/etc/.initialized"
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
    if command -v fakeroot >/dev/null 2>&1; then
        fakeroot -- sh -c "chown -R 0:0 '$DIRECT_ROOT/root-content' '$DIRECT_ROOT/var-content' && mke2fs -q -t ext4 -L nex-root -d '$DIRECT_ROOT/root-content' '$ROOT_IMG' 6144M && mke2fs -q -t ext4 -L nex-var -d '$DIRECT_ROOT/var-content' '$VAR_IMG' 1024M"
    else
        mke2fs -q -t ext4 -L nex-root -d "$DIRECT_ROOT/root-content" "$ROOT_IMG" 6144M
        mke2fs -q -t ext4 -L nex-var -d "$DIRECT_ROOT/var-content" "$VAR_IMG" 1024M
    fi

    ROOT_START=133120
    VAR_START=12716032
    dd if=/dev/zero of="$TARGET_IMG" bs=1M count=8192 status=none
    sfdisk "$TARGET_IMG" >/dev/null <<EOF
label: gpt
unit: sectors

start=2048, size=131072, type=uefi, name="EFI"
start=${ROOT_START}, size=12582912, type=linux, name="nex-root"
start=${VAR_START}, type=linux, name="nex-var"
EOF
    dd if="$ROOT_IMG" of="$TARGET_IMG" bs=512 seek="$ROOT_START" conv=notrunc status=none
    dd if="$VAR_IMG" of="$TARGET_IMG" bs=512 seek="$VAR_START" conv=notrunc status=none

    DIRECT_KERNEL="$DIRECT_ROOT/boot/boot/vmlinuz-6.12.58"
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
        "$NEX_BIN" build asm/installer/installer.yaml --update-checksum
        echo "building installer image..."
        if [ "$AUTOINSTALL" = "true" ]; then
            KCMDLINE_TMP="$TMP_DIR/kcmdline.qemu-autoinstall.txt"
            cp "$ROOT_DIR/src/bootloader/kcmdline.vm.txt" "$KCMDLINE_TMP"
            printf "\ninstaller.autoinstall=/dev/sdb\n" >> "$KCMDLINE_TMP"
            "$SCRIPT_DIR/create-installer-usb" --kcmdline "$KCMDLINE_TMP" systems/desktop-vwl/0.0.1 "$INSTALLER_IMG"
        else
            "$SCRIPT_DIR/create-installer-usb" systems/desktop-vwl/0.0.1 "$INSTALLER_IMG"
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
    echo ""
    if [ "$AUTOINSTALL" != "true" ]; then
        echo "in the shell, run: nex-install /dev/sdb"
        echo ""
    fi

    DISPLAY_MODE=gtk
    if [ "$AUTOINSTALL" = "true" ]; then
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
