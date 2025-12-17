#!/bin/sh
# qemu-test-installer.sh: test the nex installer in QEMU
# usage: qemu-test-installer.sh [--boot-target]
#   default: boots from installer.img with empty target disk
#   --boot-target: boots from the installed target disk
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$ROOT_DIR/build"

INSTALLER_IMG="$BUILD_DIR/installer.img"
TARGET_IMG="$BUILD_DIR/installer-target.img"
TARGET_SIZE_MB=4096
BOOT_TARGET=false

if [ "${1:-}" = "--boot-target" ]; then
    BOOT_TARGET=true
fi

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
    [ -f "$INSTALLER_IMG" ] || { echo "error: installer image not found: $INSTALLER_IMG"; exit 1; }

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
