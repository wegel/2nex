#!/bin/sh
# Exercise the real, newly built krb5 UAPI-config readers this package
# patches: the profile library behind krb5_init_context() and the GSSAPI
# mechanism registry behind gss_indicate_mechs(). Both are driven through
# the compiled smoke binary rather than a reimplemented parser; see the
# comment at the top of the smoke source for how each mode reaches its
# reader.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates (/usr/share/krb5,
# /run/krb5, /etc/krb5.conf, /usr/lib/gss, /run/gss, /etc/gss) and refuses to
# run if one already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: krb5-uapi-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

smoke_bin="${work_dir}/krb5-uapi-smoke"
gcc \
  -Wall -Wextra -Werror \
  "${smoke_source}" \
  -I"${out_dir}/usr/include" \
  -L"${out_dir}/usr/lib" \
  -lkrb5 -lgssapi_krb5 -lk5crypto -lkrb5support -lcom_err \
  -o "${smoke_bin}"

log="${work_dir}/krb5-uapi-check.log"

run_profile() {
  set +e
  env -i \
    PATH="${PATH}" \
    LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "$@" \
    "${smoke_bin}" profile >"${log}" 2>&1
  rc=$?
  set -e
}

run_gss() {
  set +e
  LD_LIBRARY_PATH="${out_dir}/usr/lib" "${smoke_bin}" gss >"${log}" 2>&1
  rc=$?
  set -e
}

assert_exit() {
  if [ "${rc}" -ne "$1" ]; then
    printf 'krb5 UAPI smoke: expected exit %s, got %s\n' "$1" "${rc}" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_contains() {
  if ! grep -qF -- "$1" "${log}"; then
    printf 'krb5 UAPI smoke: expected to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_not_contains() {
  if grep -qF -- "$1" "${log}"; then
    printf 'krb5 UAPI smoke: did not expect to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_no_warning() {
  if grep -qF -- 'krb5 uapi smoke:' "${log}"; then
    printf 'krb5 UAPI smoke: did not expect an error line in the check log\n' >&2
    cat "${log}" >&2
    exit 1
  fi
}

vendor_profile_dir=/usr/share/krb5
run_profile_dir=/run/krb5
etc_profile_file=/etc/krb5.conf

vendor_mech_dir=/usr/lib/gss
run_mech_dir=/run/gss
etc_mech_dir=/etc/gss

cleanup_uapi_check() {
  rm -rf "${vendor_profile_dir}" "${run_profile_dir}" "${etc_profile_file}" \
    "${vendor_mech_dir}" "${run_mech_dir}" "${etc_mech_dir}"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_profile_dir}"
test ! -e "${run_profile_dir}"
test ! -e "${etc_profile_file}"
test ! -e "${vendor_mech_dir}"
test ! -e "${run_mech_dir}"
test ! -e "${etc_mech_dir}"

# The build sandbox does not create /etc on its own; other packages'
# writable-config drop-in directories (/etc/gss below) create it as a side
# effect of their own mkdir -p, but /etc/krb5.conf is a single file with no
# such side effect, so this script creates the directory itself.
mkdir -p /etc

write_profile() {
  printf '[nex-uapi-smoke]\nvalue = %s\n' "$2" > "$1"
}

### Profile search: vendor tier alone takes effect, then /run overrides it,
### then /etc overrides both. Missing vendor and /run afterward is silent.

mkdir -p "${vendor_profile_dir}"
write_profile "${vendor_profile_dir}/krb5.conf" vendor
run_profile
assert_exit 0
assert_contains 'profile value: vendor'

mkdir -p "${run_profile_dir}"
write_profile "${run_profile_dir}/krb5.conf" run
run_profile
assert_exit 0
assert_contains 'profile value: run'
assert_not_contains 'profile value: vendor'

write_profile "${etc_profile_file}" etc
run_profile
assert_exit 0
assert_contains 'profile value: etc'
assert_not_contains 'profile value: run'
assert_not_contains 'profile value: vendor'

### A missing vendor and /run tier is normal: only /etc remains, and the
### profile library must neither error nor warn about the absent tiers.

rm -rf "${vendor_profile_dir}" "${run_profile_dir}"
run_profile
assert_exit 0
assert_contains 'profile value: etc'
assert_no_warning

### KRB5_CONFIG still replaces the whole compiled search, not only the
### missing tiers: recreate all three tiers with distinct values, then prove
### the environment variable's own file wins over every one of them.

mkdir -p "${vendor_profile_dir}" "${run_profile_dir}"
write_profile "${vendor_profile_dir}/krb5.conf" vendor
write_profile "${run_profile_dir}/krb5.conf" run
custom_conf="${work_dir}/krb5-uapi-custom.conf"
write_profile "${custom_conf}" override
run_profile KRB5_CONFIG="${custom_conf}"
assert_exit 0
assert_contains 'profile value: override'
assert_not_contains 'profile value: etc'
assert_not_contains 'profile value: run'
assert_not_contains 'profile value: vendor'
rm -f "${custom_conf}"
rm -rf "${vendor_profile_dir}" "${run_profile_dir}" "${etc_profile_file}"

### GSSAPI mechanism registry: a vendor base file, a /run base file, and the
### existing /etc base file are all read together, and so are their .d
### drop-ins, alongside the pre-existing /etc entries.

write_mech() {
  printf '%s\n' "nex-uapi-smoke-$2 $1 libnex-uapi-smoke-$2.so" > "$3"
}

mkdir -p "${vendor_mech_dir}"
write_mech 1.3.6.1.4.1.99999.1.1 vendor-base "${vendor_mech_dir}/mech"
run_gss
assert_exit 0
assert_contains '1 3 6 1 4 1 99999 1 1 }'

mkdir -p "${run_mech_dir}"
write_mech 1.3.6.1.4.1.99999.1.2 run-base "${run_mech_dir}/mech"
run_gss
assert_exit 0
assert_contains '1 3 6 1 4 1 99999 1 1 }'
assert_contains '1 3 6 1 4 1 99999 1 2 }'

mkdir -p "${etc_mech_dir}"
write_mech 1.3.6.1.4.1.99999.1.3 etc-base "${etc_mech_dir}/mech"
run_gss
assert_exit 0
assert_contains '1 3 6 1 4 1 99999 1 1 }'
assert_contains '1 3 6 1 4 1 99999 1 2 }'
assert_contains '1 3 6 1 4 1 99999 1 3 }'

mkdir -p "${vendor_mech_dir}/mech.d" "${run_mech_dir}/mech.d" "${etc_mech_dir}/mech.d"
write_mech 1.3.6.1.4.1.99999.1.4 vendor-drop "${vendor_mech_dir}/mech.d/extra.conf"
write_mech 1.3.6.1.4.1.99999.1.5 run-drop "${run_mech_dir}/mech.d/extra.conf"
write_mech 1.3.6.1.4.1.99999.1.6 etc-drop "${etc_mech_dir}/mech.d/extra.conf"
run_gss
assert_exit 0
assert_contains '1 3 6 1 4 1 99999 1 1 }'
assert_contains '1 3 6 1 4 1 99999 1 2 }'
assert_contains '1 3 6 1 4 1 99999 1 3 }'
assert_contains '1 3 6 1 4 1 99999 1 4 }'
assert_contains '1 3 6 1 4 1 99999 1 5 }'
assert_contains '1 3 6 1 4 1 99999 1 6 }'

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'krb5 UAPI smoke: the profile reader selected vendor, then run, then etc for the same key, tolerated a missing vendor and run tier, KRB5_CONFIG still replaced the whole search, and the GSSAPI registry read vendor, run, and etc base files and .d drop-ins\n'
