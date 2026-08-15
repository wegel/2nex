#!/usr/bin/env bash
# qemu-test-graphical.sh: boot a Nex desktop and prove a graphical app renders.
#
# usage:
#   scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300
#   scripts/qemu-test-graphical.sh --graphics-mode gtk-debug --app chromium
#   scripts/qemu-test-graphical.sh --self-test-zub-override
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TMP_DIR="$ROOT_DIR/.nex/tmp"
ZUB_REPO="$ROOT_DIR/.nex/repo"
NEX_BIN="${NEX_BIN:-$ROOT_DIR/src/cli/target/debug/nex}"
ZUB_BIN="${ZUB_BIN:-zub}"

TARGET_REF="${TARGET_REF:-systems/desktop-vwl/0.0.1}"
APP="${APP:-chromium}"
GRAPHICS_MODE="${GRAPHICS_MODE:-auto}"
TIMEOUT_SECS="${TIMEOUT_SECS:-300}"
SSH_PORT="${SSH_PORT:-10024}"
MEMORY="${MEMORY:-6G}"
SMP="${SMP:-4}"
LINUX_BOOT_REF="${LINUX_BOOT_REF:-x86_64/pkg/core/kernel/linux/6.18.24/outputs/boot}"
INITRAMFS_BOOT_REF="${INITRAMFS_BOOT_REF:-x86_64/pkg/core/kernel/initramfs/1.0.0/outputs/boot}"

WORK_DIR="$TMP_DIR/graphical-smoke"
DIRECT_ROOT="$WORK_DIR/direct-root"
TARGET_IMG="$WORK_DIR/graphical-target.img"
SERIAL_LOG="$WORK_DIR/qemu.serial.log"
QEMU_LOG="$WORK_DIR/qemu.log"
ARTIFACT_DIR="$WORK_DIR/artifacts"
ASSERT_KEY="$WORK_DIR/qemu-assert-ed25519"

ROOT_SIZE_MB="${ROOT_SIZE_MB:-16384}"
VAR_SIZE_MB="${VAR_SIZE_MB:-2048}"
ESP_SIZE_MB=64

qemu_pid=""
SELECTED_MODE=""
DIRECT_KERNEL=""
DIRECT_INITRAMFS=""
DIRECT_DEPLOY=""
SELF_TEST_ZUB_OVERRIDE=0

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

usage() {
    sed -n '2,9p' "$0" >&2
}

run_zub() {
    "$ZUB_BIN" "$@"
}

while (($# > 0)); do
    case "$1" in
        --target-ref)
            (($# >= 2)) || die "--target-ref requires a ref"
            TARGET_REF="$2"
            shift 2
            ;;
        --app)
            (($# >= 2)) || die "--app requires a name"
            APP="$2"
            shift 2
            ;;
        --graphics-mode)
            (($# >= 2)) || die "--graphics-mode requires a mode"
            GRAPHICS_MODE="$2"
            shift 2
            ;;
        --timeout)
            (($# >= 2)) || die "--timeout requires seconds"
            TIMEOUT_SECS="$2"
            shift 2
            ;;
        --self-test-zub-override)
            SELF_TEST_ZUB_OVERRIDE=1
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            die "unknown option: $1"
            ;;
    esac
done

if ((SELF_TEST_ZUB_OVERRIDE)); then
    result=$(ZUB_BIN=printf run_zub '%s\n' 'PASS: graphical QEMU honors ZUB_BIN')
    [[ "$result" == 'PASS: graphical QEMU honors ZUB_BIN' ]] || exit 1
    printf '%s\n' "$result"
    exit 0
fi

if [[ "$APP" != "chromium" ]]; then
    die "unsupported app '$APP'; currently supported: chromium"
fi

cleanup_qemu() {
    if [[ -n "$qemu_pid" ]] && kill -0 "$qemu_pid" 2>/dev/null; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
}

trap cleanup_qemu EXIT INT TERM

require_commands() {
    local missing=0
    for command_name in \
        "$NEX_BIN" \
        qemu-system-x86_64 \
        "$ZUB_BIN" \
        sfdisk \
        mke2fs \
        ssh \
        scp \
        ssh-keygen
    do
        if ! command -v "$command_name" >/dev/null 2>&1; then
            printf 'missing required command: %s\n' "$command_name" >&2
            missing=1
        fi
    done
    ((missing == 0)) || die "install missing host commands and rerun"
}

qemu_display_has() {
    qemu-system-x86_64 -display help 2>/dev/null | grep -Eq "^$1$"
}

qemu_device_has() {
    qemu-system-x86_64 -device help 2>/dev/null | grep -Eq "name \"$1\""
}

select_graphics_mode() {
    case "$GRAPHICS_MODE" in
        auto)
            if qemu_display_has egl-headless && qemu_device_has virtio-vga-gl; then
                SELECTED_MODE=egl-headless-gl
                return
            fi
            if qemu_device_has virtio-vga; then
                SELECTED_MODE=virtio-vga-software
                return
            fi
            die "QEMU lacks egl-headless/virtio-vga-gl and virtio-vga"
            ;;
        egl-headless-gl)
            qemu_display_has egl-headless || die "QEMU lacks egl-headless display backend"
            qemu_device_has virtio-vga-gl || die "QEMU lacks virtio-vga-gl"
            SELECTED_MODE=egl-headless-gl
            ;;
        virtio-vga-software)
            qemu_device_has virtio-vga || die "QEMU lacks virtio-vga"
            SELECTED_MODE=virtio-vga-software
            ;;
        gtk-debug)
            qemu_display_has gtk || die "QEMU lacks gtk display backend"
            qemu_device_has virtio-vga-gl || die "QEMU lacks virtio-vga-gl"
            SELECTED_MODE=gtk-debug
            ;;
        *)
            die "unknown graphics mode '$GRAPHICS_MODE'"
            ;;
    esac
}

ensure_assert_key() {
    if [[ ! -f "$ASSERT_KEY" ]]; then
        ssh-keygen -q -t ed25519 -N "" -f "$ASSERT_KEY"
    fi
}

ssh_probe() {
    ssh \
        -i "$ASSERT_KEY" \
        -p "$SSH_PORT" \
        -o BatchMode=yes \
        -o ConnectTimeout=5 \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        root@127.0.0.1 "$@"
}

wait_for_ssh() {
    local timeout_secs=$1
    local end_time
    end_time=$(($(date +%s) + timeout_secs))
    while (($(date +%s) < end_time)); do
        if ssh_probe "true" >/dev/null 2>&1; then
            return 0
        fi
        sleep 2
    done
    return 1
}

resolve_deploy_file() {
    local root_dir=$1
    local deploy_dir=$2
    local path=$3
    local candidate="$deploy_dir/$path"
    local target

    if [[ -s "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return 0
    fi

    if [[ -L "$candidate" ]]; then
        target=$(readlink "$candidate")
        case "$target" in
            /*) candidate="$root_dir$target" ;;
            *) candidate="$(dirname "$candidate")/$target" ;;
        esac

        if [[ -s "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    fi

    return 1
}

build_combined_initramfs() {
    local root_dir=$1
    local deploy_dir=$2
    local base_initramfs=$3
    local output_initramfs=$4
    local microcode_name
    local microcode_initrd
    local microcode_offset
    local microcode_size

    : > "$output_initramfs"

    for microcode_name in amd-ucode.cpio intel-ucode.cpio; do
        if microcode_initrd=$(resolve_deploy_file "$root_dir" "$deploy_dir" "boot/$microcode_name"); then
            printf 'including early microcode initrd: %s\n' "$microcode_initrd"
            microcode_offset=$(wc -c < "$output_initramfs")
            cat "$microcode_initrd" >> "$output_initramfs"
            microcode_size=$(wc -c < "$microcode_initrd")
            cmp -n "$microcode_size" -i "0:$microcode_offset" "$microcode_initrd" "$output_initramfs" \
                || die "combined initramfs has $microcode_name at the wrong offset"
        fi
    done

    cat "$base_initramfs" >> "$output_initramfs"
}

write_guest_assertions() {
    local root_script=$1
    local user_script=$2

    cat > "$root_script" <<'ROOT_ASSERT'
#!/bin/sh
set -eu

LOG_DIR=/var/log/nex/graphical-smoke
USER_SCRIPT=/usr/lib/nex-assert-graphics-user.sh

say() {
    echo "$*" >/dev/console
    echo "$*"
}

fail() {
    say "ASSERT-FAIL: $*"
    sleep 300
    exit 1
}

mkdir -p "$LOG_DIR" /run/user/1000 /home/testuser
chown 1000:1000 "$LOG_DIR" /run/user/1000 /home/testuser
chmod 700 /run/user/1000

systemctl stop getty@tty1.service >/dev/null 2>&1 || true

if ! systemctl start nex-assert-graphics-user.service; then
    journalctl -u nex-assert-graphics-user.service --no-pager > "$LOG_DIR/assert.log" 2>&1 || true
    if [ -f "$LOG_DIR/assert-user.log" ]; then
        cat "$LOG_DIR/assert-user.log" >> "$LOG_DIR/assert.log"
    fi
    cat "$LOG_DIR/assert.log" >/dev/console || true
    fail "graphical guest assertions failed"
fi

journalctl -u nex-assert-graphics-user.service --no-pager > "$LOG_DIR/assert.log" 2>&1 || true
if [ -f "$LOG_DIR/assert-user.log" ]; then
    cat "$LOG_DIR/assert-user.log" >> "$LOG_DIR/assert.log"
fi
cat "$LOG_DIR/assert.log" >/dev/console || true
say "ASSERT-GRAPHICS-PASS"
touch "$LOG_DIR/PASS"
sleep 300
ROOT_ASSERT

    cat > "$user_script" <<'USER_ASSERT'
#!/bin/sh
set -eu

LOG_DIR=/var/log/nex/graphical-smoke
exec > "$LOG_DIR/assert-user.log" 2>&1

PAGE=$LOG_DIR/chromium-smoke.html
SHOT=$LOG_DIR/chromium-smoke.png
PIXELS=$LOG_DIR/pixels.txt
PROFILE=$LOG_DIR/chromium-profile
CHROMIUM_JSON=$LOG_DIR/chromium-json.txt

fail() {
    echo "ASSERT-FAIL: $*"
    exit 1
}

cleanup() {
    if [ -n "${chromium_pid:-}" ]; then
        kill "$chromium_pid" 2>/dev/null || true
    fi
    if [ -n "${vwl_pid:-}" ]; then
        kill "$vwl_pid" 2>/dev/null || true
    fi
}

trap cleanup EXIT

command -v vwl >/dev/null 2>&1 || fail "vwl not found"
command -v chromium >/dev/null 2>&1 || fail "chromium not found"
command -v grim >/dev/null 2>&1 || fail "grim not found"
command -v wlr-randr >/dev/null 2>&1 || fail "wlr-randr not found"
command -v identify >/dev/null 2>&1 || fail "identify not found"
command -v convert >/dev/null 2>&1 || fail "convert not found"
command -v python3 >/dev/null 2>&1 || fail "python3 not found"

if ! ls /dev/dri/card* /dev/dri/renderD* >/dev/null 2>&1; then
    fail "no DRM device exists under /dev/dri"
fi

mkdir -p "$PROFILE"
chmod 700 "$PROFILE"

cat > "$PAGE" <<'HTML'
<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>NEX_GRAPHICAL_SMOKE_READY</title>
  <style>
    html, body {
      margin: 0;
      width: 100%;
      height: 100%;
      overflow: hidden;
      background: rgb(12, 34, 56);
    }

    #marker {
      position: fixed;
      left: 20vw;
      top: 20vh;
      width: 60vw;
      height: 60vh;
      background: rgb(240, 0, 255);
    }

    #label {
      position: fixed;
      left: 10vw;
      top: 8vh;
      color: white;
      font: 48px sans-serif;
    }
  </style>
</head>
<body>
  <div id="marker"></div>
  <div id="label">NEX_GRAPHICAL_SMOKE_READY</div>
</body>
</html>
HTML

vwl > "$LOG_DIR/vwl.log" 2>&1 &
vwl_pid=$!

for _ in $(seq 1 60); do
    if [ -S "$XDG_RUNTIME_DIR/wayland-0" ]; then
        break
    fi
    if ! kill -0 "$vwl_pid" 2>/dev/null; then
        cat "$LOG_DIR/vwl.log" || true
        fail "vwl exited before creating a Wayland socket"
    fi
    sleep 1
done

[ -S "$XDG_RUNTIME_DIR/wayland-0" ] || fail "Wayland socket was not created"
export WAYLAND_DISPLAY=wayland-0

wlr-randr > "$LOG_DIR/wlr-randr.log" 2>&1 || {
    cat "$LOG_DIR/wlr-randr.log" || true
    fail "wlr-randr could not query outputs"
}

grep -Eq '[0-9]+x[0-9]+' "$LOG_DIR/wlr-randr.log" || {
    cat "$LOG_DIR/wlr-randr.log" || true
    fail "wlr-randr did not report an output mode"
}

chromium \
    --ozone-platform=wayland \
    --enable-features=UseOzonePlatform \
    --user-data-dir="$PROFILE" \
    --no-first-run \
    --disable-background-networking \
    --disable-default-apps \
    --disable-sync \
    --remote-debugging-address=127.0.0.1 \
    --remote-debugging-port=9222 \
    --window-size=1024,768 \
    "file://$PAGE" > "$LOG_DIR/chromium.log" 2>&1 &
chromium_pid=$!

python3 - "$CHROMIUM_JSON" <<'PY' || fail "Chromium remote debugging did not report the test page"
import json
import sys
import time
import urllib.request

output_path = sys.argv[1]
deadline = time.time() + 90
last_error = None
while time.time() < deadline:
    try:
        with urllib.request.urlopen("http://127.0.0.1:9222/json", timeout=2) as response:
            targets = json.loads(response.read().decode("utf-8"))
        with open(output_path, "w", encoding="utf-8") as handle:
            json.dump(targets, handle, indent=2, sort_keys=True)
        for target in targets:
            title = target.get("title", "")
            url = target.get("url", "")
            if "NEX_GRAPHICAL_SMOKE_READY" in title and url.startswith("file://"):
                print("remote-debugging-title=NEX_GRAPHICAL_SMOKE_READY")
                sys.exit(0)
    except Exception as error:
        last_error = error
    time.sleep(1)

print(f"last remote debugging error: {last_error}", file=sys.stderr)
sys.exit(1)
PY

sleep 4
kill -0 "$chromium_pid" 2>/dev/null || fail "Chromium exited before screenshot capture"

grim "$SHOT" > "$LOG_DIR/grim.log" 2>&1 || {
    cat "$LOG_DIR/grim.log" || true
    fail "grim could not capture a screenshot"
}

[ -s "$SHOT" ] || fail "screenshot is empty"
dimensions=$(identify -format '%w %h' "$SHOT")
set -- $dimensions
width=$1
height=$2
[ "$width" -ge 320 ] || fail "screenshot width is too small: $width"
[ "$height" -ge 240 ] || fail "screenshot height is too small: $height"

center_x=$((width / 2))
center_y=$((height / 2))
pixel=$(convert "$SHOT" -format "%[pixel:p{$center_x,$center_y}]" info:)
printf 'dimensions=%s\ncenter=%s,%s\ncenter-pixel=%s\n' "$dimensions" "$center_x" "$center_y" "$pixel" > "$PIXELS"

case "$pixel" in
    *"srgb(240,0,255)"*|*"srgba(240,0,255"*)
        ;;
    *)
        cat "$PIXELS"
        fail "screenshot center pixel did not match the page marker"
        ;;
esac

echo "display-device=$(ls /dev/dri/card* /dev/dri/renderD* 2>/dev/null | tr '\n' ' ')"
cat "$PIXELS"
echo "chromium-command=chromium --ozone-platform=wayland --remote-debugging-port=9222 file://$PAGE"
echo "guest-artifact-dir=$LOG_DIR"
USER_ASSERT

    chmod +x "$root_script" "$user_script"
}

build_direct_initramfs_disk() {
    local target_checksum
    local deploy_dir
    local root_img
    local var_img
    local root_start
    local root_sectors
    local var_start
    local disk_size_mb
    local var_content
    local root_content
    local public_key
    local base_initramfs

    "$NEX_BIN" check "$ROOT_DIR/asm/desktop-vwl/desktop-vwl.yaml"

    rm -rf "$WORK_DIR"
    mkdir -p "$DIRECT_ROOT"/{boot,initramfs,root-content,var-content} "$ARTIFACT_DIR"
    : > "$SERIAL_LOG"
    : > "$QEMU_LOG"

    ensure_assert_key

    run_zub --repo "$ZUB_REPO" checkout --copy "$LINUX_BOOT_REF" "$DIRECT_ROOT/boot"
    run_zub --repo "$ZUB_REPO" checkout --copy "$INITRAMFS_BOOT_REF" "$DIRECT_ROOT/initramfs"

    target_checksum=$(run_zub --repo "$ZUB_REPO" show "$TARGET_REF" 2>/dev/null | awk '/nex.system.checksum:/ {print $2; exit}')
    if [[ -z "$target_checksum" ]]; then
        target_checksum=$(run_zub --repo "$ZUB_REPO" rev-parse "$TARGET_REF" 2>/dev/null)
    fi
    [[ -n "$target_checksum" ]] || die "could not get checksum for $TARGET_REF"

    deploy_dir="$DIRECT_ROOT/root-content/nex/deployments/${target_checksum}.0"
    mkdir -p "$deploy_dir"
    run_zub --repo "$ZUB_REPO" checkout --copy "$TARGET_REF" "$deploy_dir"

    root_content="$DIRECT_ROOT/root-content"
    var_content="$DIRECT_ROOT/var-content"

    mkdir -p \
        "$root_content/nex/repo" \
        "$root_content/nex/staging" \
        "$root_content/proc" \
        "$root_content/sys" \
        "$root_content/dev" \
        "$root_content/run" \
        "$root_content/tmp"
    ln -sfn "deployments/${target_checksum}.0" "$root_content/nex/current"
    ln -sfn "current/nex/pkg" "$root_content/nex/pkg"
    ln -sfn "current/nex/db" "$root_content/nex/db"

    mkdir -p \
        "$deploy_dir/usr/lib" \
        "$deploy_dir/usr/lib/systemd/system" \
        "$deploy_dir/sysroot" \
        "$deploy_dir/var" \
        "$deploy_dir/etc" \
        "$deploy_dir/home" \
        "$deploy_dir/root" \
        "$deploy_dir/proc" \
        "$deploy_dir/sys" \
        "$deploy_dir/dev" \
        "$deploy_dir/run" \
        "$deploy_dir/tmp" \
        "$deploy_dir/nex/repo" \
        "$deploy_dir/nex/deployments" \
        "$deploy_dir/nex/staging" \
        "$deploy_dir/nex/users" \
        "$deploy_dir/nex/manifests"

    ln -sf "nex/deployments/${target_checksum}.0/usr" "$root_content/usr"
    ln -sf /usr/lib "$root_content/lib"
    ln -sf "nex/deployments/${target_checksum}.0/lib64" "$root_content/lib64"
    ln -sf /usr/bin "$root_content/bin"
    ln -sf /usr/bin "$root_content/sbin"
    ln -sf /var/etc "$root_content/etc"
    ln -sf /var/home "$root_content/home"
    ln -sf /var/root "$root_content/root"
    ln -sf usr/bin/init "$root_content/init"

    mkdir -p \
        "$var_content/etc/systemd/system/multi-user.target.wants" \
        "$var_content/home/testuser" \
        "$var_content/root/.ssh" \
        "$var_content/log/journal" \
        "$var_content/lib/sshd" \
        "$var_content/lib/systemd/random-seed" \
        "$var_content/lib/systemd/timers" \
        "$var_content/lib/systemd/coredump" \
        "$var_content/cache/fontconfig" \
        "$var_content/tmp" \
        "$var_content/nex/repo" \
        "$var_content/nex/staging" \
        "$var_content/nex/users" \
        "$var_content/nex/manifests"
    chmod 1777 "$var_content/tmp"
    chmod 700 "$var_content/lib/sshd" "$var_content/root/.ssh"
    cp -a "$deploy_dir/etc/." "$var_content/etc/"
    if [[ -d "$deploy_dir/home/testuser" ]]; then
        cp -a "$deploy_dir/home/testuser/." "$var_content/home/testuser/"
    fi
    touch "$var_content/etc/.initialized"

    public_key=$(cat "$ASSERT_KEY.pub")
    printf '%s\n' "$public_key" > "$var_content/root/.ssh/authorized_keys"
    chmod 600 "$var_content/root/.ssh/authorized_keys"

    write_guest_assertions \
        "$var_content/etc/nex-assert-graphics.sh" \
        "$var_content/etc/nex-assert-graphics-user.sh"
    cp "$var_content/etc/nex-assert-graphics.sh" "$deploy_dir/usr/lib/nex-assert-graphics.sh"
    cp "$var_content/etc/nex-assert-graphics-user.sh" "$deploy_dir/usr/lib/nex-assert-graphics-user.sh"

    cat > "$var_content/etc/systemd/system/nex-assert-graphics.service" <<'SERVICE'
[Unit]
Description=Nex graphical smoke assertion
Wants=multi-user.target dbus.service systemd-logind.service sshd.service
After=multi-user.target dbus.service systemd-logind.service sshd.service systemd-udev-settle.service

[Service]
Type=oneshot
ExecStart=/bin/sh /usr/lib/nex-assert-graphics.sh
StandardOutput=journal+console
StandardError=journal+console

[Install]
WantedBy=multi-user.target
SERVICE
    cp "$var_content/etc/systemd/system/nex-assert-graphics.service" "$deploy_dir/usr/lib/systemd/system/nex-assert-graphics.service"
    ln -sfn ../nex-assert-graphics.service "$var_content/etc/systemd/system/multi-user.target.wants/nex-assert-graphics.service"

    cat > "$var_content/etc/systemd/system/nex-assert-graphics-user.service" <<'SERVICE'
[Unit]
Description=Nex graphical smoke user session
Conflicts=getty@tty1.service
After=systemd-logind.service systemd-user-sessions.service

[Service]
Type=oneshot
User=testuser
Group=testuser
SupplementaryGroups=video render input seat audio
PAMName=login
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes
StandardInput=tty-force
StandardOutput=journal+console
StandardError=journal+console
Environment=HOME=/home/testuser
Environment=XDG_RUNTIME_DIR=/run/user/1000
Environment=XDG_SESSION_TYPE=wayland
Environment=WLR_LIBINPUT_NO_DEVICES=1
Environment=WLR_RENDERER=pixman
Environment=MOZ_ENABLE_WAYLAND=1
Environment=QT_QPA_PLATFORM=wayland
ExecStart=/bin/sh /usr/lib/nex-assert-graphics-user.sh
SERVICE
    cp "$var_content/etc/systemd/system/nex-assert-graphics-user.service" "$deploy_dir/usr/lib/systemd/system/nex-assert-graphics-user.service"

    root_img="$DIRECT_ROOT/root.img"
    var_img="$DIRECT_ROOT/var.img"
    if command -v fakeroot >/dev/null 2>&1; then
        fakeroot -- bash -c "chown -R 0:0 '$root_content' '$var_content' && mke2fs -q -t ext4 -L nex-root -d '$root_content' '$root_img' ${ROOT_SIZE_MB}M && mke2fs -q -t ext4 -L nex-var -d '$var_content' '$var_img' ${VAR_SIZE_MB}M"
    else
        mke2fs -q -t ext4 -L nex-root -d "$root_content" "$root_img" "${ROOT_SIZE_MB}M"
        mke2fs -q -t ext4 -L nex-var -d "$var_content" "$var_img" "${VAR_SIZE_MB}M"
    fi

    root_start=$((ESP_SIZE_MB * 2048 + 2048))
    root_sectors=$((ROOT_SIZE_MB * 2048))
    var_start=$((root_start + root_sectors))
    disk_size_mb=$((ESP_SIZE_MB + ROOT_SIZE_MB + VAR_SIZE_MB + 64))

    dd if=/dev/zero of="$TARGET_IMG" bs=1M count="$disk_size_mb" status=none
    sfdisk "$TARGET_IMG" >/dev/null <<EOF
label: gpt
unit: sectors

start=2048, size=$((ESP_SIZE_MB * 2048)), type=uefi, name="EFI"
start=${root_start}, size=${root_sectors}, type=linux, name="nex-root"
start=${var_start}, type=linux, name="nex-var"
EOF
    dd if="$root_img" of="$TARGET_IMG" bs=512 seek="$root_start" conv=notrunc status=none
    dd if="$var_img" of="$TARGET_IMG" bs=512 seek="$var_start" conv=notrunc status=none

    DIRECT_KERNEL=$(find "$DIRECT_ROOT/boot/boot" -maxdepth 1 -type f -name 'vmlinuz-*' | sort | head -n 1)
    base_initramfs=$(find "$DIRECT_ROOT/initramfs/boot" -maxdepth 1 -type f -name 'initramfs*.cpio' ! -name 'combined-*' | sort | head -n 1)
    [[ -n "$DIRECT_KERNEL" ]] || die "could not find checked-out kernel"
    [[ -n "$base_initramfs" ]] || die "could not find checked-out initramfs"
    DIRECT_INITRAMFS="$DIRECT_ROOT/initramfs/boot/combined-initramfs.cpio"
    build_combined_initramfs "$root_content" "$deploy_dir" "$base_initramfs" "$DIRECT_INITRAMFS"
    DIRECT_DEPLOY="/nex/deployments/${target_checksum}.0"
}

qemu_graphics_args() {
    case "$SELECTED_MODE" in
        egl-headless-gl)
            printf '%s\n' -device virtio-vga-gl -display egl-headless,gl=on
            ;;
        virtio-vga-software)
            printf '%s\n' -device virtio-vga -display none
            ;;
        gtk-debug)
            printf '%s\n' -device virtio-vga-gl -display gtk,gl=on
            ;;
        *)
            die "no graphics mode selected"
            ;;
    esac
}

collect_artifacts() {
    local artifact_name

    mkdir -p "$ARTIFACT_DIR"
    if wait_for_ssh 45; then
        for artifact_name in \
            PASS \
            assert.log \
            assert-user.log \
            chromium-json.txt \
            chromium.log \
            chromium-smoke.html \
            chromium-smoke.png \
            grim.log \
            pixels.txt \
            vwl.log \
            wlr-randr.log
        do
            scp \
                -i "$ASSERT_KEY" \
                -P "$SSH_PORT" \
                -o BatchMode=yes \
                -o ConnectTimeout=5 \
                -o StrictHostKeyChecking=no \
                -o UserKnownHostsFile=/dev/null \
                "root@127.0.0.1:/var/log/nex/graphical-smoke/$artifact_name" \
                "$ARTIFACT_DIR/" \
                >> "$QEMU_LOG" 2>&1 || true
        done
    else
        printf 'guest SSH did not become ready for artifact copy\n' >> "$QEMU_LOG"
    fi
}

print_logs() {
    printf '\n=== graphical smoke paths ===\n'
    printf 'work dir: %s\n' "$WORK_DIR"
    printf 'serial log: %s\n' "$SERIAL_LOG"
    printf 'qemu log: %s\n' "$QEMU_LOG"
    printf 'artifact dir: %s\n' "$ARTIFACT_DIR"

    if [[ -f "$SERIAL_LOG" ]]; then
        printf '\n=== QEMU serial log ===\n'
        cat "$SERIAL_LOG"
    fi

    if [[ -f "$ARTIFACT_DIR/assert.log" ]]; then
        printf '\n=== guest assertion log ===\n'
        cat "$ARTIFACT_DIR/assert.log"
    fi

    if [[ -f "$ARTIFACT_DIR/pixels.txt" ]]; then
        printf '\n=== screenshot pixel proof ===\n'
        cat "$ARTIFACT_DIR/pixels.txt"
    fi
}

run_qemu() {
    local timeout_secs=$1
    local end_time
    local accel_args=()
    local graphics_args=()
    local qemu_args=()

    if [[ -e /dev/kvm ]]; then
        accel_args=(-enable-kvm -cpu host)
    else
        accel_args=(-accel tcg -cpu max)
    fi

    mapfile -t graphics_args < <(qemu_graphics_args)

    qemu_args=(
        "${accel_args[@]}"
        -machine q35
        -m "$MEMORY"
        -smp "$SMP"
        -kernel "$DIRECT_KERNEL"
        -initrd "$DIRECT_INITRAMFS"
        -append "console=ttyS0 root=/dev/sda2 zub=$DIRECT_DEPLOY systemd.unit=nex-assert-graphics.service nex.novwl=1"
        -drive "file=$TARGET_IMG,format=raw,if=none,id=disk"
        -device ahci,id=ahci
        -device ide-hd,drive=disk,bus=ahci.0
        "${graphics_args[@]}"
        -device virtio-keyboard-pci
        -device virtio-tablet-pci
        -netdev "user,id=net0,hostfwd=tcp:127.0.0.1:${SSH_PORT}-:22"
        -device virtio-net-pci,netdev=net0
        -serial "file:$SERIAL_LOG"
        -no-reboot
    )

    printf 'target-ref=%s\n' "$TARGET_REF" | tee "$QEMU_LOG"
    printf 'graphics-mode=%s\n' "$SELECTED_MODE" | tee -a "$QEMU_LOG"
    printf 'qemu-command=' | tee -a "$QEMU_LOG"
    printf '%q ' qemu-system-x86_64 "${qemu_args[@]}" | tee -a "$QEMU_LOG"
    printf '\n' | tee -a "$QEMU_LOG"

    qemu-system-x86_64 "${qemu_args[@]}" >> "$QEMU_LOG" 2>&1 &
    qemu_pid=$!

    end_time=$(($(date +%s) + timeout_secs))
    while (($(date +%s) < end_time)); do
        if grep -q "ASSERT-GRAPHICS-PASS" "$SERIAL_LOG" 2>/dev/null; then
            collect_artifacts
            print_logs
            return 0
        fi
        if grep -q "ASSERT-FAIL:" "$SERIAL_LOG" 2>/dev/null; then
            collect_artifacts
            print_logs
            die "graphical smoke failed"
        fi
        if ! kill -0 "$qemu_pid" 2>/dev/null; then
            wait "$qemu_pid" 2>/dev/null || true
            collect_artifacts
            print_logs
            die "QEMU exited before graphical assertions passed"
        fi
        sleep 2
    done

    collect_artifacts
    print_logs
    die "graphical smoke timed out after ${timeout_secs}s"
}

require_commands
select_graphics_mode
printf 'selected QEMU graphics mode: %s\n' "$SELECTED_MODE"
build_direct_initramfs_disk
run_qemu "$TIMEOUT_SECS"
