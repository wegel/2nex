#!/bin/bash
# test nex bootloader in QEMU with OVMF
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BOOTLOADER="$ROOT_DIR/src/bootloader/target/x86_64-unknown-uefi/debug/nex-bootloader.efi"

# colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}===${NC} $1"; }
warn() { echo -e "${YELLOW}WARNING:${NC} $1"; }
error() { echo -e "${RED}ERROR:${NC} $1"; exit 1; }

# find OVMF firmware
find_ovmf() {
    local paths=(
        "/usr/share/edk2/x64/OVMF_CODE.4m.fd"
        "/usr/share/edk2/x64/OVMF_CODE.fd"
        "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd"
        "/usr/share/OVMF/OVMF_CODE.fd"
        "/usr/share/qemu/OVMF.fd"
    )
    for p in "${paths[@]}"; do
        if [ -f "$p" ]; then
            echo "$p"
            return 0
        fi
    done
    return 1
}

# check prerequisites
[ -f "$BOOTLOADER" ] || error "bootloader not found. Run: cargo build --manifest-path src/bootloader/Cargo.toml"

OVMF_CODE=$(find_ovmf) || error "OVMF firmware not found. Install edk2-ovmf"
OVMF_VARS_TEMPLATE="${OVMF_CODE/OVMF_CODE/OVMF_VARS}"
[ -f "$OVMF_VARS_TEMPLATE" ] || warn "OVMF_VARS not found, using code-only mode"

command -v qemu-system-x86_64 >/dev/null || error "qemu-system-x86_64 not found"

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

log "Using OVMF: $OVMF_CODE"
log "Bootloader: $BOOTLOADER"

# create ESP FAT32 image
log "Creating ESP image..."
ESP_IMG="$TMPDIR/esp.img"
dd if=/dev/zero of="$ESP_IMG" bs=1M count=64 status=none
mkfs.vfat -F 32 "$ESP_IMG" >/dev/null

# mount ESP and install bootloader
MOUNT_DIR="$TMPDIR/mnt"
mkdir -p "$MOUNT_DIR"

# use mtools instead of mounting (no root needed)
mmd -i "$ESP_IMG" ::/EFI
mmd -i "$ESP_IMG" ::/EFI/BOOT
mcopy -i "$ESP_IMG" "$BOOTLOADER" ::/EFI/BOOT/BOOTX64.EFI

log "Installed bootloader to ESP:/EFI/BOOT/BOOTX64.EFI"

# copy OVMF_VARS if available
OVMF_VARS="$TMPDIR/OVMF_VARS.fd"
if [ -f "$OVMF_VARS_TEMPLATE" ]; then
    cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS"
fi

# build qemu command
QEMU_ARGS=(
    -enable-kvm
    -machine q35
    -cpu host
    -m 512M
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
)

if [ -f "$OVMF_VARS" ]; then
    QEMU_ARGS+=(-drive "if=pflash,format=raw,file=$OVMF_VARS")
fi

QEMU_ARGS+=(
    -drive "file=$ESP_IMG,format=raw,if=virtio"
    -serial mon:stdio
    -display none
    -no-reboot
)

log "Starting QEMU..."
echo ""
echo "--- QEMU output (press Ctrl-A X to exit) ---"
echo ""

qemu-system-x86_64 "${QEMU_ARGS[@]}"

echo ""
log "QEMU exited"
