#!/bin/sh
# Exercise the real, newly built, newly installed fusermount3 this package
# patches: read_conf() and the mount_max / user_allow_other checks it feeds,
# driven through the compiled smoke harness rather than a reimplemented
# parser; see the comment at the top of the smoke source for how it reaches
# fusermount3 without a functional /dev/fuse.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates (/usr/share/fuse3,
# /run/fuse3, /etc/fuse.conf, /etc/mtab, /dev/fuse) and refuses to run if one
# already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: fuse3-uapi-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

smoke_bin="${work_dir}/fuse3-uapi-smoke"
gcc \
  -Wall -Wextra -Werror \
  "${smoke_source}" \
  -o "${smoke_bin}"

# fusermount3 refuses -o allow_other and enforces mount_max only for a
# non-root caller (getuid() != 0). This build runs the whole package build
# as the sandbox's real (and only mapped) root, so no ordinary setuid()
# can leave that identity; interpose getuid()/geteuid() instead, the same
# technique pkg/apps/virt/looking-glass.yaml uses for the same reason.
shim_c="${work_dir}/fuse3-fake-uid.c"
shim_so="${work_dir}/fuse3-fake-uid.so"
cat > "${shim_c}" <<'FAKEUID'
#include <sys/types.h>
#include <unistd.h>

uid_t getuid(void) { return 65534; }
uid_t geteuid(void) { return 65534; }
FAKEUID
gcc -shared -fPIC -o "${shim_so}" "${shim_c}"

fusermount3_bin="${out_dir}/usr/bin/fusermount3"
test -x "${fusermount3_bin}"

log="${work_dir}/fuse3-uapi-check.log"

vendor_dir=/usr/share/fuse3
vendor_file="${vendor_dir}/fuse.conf"
run_dir=/run/fuse3
run_file="${run_dir}/fuse.conf"
etc_file=/etc/fuse.conf
mtab_file=/etc/mtab
dev_fuse=/dev/fuse
mnt_dir="${work_dir}/fuse3-uapi-mnt"

cleanup_uapi_check() {
  rm -rf "${vendor_dir}" "${run_dir}" "${etc_file}" "${mtab_file}" \
    "${dev_fuse}" "${mnt_dir}"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_dir}"
test ! -e "${run_dir}"
test ! -e "${etc_file}"
test ! -e "${mtab_file}"
test ! -e "${dev_fuse}"

# The build sandbox does not create /etc, /run, or /dev on its own.
mkdir -p /etc /run /usr/share /dev

# open_fuse_device() only needs open("/dev/fuse", O_RDWR) to succeed; a
# plain world-writable regular file satisfies that without a real FUSE
# kernel connection, which this sandbox cannot provide. Every check this
# script exercises (read_conf(), the mount_max count, and the
# user_allow_other option check) runs before fusermount3 would ever read
# or write through that fd.
: > "${dev_fuse}"
chmod 0666 "${dev_fuse}"

# count_fuse_fs() (called by the mount_max check) opens _PATH_MOUNTED
# (/etc/mtab) with setmntent() and treats a missing file as an error,
# which would make count_fuse_fs() return -1 and mask every mount_max
# case below. An empty, valid mtab gives a real, reproducible count of
# zero currently-mounted FUSE filesystems.
: > "${mtab_file}"
chmod 0644 "${mtab_file}"

mkdir -p "${mnt_dir}"
chmod 0777 "${mnt_dir}"

run_probe() {
  # $1 = "-" for no -o flag, or an -o option string.
  set +e
  LD_PRELOAD="${shim_so}" "${smoke_bin}" "${fusermount3_bin}" "${mnt_dir}" \
    "$1" > "${log}" 2>&1
  rc=$?
  set -e
  if [ "${rc}" -ne 0 ]; then
    printf 'fuse3 UAPI smoke: harness exited %s\n' "${rc}" >&2
    cat "${log}" >&2
    exit 1
  fi
}

assert_contains() {
  if ! grep -qF -- "$1" "${log}"; then
    printf 'fuse3 UAPI smoke: expected to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}

assert_not_contains() {
  if grep -qF -- "$1" "${log}"; then
    printf 'fuse3 UAPI smoke: did not expect to find "%s" in the check log\n' \
      "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}

too_many_msg='too many FUSE filesystems mounted'
allow_other_msg="only allowed if 'user_allow_other' is set"

### Every tier absent is silent and matches today's behavior: mount_max
### stays at its compiled default (well above zero current mounts) and
### user_allow_other stays disabled.

run_probe -
assert_not_contains "${too_many_msg}"
run_probe allow_other
assert_contains "${allow_other_msg}"

### mount_max resolves from the highest tier that sets it: vendor alone,
### then /run overriding vendor, then /etc overriding both. Neither probe
### passes -o, so only the mount_max check (which runs before check_perm())
### can produce the message.

mkdir -p "${vendor_dir}"
printf 'mount_max = 0\n' > "${vendor_file}"
run_probe -
assert_contains "${too_many_msg}"

mkdir -p "${run_dir}"
printf 'mount_max = 1\n' > "${run_file}"
run_probe -
assert_not_contains "${too_many_msg}"

printf 'mount_max = 0\n' > "${etc_file}"
run_probe -
assert_contains "${too_many_msg}"

rm -f "${vendor_file}" "${run_file}" "${etc_file}"

### user_allow_other: each tier, read in isolation, is genuinely searched
### and honoured. fusermount3's file format only ever turns this flag on
### (there is no "off" directive), so a lower tier's grant can never be
### taken away by a higher tier that simply omits the keyword; the
### fail-closed case below is where a higher tier does override a lower
### tier's grant, by forcing it off.

printf 'user_allow_other\n' > "${vendor_file}"
run_probe allow_other
assert_not_contains "${allow_other_msg}"

rm -f "${vendor_file}"
printf 'user_allow_other\n' > "${run_file}"
run_probe allow_other
assert_not_contains "${allow_other_msg}"

rm -f "${run_file}"
printf 'user_allow_other\n' > "${etc_file}"
run_probe allow_other
assert_not_contains "${allow_other_msg}"

rm -f "${etc_file}"
run_probe allow_other
assert_contains "${allow_other_msg}"

### Fail closed: a tier that exists but cannot be read must never produce a
### more permissive result than every tier being absent. /run/fuse3 is made
### unreadable by replacing the directory itself with a plain file, so
### opening /run/fuse3/fuse.conf fails with ENOTDIR; this is deterministic
### and, unlike a permission bit, is not something this sandbox's root
### capabilities could let the probe quietly bypass. The vendor tier alone
### grants user_allow_other and sets an aggressive mount_max, and both must
### be discarded once run's read fails.

printf 'user_allow_other\nmount_max = 0\n' > "${vendor_file}"
rm -rf "${run_dir}"
: > "${run_dir}"

run_probe -
assert_not_contains "${too_many_msg}"
run_probe allow_other
assert_contains "${allow_other_msg}"

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'fuse3 UAPI smoke: mount_max resolved vendor, then run, then etc, user_allow_other took effect from each tier read in isolation, every tier absent matched today, and an unreadable tier forced both settings back to their compiled defaults\n'
