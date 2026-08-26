#!/bin/sh
# Exercise the real, newly built and installed iproute2 UAPI-config readers
# this package patches: the CONF_ETC_DIR/CONF_RUN_DIR/CONF_USR_DIR main-file
# fallback and the rtnl_tabhash_readdir()/rtnl_tabhash_initialize_dir()
# drop-in merge in lib/rt_names.c, driven entirely through the installed
# `ip` command line tool rather than a reimplemented parser.
#
# rt_tables is one of the nine patched families and shares its reader code
# (rtnl_tab_initialize()/rtnl_hash_initialize(), rtnl_tabhash_readdir(),
# rtnl_tabhash_initialize_dir()) with the other eight, so a probe that proves
# rt_tables proves the shared reader for all of them. `ip route show table
# all` prints each route's numeric table id translated through
# rtnl_rttable_n2a(), which is the read side of the same lazily-initialized
# table this patch adds a /run tier to; adding one route per probed table id
# and reading it back is the cheapest way to make that translation observable
# without a kernel routing decision or a second process.
#
# The caller supplies an isolated package output directory; this script owns
# every absolute fixture path it creates (/usr/share/iproute2, /run/iproute2,
# /etc/iproute2) and refuses to run if one already exists. It also brings up
# `lo` and adds routes into unused table ids in the sandbox's already-isolated
# network namespace (see .agents/knowledge/build-sandbox.md); it does not
# touch any table, interface, or address a real deployment would use.
set -eu

if [ "$#" -ne 2 ]; then
  printf '%s\n' 'usage: iproute2-uapi-check.sh OUT_DIR WORK_DIR' >&2
  exit 2
fi

out_dir=$1
work_dir=$2

ip_bin="${out_dir}/usr/bin/ip"
log="${work_dir}/iproute2-uapi-check.log"

vendor_dir=/usr/share/iproute2
run_dir=/run/iproute2
etc_dir=/etc/iproute2

cleanup_uapi_check() {
  rm -rf "${vendor_dir}" "${run_dir}" "${etc_dir}"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_dir}"
test ! -e "${run_dir}"
test ! -e "${etc_dir}"

"${ip_bin}" link set lo up

# assert_table_name TABLE_ID EXPECTED_NAME
# Reads back the resolved name for a route already planted in TABLE_ID and
# fails unless "table EXPECTED_NAME" appears on that route's line.
assert_table_name() {
  id=$1
  expected=$2

  "${ip_bin}" -o route show table all > "${log}" 2>&1
  line=$(grep -F "198.51.100.${id} " "${log}") || {
    printf 'iproute2 UAPI smoke: no route found for table %s\n' "${id}" >&2
    cat "${log}" >&2
    exit 1
  }
  case "${line}" in
  *"table ${expected} "*) ;;
  *)
    printf 'iproute2 UAPI smoke: expected "table %s" for table id %s, got: %s\n' \
      "${expected}" "${id}" "${line}" >&2
    exit 1
    ;;
  esac
}

# assert_table_numeric TABLE_ID
# Fails unless TABLE_ID's route line names the table only by its raw number,
# proving no rt_tables entry resolved it in the current tier configuration.
assert_table_numeric() {
  id=$1

  "${ip_bin}" -o route show table all > "${log}" 2>&1
  line=$(grep -F "198.51.100.${id} " "${log}") || {
    printf 'iproute2 UAPI smoke: no route found for table %s\n' "${id}" >&2
    cat "${log}" >&2
    exit 1
  }
  case "${line}" in
  *"table ${id} "*) ;;
  *)
    printf 'iproute2 UAPI smoke: expected numeric "table %s", got: %s\n' \
      "${id}" "${line}" >&2
    exit 1
    ;;
  esac
}

### Main-file precedence: vendor alone resolves, then /run overrides the
### same table id, then /etc overrides both. A missing /run and /etc tier
### is silent: removing them afterward must fall back to the surviving
### vendor mapping with no error.

mkdir -p "${vendor_dir}"
printf '101 mainvendor\n' > "${vendor_dir}/rt_tables"
"${ip_bin}" route add 198.51.100.101/32 dev lo table 101
assert_table_name 101 mainvendor

mkdir -p "${run_dir}"
printf '101 mainrun\n' > "${run_dir}/rt_tables"
assert_table_name 101 mainrun

mkdir -p "${etc_dir}"
printf '101 mainetc\n' > "${etc_dir}/rt_tables"
assert_table_name 101 mainetc

rm -f "${run_dir}/rt_tables" "${etc_dir}/rt_tables"
assert_table_name 101 mainvendor

### Drop-in merge: distinct table ids named by a vendor-only, a run-only,
### and an etc-only drop-in file all resolve at once, proving every tier is
### actually read rather than only the tier that would win a tie.

mkdir -p "${vendor_dir}/rt_tables.d" "${run_dir}/rt_tables.d" "${etc_dir}/rt_tables.d"
printf '111 dropvendor\n' > "${vendor_dir}/rt_tables.d/vendor-only.conf"
printf '112 droprun\n' > "${run_dir}/rt_tables.d/run-only.conf"
printf '113 dropetc\n' > "${etc_dir}/rt_tables.d/etc-only.conf"
"${ip_bin}" route add 198.51.100.111/32 dev lo table 111
"${ip_bin}" route add 198.51.100.112/32 dev lo table 112
"${ip_bin}" route add 198.51.100.113/32 dev lo table 113
assert_table_name 111 dropvendor
assert_table_name 112 droprun
assert_table_name 113 dropetc

### Drop-in basename masking: the same basename ("shared.conf") appears in
### all three drop-in directories, mapping the same table id to a different
### name in each. Only the highest present tier's copy applies; removing it
### reveals the next tier's copy rather than merging the two.

printf '120 sharedvendor\n' > "${vendor_dir}/rt_tables.d/shared.conf"
printf '120 sharedrun\n' > "${run_dir}/rt_tables.d/shared.conf"
printf '120 sharedetc\n' > "${etc_dir}/rt_tables.d/shared.conf"
"${ip_bin}" route add 198.51.100.120/32 dev lo table 120
assert_table_name 120 sharedetc

rm -f "${etc_dir}/rt_tables.d/shared.conf"
assert_table_name 120 sharedrun

rm -f "${run_dir}/rt_tables.d/shared.conf"
assert_table_name 120 sharedvendor

### A completely missing /run and /etc tier (both the main file and the
### drop-in directory) is silent: an id with no rt_tables entry anywhere
### still prints its raw number instead of erroring out.

rm -rf "${run_dir}" "${etc_dir}"
"${ip_bin}" route add 198.51.100.130/32 dev lo table 130
assert_table_numeric 130

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'iproute2 UAPI smoke: rt_tables resolved vendor, then run, then etc for the same id, tolerated a missing run and etc tier, merged distinct ids from every drop-in tier, masked a shared drop-in basename in vendor/run/etc order, and left an unmapped id numeric\n'
