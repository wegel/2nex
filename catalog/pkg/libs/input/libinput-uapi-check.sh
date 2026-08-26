#!/bin/sh
# Exercise the real, newly built libinput quirks reader
# (quirks_init_subsystem() in src/quirks.c) two ways:
#
#   - The already-installed /usr/libexec/libinput/libinput-quirks tool's
#     `validate --verbose` action initializes the real quirks subsystem with
#     no device required, and with --verbose its debug log names every
#     absolute quirks file path the real scandir()/parse_file() calls
#     actually selected, in the order they were parsed. This proves tier
#     order, same-basename masking (empty file and /dev/null symlink), and
#     distinct-basename merging directly from that log, without
#     reimplementing the selection algorithm.
#
#   - A small compiled smoke, linked against the real newly built
#     libinput.so, calls libinput_udev_assign_seat() (also no device
#     required; see the comment in the smoke source), which reaches
#     libinput_init_quirks() with a real log handler installed. That path
#     suppresses the tool's per-file debug lines, but its QLOG_ERROR lines
#     survive, which is enough to prove LIBINPUT_QUIRKS_DIR still replaces
#     the whole system search, including the required vendor-tier check.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates and refuses to run
# if one already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: libinput-uapi-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

quirks_bin="${out_dir}/usr/libexec/libinput/libinput-quirks"

smoke_bin="${work_dir}/libinput-uapi-smoke"
gcc \
  -Wall -Wextra -Werror \
  "${smoke_source}" \
  -I"${out_dir}/usr/include" \
  -L"${out_dir}/usr/lib" \
  -linput -ludev \
  -o "${smoke_bin}"

log="${work_dir}/libinput-uapi-check.log"

run_validate() {
  set +e
  LD_LIBRARY_PATH="${out_dir}/usr/lib" "${quirks_bin}" validate --verbose \
    >"${log}" 2>&1
  rc=$?
  set -e
}

run_smoke() {
  set +e
  env -i \
    PATH="${PATH}" \
    LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "$@" \
    "${smoke_bin}" >"${log}" 2>&1
  rc=$?
  set -e
}

assert_exit() {
  if [ "${rc}" -ne "$1" ]; then
    printf 'libinput UAPI smoke: expected exit %s, got %s\n' "$1" "${rc}" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_contains() {
  if ! grep -qF -- "$1" "${log}"; then
    printf 'libinput UAPI smoke: expected to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}
assert_not_contains() {
  if grep -qF -- "$1" "${log}"; then
    printf 'libinput UAPI smoke: did not expect to find "%s" in the check log\n' "$1" >&2
    cat "${log}" >&2
    exit 1
  fi
}

write_quirks() {
  printf '[nex-uapi-smoke]\nMatchName=nex-uapi-smoke-*\nModelBouncingKeys=1\n' > "$1"
}

vendor_dir=/usr/share/libinput
run_dir=/run/libinput
etc_dir=/etc/libinput

cleanup_uapi_check() {
  rm -rf "${vendor_dir}" "${run_dir}" "${etc_dir}"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_dir}"
test ! -e "${run_dir}"
test ! -e "${etc_dir}"
mkdir -p "${vendor_dir}" "${run_dir}" "${etc_dir}"

### Vendor only, then /run over vendor, then /etc over /run over vendor, all
### for the same basename.

shared="50-nex-uapi-shared.quirks"
write_quirks "${vendor_dir}/${shared}"
run_validate
assert_exit 0
assert_contains "${vendor_dir}/${shared}"

write_quirks "${run_dir}/${shared}"
run_validate
assert_exit 0
assert_contains "${run_dir}/${shared}"
assert_not_contains "${vendor_dir}/${shared}"

write_quirks "${etc_dir}/${shared}"
run_validate
assert_exit 0
assert_contains "${etc_dir}/${shared}"
assert_not_contains "${run_dir}/${shared}"
assert_not_contains "${vendor_dir}/${shared}"

rm "${vendor_dir}/${shared}" "${run_dir}/${shared}" "${etc_dir}/${shared}"

### An empty higher-tier file masks a lower same-basename file. If the empty
### file were handed to the parser instead of masking, quirks_init_subsystem
### would fail (an empty .quirks file is a parse error), so this also
### asserts a clean exit, not only the absent log line.

mask_empty="60-nex-uapi-mask-empty.quirks"
write_quirks "${vendor_dir}/${mask_empty}"
run_validate
assert_exit 0
assert_contains "${vendor_dir}/${mask_empty}"

: > "${run_dir}/${mask_empty}"
run_validate
assert_exit 0
assert_not_contains "${run_dir}/${mask_empty}"
assert_not_contains "${vendor_dir}/${mask_empty}"

rm "${vendor_dir}/${mask_empty}" "${run_dir}/${mask_empty}"

### A /dev/null symlink at a higher tier masks a lower same-basename file the
### same way, even though the link itself is never opened for parsing.

mask_devnull="61-nex-uapi-mask-devnull.quirks"
write_quirks "${vendor_dir}/${mask_devnull}"
run_validate
assert_exit 0
assert_contains "${vendor_dir}/${mask_devnull}"

ln -s /dev/null "${etc_dir}/${mask_devnull}"
run_validate
assert_exit 0
assert_not_contains "${vendor_dir}/${mask_devnull}"
assert_not_contains "${etc_dir}/${mask_devnull}"

rm "${vendor_dir}/${mask_devnull}" "${etc_dir}/${mask_devnull}"

### Distinct basenames from different tiers all apply together.

write_quirks "${vendor_dir}/70-nex-uapi-vendor-only.quirks"
write_quirks "${run_dir}/71-nex-uapi-run-only.quirks"
write_quirks "${etc_dir}/72-nex-uapi-etc-only.quirks"
run_validate
assert_exit 0
assert_contains "${vendor_dir}/70-nex-uapi-vendor-only.quirks"
assert_contains "${run_dir}/71-nex-uapi-run-only.quirks"
assert_contains "${etc_dir}/72-nex-uapi-etc-only.quirks"

rm "${run_dir}/71-nex-uapi-run-only.quirks" "${etc_dir}/72-nex-uapi-etc-only.quirks"

### A missing /run and /etc tier is normal: only the vendor tier is
### required, so the same vendor-only file from above still loads cleanly
### once both other tiers are gone entirely.

rmdir "${run_dir}" "${etc_dir}"
run_validate
assert_exit 0
assert_contains "${vendor_dir}/70-nex-uapi-vendor-only.quirks"

rm "${vendor_dir}/70-nex-uapi-vendor-only.quirks"
mkdir -p "${run_dir}" "${etc_dir}"

### A completely absent (or empty) vendor tier is still an error, matching
### upstream's allow_empty_directory handling: it is the negative control
### that shows requiring /run and /etc to exist would have been the wrong
### fix, and that the required-vendor check still fires through the merge.

run_validate
assert_exit 1
assert_contains "${vendor_dir}: failed to find data files"

### LIBINPUT_QUIRKS_DIR still replaces the whole system search, including
### the required vendor-tier check: with the real vendor directory left
### empty (from the case above), the compiled smoke's default run must still
### report the vendor failure, and a run with LIBINPUT_QUIRKS_DIR pointing
### at a valid directory instead must not.

run_smoke
assert_contains "${vendor_dir}: failed to find data files"

override_dir="${work_dir}/libinput-uapi-quirks-dir-override"
mkdir -p "${override_dir}"
write_quirks "${override_dir}/80-nex-uapi-override.quirks"
run_smoke LIBINPUT_QUIRKS_DIR="${override_dir}"
assert_exit 0
assert_not_contains "${vendor_dir}: failed to find data files"
rm -rf "${override_dir}"

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'libinput UAPI smoke: the quirks reader merged /usr/share, /run, and /etc in order, masked same-basename files with both empty files and /dev/null symlinks, merged distinct basenames, tolerated a missing /run and /etc, required the vendor tier, and LIBINPUT_QUIRKS_DIR still replaced the whole system search\n'
