#!/bin/sh
# EFI boot test with Wayland desktop and GL acceleration
#
# usage: ./qemu-desktop.sh [system-ref]
#   system-ref: zub ref to boot (default: systems/desktop-vwl/0.0.1)
set -eux

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
REPO="$ROOT_DIR/.nex/repo"
BOOTLOADER="$ROOT_DIR/src/bootloader/target/x86_64-unknown-uefi/debug/nex-bootloader.efi"
OUTPUT="$ROOT_DIR/build/desktop-disk.img"
SYSTEM_REF="${1:-systems/desktop-vwl/0.0.1}"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

log() { printf "${GREEN}===${NC} %s\n" "$1"; }
error() { printf "${RED}ERROR:${NC} %s\n" "$1"; exit 1; }

log "Building bootloader..."
(cd "$ROOT_DIR/src/bootloader" && cargo build) || error "bootloader build failed"
command -v mke2fs >/dev/null || error "mke2fs not found (install e2fsprogs)"
command -v mcopy >/dev/null || error "mcopy not found (install mtools)"
command -v qemu-system-x86_64 >/dev/null || error "qemu-system-x86_64 not found"
command -v zub >/dev/null || error "zub not found"

{ zub --repo="$REPO" refs 2>/dev/null || true; } | grep -q "$SYSTEM_REF" || \
    error "$SYSTEM_REF not found. Build with: nex build asm/bootable/desktop-vwl.yaml"

find_ovmf() {
    for p in \
        "/usr/share/edk2/x64/OVMF_CODE.4m.fd" \
        "/usr/share/edk2/x64/OVMF_CODE.fd" \
        "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd" \
        "/usr/share/OVMF/OVMF_CODE.fd" \
        "/usr/share/qemu/OVMF.fd"
    do
        [ -f "$p" ] && echo "$p" && return 0
    done
    return 1
}

OVMF_CODE=$(find_ovmf) || error "OVMF firmware not found. Install edk2-ovmf"

mkdir -p "$ROOT_DIR/build"
TMPDIR=$(mktemp -d "$ROOT_DIR/build/tmp.XXXXXX")
trap "rm -rf $TMPDIR" EXIT

DISK_SIZE_MB=4096
ESP_SIZE_MB=64
ROOT_SIZE_MB=2048
VAR_SIZE_MB=$((DISK_SIZE_MB - ESP_SIZE_MB - ROOT_SIZE_MB - 1))

log "Creating ${DISK_SIZE_MB}MB disk image..."
dd if=/dev/zero of="$OUTPUT" bs=1MiB count=$DISK_SIZE_MB status=none

log "Creating GPT partition table (3 partitions: ESP, nex, nex-var)..."
parted -s "$OUTPUT" \
    mklabel gpt \
    mkpart ESP fat32 1MiB ${ESP_SIZE_MB}MiB \
    set 1 esp on \
    mkpart nex ext4 ${ESP_SIZE_MB}MiB $((ESP_SIZE_MB + ROOT_SIZE_MB))MiB \
    mkpart nex-var ext4 $((ESP_SIZE_MB + ROOT_SIZE_MB))MiB 100%

log "Creating ESP with bootloader..."
ESP_IMG="$TMPDIR/esp.img"
dd if=/dev/zero of="$ESP_IMG" bs=1MiB count=$((ESP_SIZE_MB - 1)) status=none
mkfs.vfat -F 32 "$ESP_IMG" >/dev/null
mmd -i "$ESP_IMG" ::/EFI
mmd -i "$ESP_IMG" ::/EFI/BOOT
mcopy -i "$ESP_IMG" "$BOOTLOADER" ::/EFI/BOOT/BOOTX64.EFI
dd if="$ESP_IMG" of="$OUTPUT" bs=1MiB seek=1 conv=notrunc status=none

log "Building root filesystem..."
ROOT_CONTENT="$TMPDIR/root"
mkdir -p "$ROOT_CONTENT"

SYSTEM_CHECKSUM=$(zub --repo="$REPO" show "$SYSTEM_REF" 2>/dev/null | grep "nex.system.checksum:" | awk '{print $2}')
if [ -z "$SYSTEM_CHECKSUM" ]; then
    SYSTEM_CHECKSUM=$(zub --repo="$REPO" rev-parse "$SYSTEM_REF")
fi

DEPLOY_PATH="nex/deployments/${SYSTEM_CHECKSUM}.0"
DEPLOY_DIR="$ROOT_CONTENT/$DEPLOY_PATH"

log "Extracting $SYSTEM_REF..."
mkdir -p "$(dirname "$DEPLOY_DIR")"
zub --repo="$REPO" checkout "$SYSTEM_REF" "$DEPLOY_DIR"

# create /nex/current symlink to active deployment
ln -sfn "deployments/${SYSTEM_CHECKSUM}.0" "$ROOT_CONTENT/nex/current"

log "Initializing repo with remote (SSH to host)..."
rm -rf "$ROOT_CONTENT/nex/store"
zub init "$ROOT_CONTENT/nex/store"

HOST_USER="$USER"
HOST_REPO_PATH="$REPO"
cat > "$ROOT_CONTENT/nex/store/config.toml" << EOF
[namespace]
uid_map = []
gid_map = []

[[remotes]]
name = "origin"
url = "ssh://${HOST_USER}@10.0.2.2${HOST_REPO_PATH}"
EOF

log "Remote configured: ${HOST_USER}@10.0.2.2:${HOST_REPO_PATH}"

log "Preparing SSH keys for var partition..."
SSH_KEYS="$TMPDIR/ssh_keys"
mkdir -p "$SSH_KEYS"
chmod 700 "$SSH_KEYS"
if [ -f "$HOME/.ssh/id_ed25519" ]; then
    cp "$HOME/.ssh/id_ed25519" "$SSH_KEYS/"
    cp "$HOME/.ssh/id_ed25519.pub" "$SSH_KEYS/"
elif [ -f "$HOME/.ssh/id_rsa" ]; then
    cp "$HOME/.ssh/id_rsa" "$SSH_KEYS/"
    cp "$HOME/.ssh/id_rsa.pub" "$SSH_KEYS/"
else
    error "No SSH key found in ~/.ssh (need id_ed25519 or id_rsa)"
fi
chmod 600 "$SSH_KEYS/"id_*

cat > "$SSH_KEYS/config" << 'EOF'
Host 10.0.2.2
    StrictHostKeyChecking accept-new
    UserKnownHostsFile /dev/null
    LogLevel ERROR
EOF
chmod 600 "$SSH_KEYS/config"

log "Creating root symlinks..."
# readonly symlinks to deployment
ln -sf "$DEPLOY_PATH/usr" "$ROOT_CONTENT/usr"
ln -sf /usr/lib "$ROOT_CONTENT/lib"
ln -sf "$DEPLOY_PATH/lib64" "$ROOT_CONTENT/lib64"
ln -sf /usr/bin "$ROOT_CONTENT/bin"
ln -sf /usr/bin "$ROOT_CONTENT/sbin"
ln -sf "usr/bin/init" "$ROOT_CONTENT/init"

# writable symlinks to /var (will be separate partition)
ln -sf "var/etc" "$ROOT_CONTENT/etc"
ln -sf "var/home" "$ROOT_CONTENT/home"
ln -sf "var/home/root" "$ROOT_CONTENT/root"

# /var mount point for nex-var partition
mkdir -p "$ROOT_CONTENT/var"

# mount points for virtual filesystems (root is readonly, can't create at boot)
mkdir -p "$ROOT_CONTENT/proc"
mkdir -p "$ROOT_CONTENT/sys"
mkdir -p "$ROOT_CONTENT/dev"
mkdir -p "$ROOT_CONTENT/run"
mkdir -p "$ROOT_CONTENT/tmp"

# symlink /nex/pkg, /nex/db, /nex/env from current deployment
ln -sfn "current/nex/pkg" "$ROOT_CONTENT/nex/pkg"
ln -sfn "current/nex/db" "$ROOT_CONTENT/nex/db"
ln -sfn "current/nex/env" "$ROOT_CONTENT/nex/env"

log "Creating /nex/users directory..."
mkdir -p "$ROOT_CONTENT/nex/users"
chmod 1777 "$ROOT_CONTENT/nex/users"

log "Deployment structure:"
ls -la "$DEPLOY_DIR/" | head -15
echo ""
log "Packages installed:"
ls "$DEPLOY_DIR/nex/pkg/" 2>/dev/null | head -10 || echo "(none)"

log "Creating ext4 filesystem..."
ROOT_IMG="$TMPDIR/root.img"
if command -v fakeroot >/dev/null; then
    fakeroot -- sh -c "chown -R 0:0 '$ROOT_CONTENT' && mke2fs -t ext4 -d '$ROOT_CONTENT' '$ROOT_IMG' ${ROOT_SIZE_MB}M"
else
    log "WARNING: fakeroot not found - SSH keys may have wrong ownership in VM"
    mke2fs -t ext4 -d "$ROOT_CONTENT" "$ROOT_IMG" ${ROOT_SIZE_MB}M
fi
dd if="$ROOT_IMG" of="$OUTPUT" bs=1MiB seek=$ESP_SIZE_MB conv=notrunc status=none

log "Building var filesystem..."
VAR_CONTENT="$TMPDIR/var"
mkdir -p "$VAR_CONTENT"/{etc,home,log,lib,cache,tmp}
chmod 1777 "$VAR_CONTENT/tmp"
mkdir -p "$VAR_CONTENT/log/journal"
mkdir -p "$VAR_CONTENT/lib/systemd"/{random-seed,timers,coredump}
mkdir -p "$VAR_CONTENT/lib/sshd"
chmod 700 "$VAR_CONTENT/lib/sshd"
mkdir -p "$VAR_CONTENT/cache/fontconfig"
ln -sf /run "$VAR_CONTENT/run"

# copy /etc from deployment (first-boot initialization)
log "Copying /etc from deployment to var..."
cp -a "$DEPLOY_DIR/etc/." "$VAR_CONTENT/etc/"
touch "$VAR_CONTENT/etc/.initialized"

# copy root's ssh keys to var/home (writable location)
mkdir -p "$VAR_CONTENT/home/root/.ssh"
chmod 700 "$VAR_CONTENT/home/root/.ssh"
cp -a "$SSH_KEYS/." "$VAR_CONTENT/home/root/.ssh/"

log "Creating var ext4 filesystem..."
VAR_IMG="$TMPDIR/var.img"
if command -v fakeroot >/dev/null; then
    fakeroot -- sh -c "chown -R 0:0 '$VAR_CONTENT' && mke2fs -t ext4 -L nex-var -d '$VAR_CONTENT' '$VAR_IMG' ${VAR_SIZE_MB}M"
else
    mke2fs -t ext4 -L nex-var -d "$VAR_CONTENT" "$VAR_IMG" ${VAR_SIZE_MB}M
fi
dd if="$VAR_IMG" of="$OUTPUT" bs=1MiB seek=$((ESP_SIZE_MB + ROOT_SIZE_MB)) conv=notrunc status=none

log "Disk image created: $OUTPUT"
log "Starting QEMU with GL-accelerated display..."
echo ""

OVMF_VARS_TEMPLATE="${OVMF_CODE%OVMF_CODE*}OVMF_VARS${OVMF_CODE#*OVMF_CODE}"
OVMF_VARS="$TMPDIR/OVMF_VARS.fd"
[ -f "$OVMF_VARS_TEMPLATE" ] && cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS"

log "Network: SSH available on localhost:10022 (ssh -p 10022 root@localhost)"
log "Display: GTK window with virtio-vga-gl (GL acceleration)"
log "Remote fetch: VM will SSH to ${HOST_USER}@10.0.2.2 for artifacts"

echo "--- QEMU output (close window or Ctrl-C to exit) ---"
echo ""

OVMF_VARS_ARG=""
[ -f "$OVMF_VARS" ] && OVMF_VARS_ARG="-drive if=pflash,format=raw,file=$OVMF_VARS"

qemu-system-x86_64 \
    -enable-kvm \
    -machine q35 \
    -cpu host \
    -m 8G \
    -smp $(nproc) \
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
    $OVMF_VARS_ARG \
    -drive "file=$OUTPUT,format=raw,if=virtio" \
    -device virtio-vga-gl \
    -display gtk,gl=on \
    -netdev user,id=net0,hostfwd=tcp::10022-:22 \
    -device virtio-net-pci,netdev=net0 \
    -device virtio-keyboard-pci \
    -device virtio-tablet-pci \
    -serial mon:stdio \
    -no-reboot

echo ""
log "QEMU exited"
