#!/bin/sh
# Exercise the real, newly built libappstream UAPI-config reader this
# package patches: as_context_ensure_os_config_loaded() (src/as-context.c),
# driven through the compiled smoke binary rather than a reimplemented
# GKeyFile reader; see the comment at the top of the smoke source for how
# it reaches that reader and proves a selected file's own value took
# effect.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates
# (/usr/share/appstream/appstream.conf, /run/appstream.conf,
# /etc/appstream.conf) and refuses to run if one already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: appstream-uapi-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

glib_flags=$(PKG_CONFIG_PATH="${out_dir}/usr/lib/pkgconfig:/usr/lib/pkgconfig" \
  pkg-config --cflags --libs glib-2.0 gobject-2.0)

smoke_bin="${work_dir}/appstream-uapi-smoke"
gcc \
  -Wall -Wextra -Werror \
  -I"${out_dir}/usr/include/appstream" \
  "${smoke_source}" \
  -L"${out_dir}/usr/lib" \
  -lappstream ${glib_flags} \
  -o "${smoke_bin}"

log="${work_dir}/appstream-uapi-check.log"
err="${work_dir}/appstream-uapi-check.err"

run_probe() {
  # $1 origin argument, extra args ($2...) are extra env "NAME=value"
  # assignments applied only for this probe.
  origin=$1
  shift
  set +e
  env "$@" LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "${smoke_bin}" "${origin}" >"${log}" 2>"${err}"
  rc=$?
  set -e
}

assert_exit() {
  if [ "${rc}" -ne "$1" ]; then
    printf 'appstream UAPI smoke: expected exit %s, got %s\n' "$1" "${rc}" >&2
    printf 'stdout:\n' >&2; cat "${log}" >&2
    printf 'stderr:\n' >&2; cat "${err}" >&2
    exit 1
  fi
}
assert_contains() {
  if ! grep -qF -- "$1" "${log}"; then
    printf 'appstream UAPI smoke: expected to find "%s" in stdout\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_stderr_empty() {
  if [ -s "${err}" ]; then
    printf 'appstream UAPI smoke: expected empty stderr, got:\n' >&2
    cat "${err}" >&2
    exit 1
  fi
}

# as_context_ensure_os_config_loaded() indexes FreeRepos by the ID field
# from the same os-release files GLib's g_get_os_info() reads
# (/etc/os-release, then /usr/lib/os-release). Read the real value with
# the same precedence so this script writes config sections the patched
# reader will actually look up, instead of assuming a fixed distro ID.
os_release_planted=0
distro_id=$(sed -n 's/^ID=//p' /etc/os-release 2>/dev/null | head -n1 | tr -d '"')
if [ -z "${distro_id}" ]; then
  distro_id=$(sed -n 's/^ID=//p' /usr/lib/os-release 2>/dev/null | head -n1 | tr -d '"')
fi
if [ -z "${distro_id}" ]; then
  # The build sandbox ships no os-release file at all, so plant one at
  # /usr/lib/os-release: the same second path GLib's g_get_os_info() reads,
  # and the path Nex's own assemblies use (they never keep /etc/os-release).
  test ! -e /usr/lib/os-release
  distro_id=nexuapismoke
  mkdir -p /usr/lib
  printf 'ID=%s\n' "${distro_id}" > /usr/lib/os-release
  os_release_planted=1
fi

vendor_dir=/usr/share/appstream
vendor_conf="${vendor_dir}/appstream.conf"
run_conf=/run/appstream.conf
etc_conf=/etc/appstream.conf

cleanup_uapi_check() {
  rm -rf "${vendor_conf}" "${run_conf}" "${etc_conf}"
  if [ "${os_release_planted}" -eq 1 ]; then
    rm -f /usr/lib/os-release
  fi
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_conf}"
test ! -e "${run_conf}"
test ! -e "${etc_conf}"

write_conf() {
  # $1 conf file path  $2 FreeRepos value
  dir=$(dirname "$1")
  mkdir -p "${dir}"
  printf '[%s]\nFreeRepos=%s\n' "${distro_id}" "$2" > "$1"
}

### Vendor tier alone is selected when neither /run nor /etc exists, and
### its own FreeRepos value takes effect.

write_conf "${vendor_conf}" vendor-origin
run_probe vendor-origin
assert_exit 0
assert_contains 'floss: yes'

run_probe vendor-origin "G_MESSAGES_DEBUG=all"
assert_exit 0
assert_contains "Loading OS configuration from: ${vendor_conf}"

### /run is selected over the vendor tier: reading vendor-origin (only
### defined by the vendor file) now fails, because the selection is
### whole-file, not a merge, while run-origin (only defined by the /run
### file) now succeeds.

write_conf "${run_conf}" run-origin
run_probe run-origin
assert_exit 0
assert_contains 'floss: yes'
run_probe vendor-origin
assert_exit 0
assert_contains 'floss: no'

run_probe run-origin "G_MESSAGES_DEBUG=all"
assert_exit 0
assert_contains "Loading OS configuration from: ${run_conf}"

### /etc is selected over both /run and the vendor tier: reading
### run-origin now fails, and etc-origin (only defined by the /etc file)
### now succeeds.

write_conf "${etc_conf}" etc-origin
run_probe etc-origin
assert_exit 0
assert_contains 'floss: yes'
run_probe run-origin
assert_exit 0
assert_contains 'floss: no'

run_probe etc-origin "G_MESSAGES_DEBUG=all"
assert_exit 0
assert_contains "Loading OS configuration from: ${etc_conf}"

### Every tier absent is silent: no error, no warning, and the origin
### stays unrecognized as free.

rm -rf "${vendor_conf}" "${run_conf}" "${etc_conf}"
run_probe etc-origin
assert_exit 0
assert_contains 'floss: no'
assert_stderr_empty

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'appstream UAPI smoke: the OS configuration reader selected vendor, then run, then etc for a distinct FreeRepos value each time with no merge across tiers, named the chosen file in its debug log, and tolerated every tier absent with no stderr output\n'
