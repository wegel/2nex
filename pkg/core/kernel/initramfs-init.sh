#!/bin/sh
# nex initramfs init
export PATH=/bin:/sbin

mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
if [ ! -c /dev/console ]; then
    mknod -m 600 /dev/console c 5 1
fi
if [ ! -c /dev/null ]; then
    mknod -m 666 /dev/null c 1 3
fi
if [ ! -c /dev/tty0 ]; then
    mknod -m 600 /dev/tty0 c 4 0
fi
if [ ! -c /dev/ttyS0 ]; then
    mknod -m 600 /dev/ttyS0 c 4 64
fi

# set up a working console for init output
CMDLINE="$(cat /proc/cmdline)"
CONSOLE=""
if echo "$CMDLINE" | grep -q "console=tty0" && [ -c /dev/tty0 ]; then
    CONSOLE=/dev/tty0
elif echo "$CMDLINE" | grep -q "console=ttyS0" && [ -c /dev/ttyS0 ]; then
    CONSOLE=/dev/ttyS0
elif [ -c /dev/console ]; then
    CONSOLE=/dev/console
fi
if [ -n "$CONSOLE" ]; then
    exec <"$CONSOLE" >"$CONSOLE" 2>&1
elif [ -c /dev/kmsg ]; then
    exec >/dev/kmsg 2>&1
fi

echo "nex initramfs starting... (v5)"

# parse cmdline
ROOT=""
ROOT_PARTUUID=""
DEPLOY_PATH=""
for param in $(cat /proc/cmdline); do
    case "$param" in
        root=PARTUUID=*) ROOT_PARTUUID="${param#root=PARTUUID=}" ;;
        root=/dev/*) ROOT="${param#root=}" ;;
        zub=*) DEPLOY_PATH="${param#zub=}" ;;
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

echo "root=$ROOT zub=$DEPLOY_PATH"

if [ -z "$ROOT" ] || [ ! -b "$ROOT" ]; then
    echo "FATAL: root device not found!"
    echo "PARTUUID=$ROOT_PARTUUID"
    echo "block devices:"
    ls /dev/vd* /dev/sd* /dev/nvme* 2>/dev/null || echo "(none)"
    exec /bin/sh
fi

echo "mounting root filesystem..."
mount -o ro "$ROOT" /mnt/root

if [ -z "$DEPLOY_PATH" ]; then
    echo "FATAL: no zub= parameter!"
    exec /bin/sh
fi

DEPLOY="/mnt/root/$DEPLOY_PATH"
if [ ! -d "$DEPLOY" ]; then
    echo "FATAL: deployment $DEPLOY not found!"
    exec /bin/sh
fi

echo "switching to deployment: $DEPLOY"

# find and mount var partition by label (root stays readonly)
echo "looking for var partition..."
VAR_DEV=""
i=0
while [ $i -lt 30 ] && [ -z "$VAR_DEV" ]; do
    VAR_DEV=$(blkid -L nex-var -o device 2>/dev/null)
    [ -n "$VAR_DEV" ] && break
    sleep 0.1
    i=$((i + 1))
done
if [ -n "$VAR_DEV" ]; then
    echo "found var partition: $VAR_DEV"
    mkdir -p /mnt/root/var
    mount "$VAR_DEV" /mnt/root/var

    # first-boot: populate /var/etc from deployment
    if [ ! -f /mnt/root/var/etc/.initialized ]; then
        echo "first boot: copying /etc from deployment..."
        mkdir -p /mnt/root/var/etc
        cp -a "$DEPLOY/etc/." /mnt/root/var/etc/
        touch /mnt/root/var/etc/.initialized
        echo "/var/etc initialized"
    fi

    # create standard /var directories if missing
    mkdir -p /mnt/root/var/home
    mkdir -p /mnt/root/var/log
    mkdir -p /mnt/root/var/lib
    mkdir -p /mnt/root/var/cache
    mkdir -p /mnt/root/var/tmp
    chmod 1777 /mnt/root/var/tmp
else
    echo "WARNING: var partition not found, remounting root rw"
    mount -o remount,rw /mnt/root
fi

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

# sysfs mountpoints are managed by systemd; don't try to create them in sysfs

# ensure /tmp mountpoint exists (root may be readonly)
if [ ! -d /mnt/root/tmp ]; then
    mkdir -p /mnt/root/tmp 2>/dev/null || true
fi

echo "mount points created"

# use /init which symlinks through the deployment
exec switch_root /mnt/root /init
exec /bin/sh
