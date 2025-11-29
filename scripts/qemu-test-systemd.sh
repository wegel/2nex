#!/bin/bash
# test system by extracting kernel and rootfs from OSTree (flat mode, no EFI)
# usage: ./qemu-test-systemd.sh [system-ref]
#   examples:
#     ./qemu-test-systemd.sh
#     ./qemu-test-systemd.sh systems/bootable-minimal/0.0.1
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
REPO="$ROOT_DIR/.nex/repo"
KERNEL_REF="x86_64/pkg/core/kernel/linux/6.12.58/outputs/boot"
SYSTEM_REF="${1:-systems/bootable-systemd/0.0.1}"

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

echo "=== Extracting kernel from OSTree ==="
unshare --map-root-user ostree --repo=$REPO checkout --union $KERNEL_REF $TMPDIR/kernel
KERNEL_PATH=$(find $TMPDIR/kernel -name 'vmlinuz*' -o -name 'bzImage' | head -1)

if [ ! -f "$KERNEL_PATH" ]; then
    echo "ERROR: Kernel not found in $KERNEL_REF"
    echo "Contents of boot output:"
    find $TMPDIR/kernel -type f
    exit 1
fi

echo "Found kernel: $KERNEL_PATH"

echo "=== Extracting rootfs from OSTree ==="
unshare --map-root-user ostree --repo=$REPO checkout --union $SYSTEM_REF $TMPDIR/rootfs

echo "=== Creating initramfs ==="
# kernel has embedded initramfs with /sbin as directory (busybox)
# our rootfs has /sbin -> /usr/bin symlink which can't replace the directory
# solution: if /sbin is a symlink, convert it to a directory with symlinks to /usr/bin/*
if [ -L "$TMPDIR/rootfs/sbin" ]; then
    rm "$TMPDIR/rootfs/sbin"
    mkdir "$TMPDIR/rootfs/sbin"
    # create symlinks for all binaries in /usr/bin
    for bin in "$TMPDIR/rootfs/usr/bin/"*; do
        name=$(basename "$bin")
        ln -sf "/usr/bin/$name" "$TMPDIR/rootfs/sbin/$name"
    done
fi
# same for /bin
if [ -L "$TMPDIR/rootfs/bin" ]; then
    rm "$TMPDIR/rootfs/bin"
    mkdir "$TMPDIR/rootfs/bin"
    for bin in "$TMPDIR/rootfs/usr/bin/"*; do
        name=$(basename "$bin")
        ln -sf "/usr/bin/$name" "$TMPDIR/rootfs/bin/$name"
    done
fi
( cd $TMPDIR/rootfs && find . | cpio -o -H newc --owner=0:0 | gzip -1 > $TMPDIR/initramfs.img )

echo "Rootfs contents:"
ls -lh $TMPDIR/rootfs/ | head -20

echo ""
echo "=== Booting with QEMU (systemd init) ==="
qemu-system-x86_64 \
  -enable-kvm \
  -machine type=q35,accel=kvm \
  -cpu host,-hypervisor \
  -smp 4 \
  -m 2G \
  -kernel "$KERNEL_PATH" \
  -initrd "$TMPDIR/initramfs.img" \
  -append "console=tty0 console=ttyS0 loglevel=7 init=/init" \
  -serial mon:stdio \
  -display none
