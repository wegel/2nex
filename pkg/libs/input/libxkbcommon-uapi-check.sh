#!/bin/sh
# Exercise the real, newly built and installed xkbcli-compile-keymap tool
# through xkb_context_include_path_append_default() (src/context.c), the
# exact function the UAPI config patch changes. With no --include argument,
# compile-keymap falls back to that function unmodified, so every compile
# below goes through the real default include path search, not a copied
# algorithm.
#
# xkbcli-compile-keymap --from-xkb reads a full keymap from stdin and prints
# the compiled result to stdout. Upstream's own main() returns that path's
# bool result directly as the process exit code without translating it to
# EXIT_SUCCESS/EXIT_FAILURE, so a successful compile exits 1 and a failed one
# exits 0; this script never asserts that exit code and instead reads the
# compiled keymap text (success) or the library's own ERROR-level log lines
# (failure) exactly as .agents/TESTING.md requires for a real reader.
#
# The driver keymap (KEYMAP_SOURCE) inlines its own keycodes/types/compat and
# only exercises the include search for xkb_symbols, via two bare-name
# includes: "nex-uapi-test" (bound to key <AE01>) and "nex-uapi-test-2"
# (bound to <AE02>). Each case below plants one or both basenames under a
# combination of the real vendor, /run, and /etc tiers and reads the
# resulting keysym for <AE01>/<AE02> out of the compiled text to prove which
# tier's file won.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates (the real
# /usr/share/X11/xkb, /run/xkb, and /etc/xkb trees) and refuses to run if one
# already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: libxkbcommon-uapi-check.sh OUT_DIR WORK_DIR KEYMAP_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
keymap_source=$3

bin="${out_dir}/usr/libexec/xkbcommon/xkbcli-compile-keymap"

home_dir="${work_dir}/xkb-uapi-home"
mkdir -p "${home_dir}"

vendor_dir=/usr/share/X11/xkb
run_dir=/run/xkb
etc_dir=/etc/xkb
extra_override_dir="${work_dir}/xkb-uapi-extra-override"
root_override_dir="${work_dir}/xkb-uapi-root-override"

cleanup_uapi_check() {
  rm -rf "${vendor_dir}" "${run_dir}" "${etc_dir}" \
         "${extra_override_dir}" "${root_override_dir}"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_dir}"
test ! -e "${run_dir}"
test ! -e "${etc_dir}"

stdout_log="${work_dir}/xkb-uapi-check-stdout.log"
stderr_log="${work_dir}/xkb-uapi-check-stderr.log"

run_compile() {
  set +e
  env -i \
    PATH="${PATH}" \
    HOME="${home_dir}" \
    LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "$@" \
    "${bin}" --from-xkb <"${keymap_source}" >"${stdout_log}" 2>"${stderr_log}"
  set -e
}

assert_stdout_contains() {
  if ! grep -qF -- "$1" "${stdout_log}"; then
    printf 'libxkbcommon UAPI smoke: expected to find "%s" in the compiled keymap\n' "$1" >&2
    cat "${stdout_log}" "${stderr_log}" >&2
    exit 1
  fi
}
assert_stdout_not_contains() {
  if grep -qF -- "$1" "${stdout_log}"; then
    printf 'libxkbcommon UAPI smoke: did not expect to find "%s" in the compiled keymap\n' "$1" >&2
    cat "${stdout_log}" "${stderr_log}" >&2
    exit 1
  fi
}
assert_stderr_empty() {
  if [ -s "${stderr_log}" ]; then
    printf 'libxkbcommon UAPI smoke: expected an empty stderr, got:\n' >&2
    cat "${stderr_log}" >&2
    exit 1
  fi
}
assert_stderr_contains() {
  if ! grep -qF -- "$1" "${stderr_log}"; then
    printf 'libxkbcommon UAPI smoke: expected to find "%s" on stderr\n' "$1" >&2
    cat "${stdout_log}" "${stderr_log}" >&2
    exit 1
  fi
}

write_symbols() {
  # $1 = directory, $2 = basename, $3 = key ("AE01" or "AE02"), $4 = keysym
  mkdir -p "$1/symbols"
  printf 'default\nxkb_symbols "basic" {\n    key <%s> { [ %s ] };\n};\n' \
    "$3" "$4" >"$1/symbols/$2"
}

### 1. Vendor only, with /etc/xkb and /run/xkb entirely absent. This is both
### the baseline vendor case and the "missing tier produces no error" case:
### xkb_context_include_path_append() records a missing directory in its
### separate failed-includes list instead of failing the context, so an
### absent /etc/xkb and /run/xkb must not appear as an error or a warning.

write_symbols "${vendor_dir}" nex-uapi-test AE01 F13
write_symbols "${vendor_dir}" nex-uapi-test-2 AE02 F19

run_compile
assert_stdout_contains 'F13'
assert_stdout_contains 'F19'
assert_stderr_empty

### 2. /run over vendor for "nex-uapi-test", while "nex-uapi-test-2" is only
### under vendor. Proves /run masks the same basename in vendor, and proves
### two distinct basenames resolve from two different tiers (/run and
### vendor) in the same compile.

write_symbols "${run_dir}" nex-uapi-test AE01 F14

run_compile
assert_stdout_contains 'F14'
assert_stdout_not_contains 'F13'
assert_stdout_contains 'F19'
assert_stderr_empty

### 3. /etc over /run over vendor for "nex-uapi-test". "nex-uapi-test-2"
### still resolves from vendor, untouched by the other two tiers filling in.

write_symbols "${etc_dir}" nex-uapi-test AE01 F15

run_compile
assert_stdout_contains 'F15'
assert_stdout_not_contains 'F14'
assert_stdout_not_contains 'F13'
assert_stdout_contains 'F19'
assert_stderr_empty

### 4. Add a /run copy of "nex-uapi-test-2" while /etc still owns
### "nex-uapi-test". Both basenames now resolve from a different tier
### (/etc and /run respectively) in one compile, and the vendor copies of
### both basenames are shadowed.

write_symbols "${run_dir}" nex-uapi-test-2 AE02 F16

run_compile
assert_stdout_contains 'F15'
assert_stdout_contains 'F16'
assert_stdout_not_contains 'F13'
assert_stdout_not_contains 'F19'
assert_stderr_empty

### 5. Remove /etc/xkb and /run/xkb again. Both basenames revert cleanly to
### the vendor tier and the compile still reports no error, confirming the
### quiet handling of a missing tier is not a one-time artifact of case 1.

rm -rf "${etc_dir}" "${run_dir}"

run_compile
assert_stdout_contains 'F13'
assert_stdout_contains 'F19'
assert_stderr_empty

### 6. XKB_CONFIG_EXTRA_PATH replaces only the administrator entry. Its
### fixture directory has no "nex-uapi-test-2", so that basename still falls
### through to the vendor tier exactly as upstream's single-entry override
### semantics require.

write_symbols "${extra_override_dir}" nex-uapi-test AE01 F17

run_compile XKB_CONFIG_EXTRA_PATH="${extra_override_dir}"
assert_stdout_contains 'F17'
assert_stdout_not_contains 'F13'
assert_stdout_contains 'F19'
assert_stderr_empty

### 7. XKB_CONFIG_ROOT replaces only the vendor entry, for every basename
### look up through it. Both compiled values must come from the override
### directory, not from the real vendor tier planted in case 1, which is
### left untouched on disk throughout.

write_symbols "${root_override_dir}" nex-uapi-test AE01 F18
write_symbols "${root_override_dir}" nex-uapi-test-2 AE02 F20

run_compile XKB_CONFIG_ROOT="${root_override_dir}"
assert_stdout_contains 'F18'
assert_stdout_contains 'F20'
assert_stdout_not_contains 'F13'
assert_stdout_not_contains 'F19'
assert_stderr_empty

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'libxkbcommon UAPI smoke: the default include path search read /etc/xkb, /run/xkb, and the vendor root in order, merged distinct basenames from different tiers, tolerated a missing /etc/xkb and /run/xkb, and XKB_CONFIG_EXTRA_PATH and XKB_CONFIG_ROOT each still overrode only their own entry\n'
