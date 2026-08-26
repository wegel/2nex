#!/bin/sh
# Exercise the real, newly built libsmbclient through smbc_new_context() and
# smbc_init_context(), which together run the module's smb.conf load and
# resolve the effective "workgroup" parameter. The caller supplies an
# isolated package output and build work directory; this script owns every
# absolute fixture path it creates below /etc/samba, /run/samba, and
# /usr/lib/samba, plus the fixture $HOME it points the smoke at, and refuses
# to run if one of the system fixture paths already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: samba-uapi-config-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

libsmbclient_so="${out_dir}/usr/lib/libsmbclient.so.0"

# Point instructions: enchant2 lost four full builds to a smoke that
# referenced an internal symbol the shared library does not export. Check
# every symbol this smoke calls before compiling against it.
for sym in \
  smbc_new_context smbc_init_context smbc_getWorkgroup smbc_free_context; do
  if ! nm -D "${libsmbclient_so}" | grep -qE "[[:space:]]T[[:space:]]${sym}(@|\$)"; then
    printf 'samba-uapi-config-check: %s is not an exported symbol of %s\n' \
      "${sym}" "${libsmbclient_so}" >&2
    nm -D "${libsmbclient_so}" | grep "${sym}" >&2 || true
    exit 1
  fi
done

smoke_bin="${work_dir}/samba-uapi-config-smoke"
# libsmbclient.so needs many private-samba libraries below usr/lib/samba;
# they resolve at runtime through LD_LIBRARY_PATH below, so the link step
# only needs to accept that they are unresolved right now.
gcc \
  -Wall -Wextra -Werror \
  -I "${out_dir}/usr/include/samba-4.0" \
  "${smoke_source}" \
  -L"${out_dir}/usr/lib" -L"${out_dir}/usr/lib/samba" \
  -Wl,--allow-shlib-undefined \
  -lsmbclient \
  -o "${smoke_bin}"

# The libsmbclient module init this smoke drives always probes the kernel's
# network interfaces (load_interfaces(), unconditional, right after the
# smb.conf load this patch changes) and refuses to continue if it finds
# none. The build sandbox's private network namespace starts with loopback
# present but administratively down, which yields zero usable interfaces, so
# bring it up once before the first smoke run.
ip link set lo up

fixture_home="${work_dir}/samba-uapi-config-home"
mkdir -p "${fixture_home}/.smb"

run_smoke() {
  env -i \
    PATH="${PATH}" \
    HOME="${fixture_home}" \
    LD_LIBRARY_PATH="${out_dir}/usr/lib:${out_dir}/usr/lib/samba" \
    "${smoke_bin}"
}

assert_eq() {
  got=$1
  want=$2
  label=$3
  if [ "${got}" != "${want}" ]; then
    printf 'samba UAPI smoke (%s): got "%s", want "%s"\n' \
      "${label}" "${got}" "${want}" >&2
    exit 1
  fi
}

etc_dir=/etc/samba
run_dir=/run/samba
vendor_dir=/usr/lib/samba

cleanup_uapi_check() {
  rm -f "${etc_dir}/smb.conf" "${run_dir}/smb.conf" "${vendor_dir}/smb.conf"
  rmdir "${etc_dir}" "${run_dir}" "${vendor_dir}" 2>/dev/null || true
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${etc_dir}/smb.conf"
test ! -e "${run_dir}"
test ! -e "${vendor_dir}"
mkdir -p "${etc_dir}" "${run_dir}" "${vendor_dir}"

### workgroup: vendor (/usr/lib/samba), then runtime (/run/samba), then
### administrator (/etc/samba) last, with $HOME/.smb/smb.conf outranking
### every system tier. A tier absent at every level is silent and falls
### back to the compiled default.

got=$(run_smoke)
assert_eq "${got}" "WORKGROUP" "every tier absent"

printf '[global]\n   workgroup = VENDORWG\n' > "${vendor_dir}/smb.conf"
got=$(run_smoke)
assert_eq "${got}" "VENDORWG" "vendor only"

printf '[global]\n   workgroup = RUNWG\n' > "${run_dir}/smb.conf"
got=$(run_smoke)
assert_eq "${got}" "RUNWG" "run beats vendor"

printf '[global]\n   workgroup = ETCWG\n' > "${etc_dir}/smb.conf"
got=$(run_smoke)
assert_eq "${got}" "ETCWG" "etc beats run and vendor"

printf '[global]\n   workgroup = HOMEWG\n' > "${fixture_home}/.smb/smb.conf"
got=$(run_smoke)
assert_eq "${got}" "HOMEWG" "user beats every system tier"

rm "${fixture_home}/.smb/smb.conf"
rm "${etc_dir}/smb.conf" "${run_dir}/smb.conf" "${vendor_dir}/smb.conf"

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'samba UAPI smoke: workgroup selection honoured /etc, /run, and /usr/lib/samba in order, and the user config still outranked every system tier\n'
