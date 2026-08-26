#!/bin/sh
# Drives the real, newly built polkitd and pkcheck through a real
# authorization decision over a private D-Bus system bus, rather than
# reimplementing polkit_backend_common_rules_file_name_cmp() or the JS
# rule-evaluation order. polkit_backend_authority_get() (called from
# polkitd's main(), before any bus connection is attempted) constructs the
# PolkitBackendJsAuthority singleton synchronously, which is what runs
# load_scripts() and setup_file_monitors(); this check only needs that
# object built and a bus for pkcheck's CheckAuthorization call to land on,
# so it does not need systemd, logind, or the host's own D-Bus.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates (/etc/polkit-1,
# /run/polkit-1, /usr/share/polkit-1/rules.d, /usr/share/polkit-1/actions,
# /etc/dbus-1/nex-polkit-smoke-bus.conf, /run/nex-polkit-smoke-bus.socket,
# and /etc/passwd) and refuses to run if one already exists.
set -eu

if [ "$#" -ne 2 ]; then
  printf '%s\n' \
    'usage: polkit-run-tier-check.sh OUT_DIR WORK_DIR' >&2
  exit 2
fi

out_dir=$1
work_dir=$2

polkitd_bin="${out_dir}/usr/lib/polkit-1/polkitd"
pkcheck_bin="${out_dir}/usr/bin/pkcheck"
polkit_ld_path="${out_dir}/usr/lib"

etc_dir=/etc/polkit-1/rules.d
run_dir=/run/polkit-1/rules.d
vendor_dir=/usr/share/polkit-1/rules.d
actions_dir=/usr/share/polkit-1/actions
action_file="${actions_dir}/nex-smoke.policy"
bus_conf=/etc/dbus-1/nex-polkit-smoke-bus.conf
bus_socket=/run/nex-polkit-smoke-bus.socket
passwd_file=/etc/passwd

log="${work_dir}/polkit-run-tier-check.log"

test ! -e /etc/polkit-1
test ! -e /run/polkit-1
test ! -e "${vendor_dir}"
test ! -e "${actions_dir}"
test ! -e "${bus_conf}"
test ! -e "${bus_socket}"
test ! -e "${passwd_file}"

bus_pid=
polkitd_pid=

cleanup_run_tier_check() {
  [ -z "${polkitd_pid}" ] || kill "${polkitd_pid}" >/dev/null 2>&1 || true
  [ -z "${polkitd_pid}" ] || wait "${polkitd_pid}" >/dev/null 2>&1 || true
  [ -z "${bus_pid}" ] || kill "${bus_pid}" >/dev/null 2>&1 || true
  [ -z "${bus_pid}" ] || wait "${bus_pid}" >/dev/null 2>&1 || true
  rm -rf /etc/polkit-1 /run/polkit-1 "${vendor_dir}" "${actions_dir}" \
    "${bus_conf}" "${bus_socket}" "${passwd_file}"
}
trap cleanup_run_tier_check EXIT HUP INT TERM

assert_exit() {
  if [ "$1" -ne "$2" ]; then
    printf 'polkit run-tier check: expected exit %s, got %s\n' "$2" "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_contains() {
  if ! grep -qF -- "$1" "${log}"; then
    printf 'polkit run-tier check: expected to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_not_contains() {
  if grep -qF -- "$1" "${log}"; then
    printf 'polkit run-tier check: did not expect to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}

wait_for_log_line() {
  needle=$1
  n=0
  while [ "${n}" -lt 50 ]; do
    if grep -qF -- "${needle}" "${log}" 2>/dev/null; then
      return 0
    fi
    sleep 0.1
    n=$((n + 1))
  done
  printf 'polkit run-tier check: timed out waiting for "%s"\n' "${needle}" >&2
  cat "${log}" >&2
  exit 1
}

mkdir -p /etc /run "${vendor_dir}" "${actions_dir}" /etc/dbus-1

own_uid=$(id -u)
own_gid=$(id -g)
printf 'polkitd:x:%s:%s:nex polkit run-tier check:/root:/bin/sh\n' \
  "${own_uid}" "${own_gid}" > "${passwd_file}"

cat > "${bus_conf}" <<BUSCONF
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>system</type>
  <listen>unix:path=${bus_socket}</listen>
  <policy context="default">
    <allow user="*"/>
    <allow own="*"/>
    <allow send_destination="*"/>
    <allow receive_sender="*"/>
  </policy>
</busconfig>
BUSCONF

cat > "${action_file}" <<'ACTIONS'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD polkit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/software/polkit/policyconfig-1.dtd">
<policyconfig>
  <vendor>Nex polkit run-tier check</vendor>
  <action id="nex.smoke.precedence">
    <description>Nex smoke: tier precedence</description>
    <message>Nex smoke: tier precedence</message>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>no</allow_active>
    </defaults>
  </action>
  <action id="nex.smoke.tier-etc">
    <description>Nex smoke: etc-only basename</description>
    <message>Nex smoke: etc-only basename</message>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>no</allow_active>
    </defaults>
  </action>
  <action id="nex.smoke.tier-run">
    <description>Nex smoke: run-only basename</description>
    <message>Nex smoke: run-only basename</message>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>no</allow_active>
    </defaults>
  </action>
  <action id="nex.smoke.tier-vendor">
    <description>Nex smoke: vendor-only basename</description>
    <message>Nex smoke: vendor-only basename</message>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>no</allow_active>
    </defaults>
  </action>
</policyconfig>
ACTIONS

export LD_LIBRARY_PATH=/usr/lib
/usr/bin/dbus-daemon --config-file="${bus_conf}" --fork --print-pid > "${work_dir}/polkit-run-tier-bus.pid"
bus_pid=$(cat "${work_dir}/polkit-run-tier-bus.pid")
unset LD_LIBRARY_PATH

n=0
while [ ! -S "${bus_socket}" ]; do
  n=$((n + 1))
  if [ "${n}" -ge 50 ]; then
    printf 'polkit run-tier check: private system bus never created %s\n' "${bus_socket}" >&2
    exit 1
  fi
  sleep 0.1
done

export DBUS_SYSTEM_BUS_ADDRESS="unix:path=${bus_socket}"

starttime=$(cut -d' ' -f22 "/proc/$$/stat")

start_polkitd() {
  LD_LIBRARY_PATH="${polkit_ld_path}" "${polkitd_bin}" > "${log}" 2>&1 &
  polkitd_pid=$!
  wait_for_log_line 'Acquired the name org.freedesktop.PolicyKit1 on the system bus'
}

stop_polkitd() {
  kill "${polkitd_pid}" >/dev/null 2>&1 || true
  wait "${polkitd_pid}" >/dev/null 2>&1 || true
  polkitd_pid=
}

check_action() {
  action_id=$1
  set +e
  LD_LIBRARY_PATH="${polkit_ld_path}" "${pkcheck_bin}" \
    --process "$$,${starttime},1000" --action-id "${action_id}" >/dev/null 2>>"${log}"
  rc=$?
  set -e
}

### A rules file present only in the vendor tier is loaded and its rule
### callback decides the check: no /etc or /run copy exists yet.

cat > "${vendor_dir}/50-precedence.rules" <<'RULES'
polkit.addRule(function(action, subject) {
    if (action.id == "nex.smoke.precedence") {
        polkit.log("precedence: vendor fired");
        return polkit.Result.YES;
    }
});
RULES

start_polkitd
check_action nex.smoke.precedence
assert_exit "${rc}" 0
assert_contains 'precedence: vendor fired'
stop_polkitd

### The same basename in /run outranks the vendor copy: this action now
### flips to the /run copy's opposite verdict, and only the /run copy's
### rule callback fires. Against the unpatched build (no PACKAGE_RUN_DIR
### entry), /run/polkit-1/rules.d is never scanned, so this would still
### show the vendor copy's verdict and marker instead.

mkdir -p "${run_dir}"
cat > "${run_dir}/50-precedence.rules" <<'RULES'
polkit.addRule(function(action, subject) {
    if (action.id == "nex.smoke.precedence") {
        polkit.log("precedence: run fired");
        return polkit.Result.NO;
    }
});
RULES

start_polkitd
check_action nex.smoke.precedence
assert_exit "${rc}" 1
assert_contains 'precedence: run fired'
assert_not_contains 'precedence: vendor fired'
stop_polkitd

### The same basename in /etc outranks both the /run and the vendor copy.

mkdir -p "${etc_dir}"
cat > "${etc_dir}/50-precedence.rules" <<'RULES'
polkit.addRule(function(action, subject) {
    if (action.id == "nex.smoke.precedence") {
        polkit.log("precedence: etc fired");
        return polkit.Result.YES;
    }
});
RULES

start_polkitd
check_action nex.smoke.precedence
assert_exit "${rc}" 0
assert_contains 'precedence: etc fired'
assert_not_contains 'precedence: run fired'
assert_not_contains 'precedence: vendor fired'
stop_polkitd

### Distinct basenames from all three tiers are all loaded and evaluated
### together, alongside the still-present, same-named precedence files.

cat > "${vendor_dir}/10-tier-vendor.rules" <<'RULES'
polkit.addRule(function(action, subject) {
    if (action.id == "nex.smoke.tier-vendor") {
        polkit.log("tier: vendor-only fired");
        return polkit.Result.YES;
    }
});
RULES

cat > "${run_dir}/20-tier-run.rules" <<'RULES'
polkit.addRule(function(action, subject) {
    if (action.id == "nex.smoke.tier-run") {
        polkit.log("tier: run-only fired");
        return polkit.Result.YES;
    }
});
RULES

cat > "${etc_dir}/30-tier-etc.rules" <<'RULES'
polkit.addRule(function(action, subject) {
    if (action.id == "nex.smoke.tier-etc") {
        polkit.log("tier: etc-only fired");
        return polkit.Result.YES;
    }
});
RULES

start_polkitd
check_action nex.smoke.tier-vendor
assert_exit "${rc}" 0
assert_contains 'tier: vendor-only fired'
check_action nex.smoke.tier-run
assert_exit "${rc}" 0
assert_contains 'tier: run-only fired'
check_action nex.smoke.tier-etc
assert_exit "${rc}" 0
assert_contains 'tier: etc-only fired'
check_action nex.smoke.precedence
assert_exit "${rc}" 0
assert_contains 'precedence: etc fired'
stop_polkitd

### A missing /run tier must not fail startup and must not repeat "Error
### opening rules directory" on every reload: /etc and the vendor tier stay
### populated from the steps above, but /run/polkit-1 is removed entirely.

rm -rf /run/polkit-1

start_polkitd
assert_not_contains 'Error opening rules directory'

touch "${vendor_dir}/60-reload-a.rules"
wait_for_log_line 'Reloading rules'
touch "${vendor_dir}/60-reload-b.rules"
n=0
while [ "$(grep -cF 'Reloading rules' "${log}")" -lt 2 ]; do
  n=$((n + 1))
  if [ "${n}" -ge 50 ]; then
    printf 'polkit run-tier check: timed out waiting for the second reload\n' >&2
    cat "${log}" >&2
    exit 1
  fi
  sleep 0.1
done
assert_not_contains 'Error opening rules directory'
stop_polkitd

printf 'polkit run-tier check: a vendor-only rules file was loaded, a same-named /run copy outranked it, a same-named /etc copy outranked both, distinct basenames from all three tiers loaded together, and a missing /run tier neither failed startup nor logged an error across two reloads\n'
