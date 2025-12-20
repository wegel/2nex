#!/bin/sh
# qemu-test-installer.sh: test the nex installer in QEMU
# usage: qemu-test-installer.sh [--boot-target] [--rebuild]
#   default: boots from installer.img with empty target disk
#   --boot-target: boots from the installed target disk
#   --rebuild: force rebuild of installer image
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TMP_DIR="$ROOT_DIR/.nex/tmp"

mkdir -p "$TMP_DIR"

INSTALLER_IMG="$TMP_DIR/installer.img"
TARGET_IMG="$TMP_DIR/installer-target.img"
TARGET_SIZE_MB=16384
BOOT_TARGET=false
REBUILD=false

for arg in "$@"; do
    case "$arg" in
        --boot-target) BOOT_TARGET=true ;;
        --rebuild) REBUILD=true ;;
    esac
done

OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd
[ -f "$OVMF_CODE" ] || { echo "error: OVMF not found at $OVMF_CODE"; exit 1; }

if [ "$BOOT_TARGET" = "true" ]; then
    [ -f "$TARGET_IMG" ] || { echo "error: target image not found: $TARGET_IMG (run installer first)"; exit 1; }
    echo "booting installed system from: $TARGET_IMG"
    echo ""
    qemu-system-x86_64 \
        -enable-kvm \
        -machine q35 \
        -cpu host \
        -m 4G \
        -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
        -drive file="$TARGET_IMG",format=raw,if=none,id=disk \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk,bus=ahci.0 \
        -serial stdio \
        -display none \
        -no-reboot
else
    # build installer image if missing or --rebuild
    if [ ! -f "$INSTALLER_IMG" ] || [ "$REBUILD" = "true" ]; then
	./nex build asm/installer/manifest.yaml --update-checksum
        echo "building installer image..."
        "$SCRIPT_DIR/create-installer-usb" systems/desktop-vwl/0.0.1 "$INSTALLER_IMG"
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
    echo "in the shell, run: nex-install /dev/sdb"
    echo ""

    qemu-system-x86_64 \
        -enable-kvm \
        -machine q35 \
        -cpu host \
        -m 4G \
        -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
        -drive file="$INSTALLER_IMG",format=raw,if=none,id=installer \
        -device ahci,id=ahci \
        -device ide-hd,drive=installer,bus=ahci.0 \
        -drive file="$TARGET_IMG",format=raw,if=none,id=target \
        -device ide-hd,drive=target,bus=ahci.1 \
        -serial stdio \
        -display none \
        -no-reboot
fi
