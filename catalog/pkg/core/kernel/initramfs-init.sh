#!/bin/sh
# nex initramfs init
export PATH=/bin:/sbin

log() { echo "$@"; }
die() { log "FATAL: $*"; exec /bin/sh; }

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
if echo "$CMDLINE" | grep -q "installer.console=ttyS0" && [ -c /dev/ttyS0 ]; then
    CONSOLE=/dev/ttyS0
elif echo "$CMDLINE" | grep -q "installer.console=tty0" && [ -c /dev/tty0 ]; then
    CONSOLE=/dev/tty0
elif echo "$CMDLINE" | grep -q "console=ttyS0" && [ -c /dev/ttyS0 ]; then
    CONSOLE=/dev/ttyS0
elif echo "$CMDLINE" | grep -q "console=tty0" && [ -c /dev/tty0 ]; then
    CONSOLE=/dev/tty0
elif [ -c /dev/console ]; then
    CONSOLE=/dev/console
fi
if [ -n "$CONSOLE" ]; then
    exec <"$CONSOLE" >"$CONSOLE" 2>&1
elif [ -c /dev/kmsg ]; then
    exec >/dev/kmsg 2>&1
fi

log "nex initramfs starting... (v7)"

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
    log "looking for root partition PARTUUID=$ROOT_PARTUUID..."
    i=0
    while [ $i -lt 30 ] && [ -z "$ROOT" ]; do
        ROOT=$(blkid -t PARTUUID="$ROOT_PARTUUID" -o device 2>/dev/null)
        [ -n "$ROOT" ] && break
        sleep 0.1
        i=$((i + 1))
    done
fi

log "root=$ROOT zub=$DEPLOY_PATH"

if [ -z "$ROOT" ] || [ ! -b "$ROOT" ]; then
    log "FATAL: root device not found!"
    log "PARTUUID=$ROOT_PARTUUID"
    log "block devices:"
    ls /dev/vd* /dev/sd* /dev/nvme* 2>/dev/null || log "(none)"
    exec /bin/sh
fi

SYSROOT_MOUNT=/mnt/sysroot
SYSROOT_RW=0

ensure_sysroot_rw() {
    if [ "$SYSROOT_RW" -eq 1 ]; then
        return 0
    fi
    mount -o remount,rw "$SYSROOT_MOUNT" || return 1
    SYSROOT_RW=1
    return 0
}

mkdirp_deploy() {
    dir="$1"
    if [ -d "$dir" ]; then
        return 0
    fi
    mkdir -p "$dir" 2>/dev/null && return 0
    ensure_sysroot_rw || die "failed to remount sysroot rw (needed to create $dir)"
    mkdir -p "$dir" || die "failed to create $dir"
}

log "mounting sysroot filesystem..."
mkdir -p "$SYSROOT_MOUNT" || die "failed to create $SYSROOT_MOUNT"
mount -o rw "$ROOT" "$SYSROOT_MOUNT" || die "failed to mount sysroot $ROOT"
SYSROOT_RW=1

if [ -z "$DEPLOY_PATH" ]; then
    die "no zub= parameter!"
fi

case "$DEPLOY_PATH" in
    /*) DEPLOY="${SYSROOT_MOUNT}${DEPLOY_PATH}" ;;
    *) DEPLOY="${SYSROOT_MOUNT}/${DEPLOY_PATH}" ;;
esac
if [ ! -d "$DEPLOY" ]; then
    die "deployment $DEPLOY not found!"
fi

log "selected deployment: $DEPLOY"

# find and mount var partition by label (root stays readonly)
log "looking for var partition..."
VAR_DEV=""
ensure_var_dirs() {
    mkdir -p \
        "${SYSROOT_MOUNT}/var/etc" \
        "${SYSROOT_MOUNT}/var/home" \
        "${SYSROOT_MOUNT}/var/root" \
        "${SYSROOT_MOUNT}/var/log" \
        "${SYSROOT_MOUNT}/var/lib" \
        "${SYSROOT_MOUNT}/var/cache" \
        "${SYSROOT_MOUNT}/var/tmp" \
        "${SYSROOT_MOUNT}/var/nex/manifests" \
        "${SYSROOT_MOUNT}/var/nex/users" \
        || die "failed to create standard /var directories"
    chmod 1777 "${SYSROOT_MOUNT}/var/tmp" 2>/dev/null || true
}

# Print the directory holding the machine's content store.
#
# The store sits on the persistent root or on /var and is bind-mounted into
# the deployment, so a machine installed before the store was renamed still
# keeps its objects under the older "repo" name. Both names are searched, and
# a directory that holds an actual store always wins over one that is merely
# present: a built deployment carries empty mount points, and binding an empty
# one would hide every object the machine owns.
store_bind_source() {
    for candidate in \
        "${SYSROOT_MOUNT}/nex/store" \
        "${SYSROOT_MOUNT}/var/nex/store" \
        "${SYSROOT_MOUNT}/nex/repo" \
        "${SYSROOT_MOUNT}/var/nex/repo"
    do
        if [ -f "${candidate}/config.toml" ] || [ -d "${candidate}/objects" ]; then
            echo "$candidate"
            return 0
        fi
    done

    # No store anywhere. An installer that put a directory on the root
    # partition meant that directory to hold one.
    if [ -d "${SYSROOT_MOUNT}/nex/store" ]; then
        echo "${SYSROOT_MOUNT}/nex/store"
        return 0
    fi
    if [ -d "${SYSROOT_MOUNT}/nex/repo" ]; then
        echo "${SYSROOT_MOUNT}/nex/repo"
        return 0
    fi

    mkdir -p "${SYSROOT_MOUNT}/var/nex/store" || die "failed to create /var/nex/store"
    echo "${SYSROOT_MOUNT}/var/nex/store"
}

if [ -n "$ROOT" ]; then
    case "$ROOT" in
        /dev/nvme*n*p2) VAR_DEV="${ROOT%p2}p3" ;;
        /dev/*2) VAR_DEV="${ROOT%2}3" ;;
        *) VAR_DEV="" ;;
    esac
fi

if [ -n "$VAR_DEV" ] && [ -b "$VAR_DEV" ]; then
    var_label=$(blkid -o value -s LABEL "$VAR_DEV" 2>/dev/null || true)
    if [ "$var_label" != "nex-var" ]; then
        VAR_DEV=""
    fi
else
    VAR_DEV=""
fi

if [ -n "$VAR_DEV" ]; then
    log "found var partition: $VAR_DEV"
    mkdirp_deploy "${SYSROOT_MOUNT}/var"
    mount "$VAR_DEV" "${SYSROOT_MOUNT}/var" || die "failed to mount var partition"
    ensure_var_dirs

else
    log "WARNING: var partition not found on root device; using sysroot /var and remounting sysroot rw"
    mkdir -p "${SYSROOT_MOUNT}/var" 2>/dev/null || true
    ensure_sysroot_rw || die "failed to remount sysroot rw for missing var partition"
    ensure_var_dirs
fi

# Seed only paths that the host does not already own. New deployments keep
# legacy defaults in the UAPI factory tree. The fallback supports deployments
# created before Nex adopted that layout.
FACTORY_ETC="${DEPLOY}/usr/share/factory/etc"
if [ ! -d "${FACTORY_ETC}" ]; then
    FACTORY_ETC="${DEPLOY}/etc"
fi
log "populating missing /etc paths from ${FACTORY_ETC#${SYSROOT_MOUNT}}..."
/bin/nex-populate-etc "${FACTORY_ETC}" "${SYSROOT_MOUNT}/var/etc" \
    || die "failed to populate missing /etc paths"

# Ensure machine-id exists and is writable for systemd.
if [ ! -f "${SYSROOT_MOUNT}/var/etc/machine-id" ]; then
    : > "${SYSROOT_MOUNT}/var/etc/machine-id" 2>/dev/null || true
fi

# Ensure deployment mount points exist (may require temporarily making sysroot writable)
mkdirp_deploy "${DEPLOY}/sysroot"
mkdirp_deploy "${DEPLOY}/var"
mkdirp_deploy "${DEPLOY}/etc"
mkdirp_deploy "${DEPLOY}/home"
mkdirp_deploy "${DEPLOY}/root"
mkdirp_deploy "${DEPLOY}/nex/store"
mkdirp_deploy "${DEPLOY}/nex/deployments"
mkdirp_deploy "${DEPLOY}/nex/staging"
mkdirp_deploy "${DEPLOY}/nex/users"
mkdirp_deploy "${DEPLOY}/nex/manifests"
mkdirp_deploy "${DEPLOY}/proc"
mkdirp_deploy "${DEPLOY}/sys"
mkdirp_deploy "${DEPLOY}/dev"
mkdirp_deploy "${DEPLOY}/run"
mkdirp_deploy "${DEPLOY}/tmp"

# busybox switch_root requires NEW_ROOT to be a mount point; make the deployment
# root a mount point before adding bind mounts under it.
mount --bind "$DEPLOY" "$DEPLOY" || die "failed to make deployment root a mount point"
mount -o remount,ro,bind "$DEPLOY" || die "failed to remount deployment root read-only"

# Bind mounts for ostree-like layout
mount --bind "$SYSROOT_MOUNT" "${DEPLOY}/sysroot" || die "failed to bind-mount sysroot"
mount --bind "${SYSROOT_MOUNT}/var" "${DEPLOY}/var" || die "failed to bind-mount /var"
mount --bind "${SYSROOT_MOUNT}/var/etc" "${DEPLOY}/etc" || die "failed to bind-mount /etc"
mount --bind "${SYSROOT_MOUNT}/var/home" "${DEPLOY}/home" || die "failed to bind-mount /home"
mount --bind "${SYSROOT_MOUNT}/var/root" "${DEPLOY}/root" || die "failed to bind-mount /root"

STORE_SOURCE="$(store_bind_source)"
mount --bind "$STORE_SOURCE" "${DEPLOY}/nex/store" || die "failed to bind-mount /nex/store"
mount --bind "${SYSROOT_MOUNT}/nex/deployments" "${DEPLOY}/nex/deployments" || die "failed to bind-mount /nex/deployments"
mount --bind "${SYSROOT_MOUNT}/nex/staging" "${DEPLOY}/nex/staging" || die "failed to bind-mount /nex/staging"
mkdir -p "${SYSROOT_MOUNT}/var/nex/users" 2>/dev/null || true
mount --bind "${SYSROOT_MOUNT}/var/nex/users" "${DEPLOY}/nex/users" || die "failed to bind-mount /nex/users"

# keep manifests repo on /var (writable), and bind it into the deployment at /nex/manifests
mkdir -p "${SYSROOT_MOUNT}/var/nex/manifests" 2>/dev/null || true
mount --bind "${SYSROOT_MOUNT}/var/nex/manifests" "${DEPLOY}/nex/manifests" || die "failed to bind-mount /nex/manifests"

mount -o remount,ro,bind "${DEPLOY}/sysroot" || die "failed to remount /sysroot read-only"
mount -o remount,rw,bind "${DEPLOY}/nex/store" || die "failed to remount /nex/store writable"
mount -o remount,rw,bind "${DEPLOY}/nex/staging" || die "failed to remount /nex/staging writable"

# Move virtual filesystems into deployment root for systemd
mount --move /proc "${DEPLOY}/proc" || die "failed to move /proc"
mount --move /sys "${DEPLOY}/sys" || die "failed to move /sys"
mount --move /dev "${DEPLOY}/dev" || die "failed to move /dev"

# /run must be tmpfs for systemd
mount -t tmpfs -o mode=755,nosuid,nodev tmpfs "${DEPLOY}/run" || die "failed to mount tmpfs on /run"
mkdir -p "${DEPLOY}/run/systemd" 2>/dev/null || true

# create subdirectories on devtmpfs for systemd mount units
log "creating mount point directories..."
mkdir -p "${DEPLOY}/dev/hugepages" 2>/dev/null || true
mkdir -p "${DEPLOY}/dev/mqueue" 2>/dev/null || true
mkdir -p "${DEPLOY}/dev/shm" 2>/dev/null || true
mkdir -p "${DEPLOY}/dev/pts" 2>/dev/null || true

# sysfs mountpoints are managed by systemd; don't try to create them in sysfs

# ensure /tmp exists
mkdir -p "${DEPLOY}/tmp" 2>/dev/null || true
chmod 1777 "${DEPLOY}/tmp" 2>/dev/null || true

log "mount points created"

# /init is inside the deployment root
exec switch_root "$DEPLOY" /init
exec /bin/sh
