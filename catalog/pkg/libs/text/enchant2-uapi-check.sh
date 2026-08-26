#!/bin/sh
# Exercises the real, newly built libenchant-2 UAPI-config reader this
# package patches: enchant_get_conf_dirs() and
# enchant_broker_load_provider_ordering() (lib/provider.c, lib/broker.c),
# driven through two small compiled EnchantProvider modules and the
# compiled smoke binary rather than a reimplemented parser; see the
# comments at the top of enchant2-uapi-provider.c and
# enchant2-uapi-smoke.c for how each reaches the real reader.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates (/usr/share/enchant-2,
# /run/enchant-2, /etc/enchant-2, /usr/lib/enchant-2) and refuses to run if
# one already exists.
set -eu

if [ "$#" -ne 4 ]; then
  printf '%s\n' \
    'usage: enchant2-uapi-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE PROVIDER_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3
provider_source=$4

glib_flags=$(PKG_CONFIG_PATH="${out_dir}/usr/lib/pkgconfig:/usr/lib/pkgconfig" \
  pkg-config --cflags --libs glib-2.0 gobject-2.0)

smoke_bin="${work_dir}/enchant2-uapi-smoke"
gcc \
  -Wall -Wextra -Werror \
  -I"${work_dir}/lib" \
  "${smoke_source}" \
  -L"${out_dir}/usr/lib" \
  -lenchant-2 ${glib_flags} \
  -o "${smoke_bin}"

build_provider() {
  # $1 provider name  $2 output .so path
  gcc \
    -Wall -Wextra -Werror \
    -shared -fPIC \
    -DPROVIDER_NAME="\"$1\"" \
    -I"${work_dir}/lib" \
    "${provider_source}" \
    -L"${out_dir}/usr/lib" \
    -lenchant-2 ${glib_flags} \
    -o "$2"
}

log="${work_dir}/enchant2-uapi-check.log"

run_probe() {
  set +e
  LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "${smoke_bin}" "$1" >"${log}" 2>&1
  rc=$?
  set -e
}

assert_exit() {
  if [ "${rc}" -ne "$1" ]; then
    printf 'enchant UAPI smoke: expected exit %s, got %s\n' "$1" "${rc}" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_contains() {
  if ! grep -qF -- "$1" "${log}"; then
    printf 'enchant UAPI smoke: expected to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}

module_dir=/usr/lib/enchant-2
vendor_dir=/usr/share/enchant-2
run_dir=/run/enchant-2
etc_dir=/etc/enchant-2
user_dir="${work_dir}/enchant2-uapi-user-config"

cleanup_uapi_check() {
  rm -rf "${module_dir}" "${vendor_dir}" "${run_dir}" "${etc_dir}" "${user_dir}"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${module_dir}"
test ! -e "${vendor_dir}"
test ! -e "${run_dir}"
test ! -e "${etc_dir}"

# The build sandbox does not create /etc on its own; the plain /etc/enchant-2
# directory below covers it, but write it explicitly first so mkdir -p never
# depends on that side effect.
mkdir -p /etc "${module_dir}" "${vendor_dir}"

build_provider nex-uapi-want "${module_dir}/libnex-uapi-want.so"
build_provider nex-uapi-other "${module_dir}/libnex-uapi-other.so"

write_ordering() {
  # $1 tier's enchant.ordering file  $2 tag  $3 comma-separated provider names
  printf '%s:%s\n' "$2" "$3" >> "$1"
}

### System search: vendor tier alone takes effect, then /run overrides the
### same tag, then /etc overrides both.

tag1=nexuapishared

write_ordering "${vendor_dir}/enchant.ordering" "${tag1}" \
  nex-uapi-other,nex-uapi-want
run_probe "${tag1}"
assert_exit 0
assert_contains 'order: nex-uapi-other nex-uapi-want'

mkdir -p "${run_dir}"
write_ordering "${run_dir}/enchant.ordering" "${tag1}" \
  nex-uapi-want,nex-uapi-other
run_probe "${tag1}"
assert_exit 0
assert_contains 'order: nex-uapi-want nex-uapi-other'

mkdir -p "${etc_dir}"
write_ordering "${etc_dir}/enchant.ordering" "${tag1}" \
  nex-uapi-other,nex-uapi-want
run_probe "${tag1}"
assert_exit 0
assert_contains 'order: nex-uapi-other nex-uapi-want'

### Distinct language tags set in only one tier each all apply, proving
### every tier's file is actually read rather than only the tier that wins
### a shared-tag tie above.

tag_vendor=nexuapionlyvendor
tag_run=nexuapionlyrun
tag_etc=nexuapionlyetc

write_ordering "${vendor_dir}/enchant.ordering" "${tag_vendor}" nex-uapi-want
write_ordering "${run_dir}/enchant.ordering" "${tag_run}" nex-uapi-want
write_ordering "${etc_dir}/enchant.ordering" "${tag_etc}" nex-uapi-want

run_probe "${tag_vendor}"
assert_exit 0
assert_contains 'order: nex-uapi-want'
run_probe "${tag_run}"
assert_exit 0
assert_contains 'order: nex-uapi-want'
run_probe "${tag_etc}"
assert_exit 0
assert_contains 'order: nex-uapi-want'

### A missing /run and /etc tier is normal: the shared tag's vendor entry
### must still resolve, with no error.

rm -rf "${run_dir}" "${etc_dir}"
run_probe "${tag1}"
assert_exit 0
assert_contains 'order: nex-uapi-other nex-uapi-want'

### ENCHANT_CONFIG_DIR still takes precedence over every system tier.

mkdir -p "${etc_dir}" "${user_dir}"
write_ordering "${etc_dir}/enchant.ordering" "${tag1}" \
  nex-uapi-other,nex-uapi-want
write_ordering "${user_dir}/enchant.ordering" "${tag1}" \
  nex-uapi-want,nex-uapi-other

set +e
LD_LIBRARY_PATH="${out_dir}/usr/lib" ENCHANT_CONFIG_DIR="${user_dir}" \
  "${smoke_bin}" "${tag1}" >"${log}" 2>&1
rc=$?
set -e
assert_exit 0
assert_contains 'order: nex-uapi-want nex-uapi-other'

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'enchant2 UAPI smoke: the provider-ordering reader selected vendor, then run, then etc for a shared tag, read distinct tags set in only one tier each, tolerated a missing run and etc tier, and let ENCHANT_CONFIG_DIR override every system tier\n'
