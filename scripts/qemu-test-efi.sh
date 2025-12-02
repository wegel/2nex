#!/bin/bash
# complete EFI boot test: bootloader -> kernel -> initramfs -> systemd + nex
# tests the full boot chain with actual nex system
#
# usage: ./qemu-test-efi.sh [system-ref]
#   system-ref: zub ref to boot (default: systems/bootable-systemd-nex/0.0.1)
#   examples:
#     ./qemu-test-efi.sh
#     ./qemu-test-efi.sh systems/bootable-minimal/0.0.1
set -eux

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
REPO="$ROOT_DIR/.nex/repo"
BOOTLOADER="$ROOT_DIR/src/bootloader/target/x86_64-unknown-uefi/debug/nex-bootloader.efi"
OUTPUT="$ROOT_DIR/build/efi-test-disk.img"
SYSTEM_REF="${1:-systems/bootable-systemd-nex/0.0.1}"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

log() { echo -e "${GREEN}===${NC} $1"; }
error() { echo -e "${RED}ERROR:${NC} $1"; exit 1; }

# check prerequisites
[ -f "$BOOTLOADER" ] || error "bootloader not found. Build with: cd src/bootloader && cargo build"
command -v mke2fs >/dev/null || error "mke2fs not found (install e2fsprogs)"
command -v mcopy >/dev/null || error "mcopy not found (install mtools)"
command -v qemu-system-x86_64 >/dev/null || error "qemu-system-x86_64 not found"
ZUB="$ROOT_DIR/src/zub/target/debug/zub"
[ -f "$ZUB" ] || error "zub not found. Build with: cd src/zub && cargo build"
$ZUB --repo="$REPO" refs | grep -q "$SYSTEM_REF" || \
    error "$SYSTEM_REF not found. Build with: nex build asm/bootable-systemd-nex.yaml"

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
        [ -f "$p" ] && echo "$p" && return 0
    done
    return 1
}

OVMF_CODE=$(find_ovmf) || error "OVMF firmware not found. Install edk2-ovmf"

mkdir -p "$ROOT_DIR/build"
TMPDIR=$(mktemp -d "$ROOT_DIR/build/tmp.XXXXXX")
trap "rm -rf $TMPDIR" EXIT

# disk layout: 6GB (repo is ~3.5GB, need room for zub repo copy + deployment)
DISK_SIZE_MB=6144
ESP_SIZE_MB=64
ROOT_SIZE_MB=$((DISK_SIZE_MB - ESP_SIZE_MB - 1))

log "Creating ${DISK_SIZE_MB}MB disk image..."
dd if=/dev/zero of="$OUTPUT" bs=1M count=$DISK_SIZE_MB status=none

log "Creating GPT partition table..."
parted -s "$OUTPUT" \
    mklabel gpt \
    mkpart ESP fat32 1MiB ${ESP_SIZE_MB}MiB \
    set 1 esp on \
    mkpart root ext4 ${ESP_SIZE_MB}MiB 100%

# create ESP with bootloader
log "Creating ESP with bootloader..."
ESP_IMG="$TMPDIR/esp.img"
dd if=/dev/zero of="$ESP_IMG" bs=1M count=$((ESP_SIZE_MB - 1)) status=none
mkfs.vfat -F 32 "$ESP_IMG" >/dev/null
mmd -i "$ESP_IMG" ::/EFI
mmd -i "$ESP_IMG" ::/EFI/BOOT
mcopy -i "$ESP_IMG" "$BOOTLOADER" ::/EFI/BOOT/BOOTX64.EFI
dd if="$ESP_IMG" of="$OUTPUT" bs=1M seek=1 conv=notrunc status=none

# build root filesystem
log "Building root filesystem..."
ROOT_CONTENT="$TMPDIR/root"
mkdir -p "$ROOT_CONTENT"

# get the system's checksum for deployment path
SYSTEM_CHECKSUM=$($ZUB --repo="$REPO" show "$SYSTEM_REF" 2>/dev/null | grep "nex.system.checksum:" | awk '{print $2}')
if [ -z "$SYSTEM_CHECKSUM" ]; then
    # fallback: use commit hash
    SYSTEM_CHECKSUM=$($ZUB --repo="$REPO" rev-parse "$SYSTEM_REF")
fi

DEPLOY_PATH="nex/deploy/2nex/deploy/${SYSTEM_CHECKSUM}.0"
DEPLOY_DIR="$ROOT_CONTENT/$DEPLOY_PATH"

log "Extracting $SYSTEM_REF..."
mkdir -p "$(dirname "$DEPLOY_DIR")"
$ZUB --repo="$REPO" checkout "$SYSTEM_REF" "$DEPLOY_DIR"

mkdir -p "$ROOT_CONTENT/nex/deploy/2nex/var"

# copy zub repo to disk
log "Copying repo to disk (this may take a while)..."
cp -a "$REPO" "$ROOT_CONTENT/nex/repo"

# create root-level symlinks to the deployment
# these are needed because binaries have PT_INTERP=/lib64/ld-linux-x86-64.so.2
log "Creating root symlinks to deployment..."
ln -sf "$DEPLOY_PATH/usr" "$ROOT_CONTENT/usr"
ln -sf "$DEPLOY_PATH/lib" "$ROOT_CONTENT/lib"
ln -sf "$DEPLOY_PATH/lib64" "$ROOT_CONTENT/lib64"
ln -sf "$DEPLOY_PATH/bin" "$ROOT_CONTENT/bin"
ln -sf "$DEPLOY_PATH/sbin" "$ROOT_CONTENT/sbin"
ln -sf "$DEPLOY_PATH/etc" "$ROOT_CONTENT/etc"
ln -sf "usr/bin/init" "$ROOT_CONTENT/init"

# symlink /nex/pkg and /nex/db from deployment into the root /nex directory
# (we don't symlink /nex itself because /nex/repo and /nex/deploy are real directories)
ln -sfn "/$DEPLOY_PATH/nex/pkg" "$ROOT_CONTENT/nex/pkg"
ln -sfn "/$DEPLOY_PATH/nex/db" "$ROOT_CONTENT/nex/db"

# copy manifests git repo for user builds (enables git worktrees)
log "Copying manifests repo for user builds..."
if [ -d "$ROOT_DIR/pkg" ]; then
    mkdir -p "$ROOT_CONTENT/nex/manifests"
    # copy pkg/ contents as a git repo (need .git for worktrees)
    cp -a "$ROOT_DIR/pkg/." "$ROOT_CONTENT/nex/manifests/"
    # if parent is a git repo, initialize manifests as a proper git repo
    if [ -d "$ROOT_DIR/.git" ]; then
        # create a standalone repo from the pkg/ subtree
        (cd "$ROOT_CONTENT/nex/manifests" && \
         git init -q && \
         git add -A && \
         git commit -q -m "Initial manifests" 2>/dev/null || true)
    fi
fi

# create /nex/users directory with sticky bit for user environments
log "Creating /nex/users directory..."
mkdir -p "$ROOT_CONTENT/nex/users"
chmod 1777 "$ROOT_CONTENT/nex/users"

log "Deployment structure:"
ls -la "$DEPLOY_DIR/" | head -15
echo ""
log "Packages installed:"
ls "$DEPLOY_DIR/nex/pkg/" 2>/dev/null | head -10 || echo "(none)"

# create ext4 filesystem
log "Creating ext4 filesystem..."
ROOT_IMG="$TMPDIR/root.img"
# -i 4096 = one inode per 4KB (vs default ~16KB) - needed for zub's many small object files
mke2fs -t ext4 -d "$ROOT_CONTENT" -i 4096 "$ROOT_IMG" ${ROOT_SIZE_MB}M
dd if="$ROOT_IMG" of="$OUTPUT" bs=1M seek=$ESP_SIZE_MB conv=notrunc status=none

log "Disk image created: $OUTPUT"
log "Starting QEMU..."
echo ""

OVMF_VARS_TEMPLATE="${OVMF_CODE/OVMF_CODE/OVMF_VARS}"
OVMF_VARS="$TMPDIR/OVMF_VARS.fd"
[ -f "$OVMF_VARS_TEMPLATE" ] && cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS"

QEMU_ARGS=(
    -enable-kvm
    -machine q35
    -cpu host
    -m 8G
    -smp $(nproc)
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
)
[ -f "$OVMF_VARS" ] && QEMU_ARGS+=(-drive "if=pflash,format=raw,file=$OVMF_VARS")
QEMU_ARGS+=(
    -drive "file=$OUTPUT,format=raw,if=virtio"
    -netdev user,id=net0,hostfwd=tcp::10022-:22
    -device virtio-net-pci,netdev=net0
    -serial mon:stdio
    -display none
    -no-reboot
)

log "Network: SSH available on localhost:10022 (ssh -p 10022 root@localhost)"

echo "--- QEMU output (Ctrl-A X to exit) ---"
echo ""
qemu-system-x86_64 "${QEMU_ARGS[@]}"
echo ""
log "QEMU exited"
