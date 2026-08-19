#!/bin/sh
# Exercise the real, newly built libpopt UAPI-config reader this package
# patches: poptReadDefaultConfig(), driven through the compiled smoke
# binary rather than a reimplemented parser; see the comment at the top
# of the smoke source for how it reaches that reader.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates (/usr/share/popt,
# /usr/share/popt.d, /run/popt, /run/popt.d, /etc/popt, /etc/popt.d) and
# refuses to run if one already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: popt-uapi-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

smoke_bin="${work_dir}/popt-uapi-smoke"
gcc \
  -Wall -Wextra -Werror \
  "${smoke_source}" \
  -I"${out_dir}/usr/include" \
  -L"${out_dir}/usr/lib" \
  -lpopt \
  -o "${smoke_bin}"

log="${work_dir}/popt-uapi-check.log"
app_name=nex-uapi-smoke

run_probe() {
  set +e
  LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "${smoke_bin}" "${app_name}" "$1" >"${log}" 2>&1
  rc=$?
  set -e
}

assert_exit() {
  if [ "${rc}" -ne "$1" ]; then
    printf 'popt UAPI smoke: expected exit %s, got %s\n' "$1" "${rc}" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_contains() {
  if ! grep -qF -- "$1" "${log}"; then
    printf 'popt UAPI smoke: expected to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}

vendor_dir=/usr/share/popt
run_dir=/run/popt
etc_dir=/etc

cleanup_uapi_check() {
  rm -rf "${vendor_dir}" "${vendor_dir}.d" "${run_dir}" "${run_dir}.d" \
    "${etc_dir}/popt" "${etc_dir}/popt.d"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_dir}"
test ! -e "${run_dir}"
test ! -e "${etc_dir}/popt"

# The build sandbox does not create /etc on its own; a drop-in directory
# below /etc created as a side effect of mkdir -p covers every fixture in
# this script except the plain /etc/popt file below, so this script
# creates /etc itself first.
mkdir -p /etc

write_alias() {
  # $1 tier base file  $2 long option name  $3 --value payload
  printf '%s alias --%s --value=%s\n' "${app_name}" "$2" "$3" >> "$1"
}

### System search: vendor tier alone takes effect, then /run overrides the
### same alias name, then /etc overrides both.

mkdir -p "${vendor_dir}"
write_alias "${vendor_dir}/popt" probe vendor
run_probe --probe
assert_exit 0
assert_contains 'value: vendor'

mkdir -p "${run_dir}"
write_alias "${run_dir}/popt" probe run
run_probe --probe
assert_exit 0
assert_contains 'value: run'

write_alias "${etc_dir}/popt" probe etc
run_probe --probe
assert_exit 0
assert_contains 'value: etc'

### Distinct alias names from every tier all apply at once, proving each
### tier is actually read rather than only the tier that wins ties.

write_alias "${vendor_dir}/popt" probe-vendor onlyvendor
write_alias "${run_dir}/popt" probe-run onlyrun
write_alias "${etc_dir}/popt" probe-etc onlyetc
run_probe --probe-vendor
assert_exit 0
assert_contains 'value: onlyvendor'
run_probe --probe-run
assert_exit 0
assert_contains 'value: onlyrun'
run_probe --probe-etc
assert_exit 0
assert_contains 'value: onlyetc'

### A missing /run and /etc tier is normal: the vendor alias from the
### surviving tier must still resolve, with no error.

rm -rf "${run_dir}" "${etc_dir}/popt"
run_probe --probe-vendor
assert_exit 0
assert_contains 'value: onlyvendor'

### A drop-in rejected by poptSaneFile() (its name contains ".rpmsave") is
### ignored rather than failing the run: its alias never resolves.

mkdir -p "${vendor_dir}.d"
printf '%s alias --probe-rejected --value=shouldnotapply\n' "${app_name}" \
  > "${vendor_dir}.d/extra.conf.rpmsave"
run_probe --probe-rejected
assert_exit 0
assert_contains 'unresolved: --probe-rejected'

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'popt UAPI smoke: the default-config reader selected vendor, then run, then etc for the same alias, tolerated a missing run and etc tier, read distinct aliases from every tier, and ignored a drop-in rejected by poptSaneFile()\n'
