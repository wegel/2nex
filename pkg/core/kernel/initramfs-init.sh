#!/bin/sh
# 2nex initramfs init
export PATH=/bin:/sbin

echo "2nex initramfs starting... (v3)"

mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev

# parse cmdline
ROOT=""
ROOT_PARTUUID=""
OSTREE=""
for param in $(cat /proc/cmdline); do
    case "$param" in
        root=PARTUUID=*) ROOT_PARTUUID="${param#root=PARTUUID=}" ;;
        root=/dev/*) ROOT="${param#root=}" ;;
        ostree=*) OSTREE="${param#ostree=}" ;;
    esac
done

# use blkid to resolve PARTUUID to device path
if [ -n "$ROOT_PARTUUID" ]; then
    echo "looking for root partition PARTUUID=$ROOT_PARTUUID..."
    i=0
    while [ $i -lt 30 ] && [ -z "$ROOT" ]; do
        ROOT=$(blkid -t PARTUUID="$ROOT_PARTUUID" -o device 2>/dev/null)
        [ -n "$ROOT" ] && break
        sleep 0.1
        i=$((i + 1))
    done
fi

echo "root=$ROOT ostree=$OSTREE"

if [ -z "$ROOT" ] || [ ! -b "$ROOT" ]; then
    echo "FATAL: root device not found!"
    echo "PARTUUID=$ROOT_PARTUUID"
    echo "block devices:"
    ls /dev/vd* /dev/sd* /dev/nvme* 2>/dev/null || echo "(none)"
    exec /bin/sh
fi

echo "mounting root filesystem..."
mount -o ro "$ROOT" /mnt/root

if [ -z "$OSTREE" ]; then
    echo "FATAL: no ostree= parameter!"
    exec /bin/sh
fi

DEPLOY="/mnt/root/$OSTREE"
if [ ! -d "$DEPLOY" ]; then
    echo "FATAL: deployment $DEPLOY not found!"
    exec /bin/sh
fi

echo "switching to deployment: $DEPLOY"
mount -o remount,rw /mnt/root

# move virtual filesystems to new root so systemd finds them
mkdir -p /mnt/root/proc /mnt/root/sys /mnt/root/dev /mnt/root/run
mount --move /proc /mnt/root/proc
mount --move /sys /mnt/root/sys
mount --move /dev /mnt/root/dev

# mount tmpfs on /run - systemd requires this to be a tmpfs
mount -t tmpfs -o mode=755,nosuid,nodev tmpfs /mnt/root/run
mkdir -p /mnt/root/run/systemd

# create subdirectories on devtmpfs for systemd mount units
echo "creating mount point directories..."
mkdir -p /mnt/root/dev/hugepages
mkdir -p /mnt/root/dev/mqueue
mkdir -p /mnt/root/dev/shm
mkdir -p /mnt/root/dev/pts

# create sysfs subdirectories for debugfs/tracingfs
mkdir -p /mnt/root/sys/kernel/debug
mkdir -p /mnt/root/sys/kernel/tracing
mkdir -p /mnt/root/sys/kernel/config
mkdir -p /mnt/root/sys/fs/cgroup

# ensure /tmp is empty for tmpfs mount
rm -rf /mnt/root/tmp
mkdir -p /mnt/root/tmp
chmod 1777 /mnt/root/tmp

echo "mount points created (v5)"

# test if we can mount tmpfs on /tmp from busybox
echo "testing busybox mount tmpfs..."
mount -t tmpfs tmpfs /mnt/root/tmp && echo "tmpfs mount OK" || echo "tmpfs mount FAILED"
umount /mnt/root/tmp 2>/dev/null
mkdir -p /mnt/root/tmp
chmod 1777 /mnt/root/tmp

# check available filesystems
echo "supported filesystems:"
cat /proc/filesystems | grep -E "hugetlbfs|mqueue|debugfs|tracingfs|tmpfs"

# use /init which symlinks through the deployment
exec switch_root /mnt/root /init
exec /bin/sh
