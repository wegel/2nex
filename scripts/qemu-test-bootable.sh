#!/bin/bash
# test bootable-minimal system by extracting kernel and rootfs from OSTree
set -eu

REPO="bootstrap_store"
KERNEL_REF="x86_64/linux/6.12.58/sys/kernel/outputs/boot"
SYSTEM_REF="asm/bootable-minimal/0.0.1"

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
( cd $TMPDIR/rootfs && find . | cpio -o -H newc --owner=0:0 | gzip -9 > $TMPDIR/initramfs.img )

echo "Rootfs contents:"
ls -lh $TMPDIR/rootfs/ | head -20

echo ""
echo "=== Booting with QEMU ==="
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
