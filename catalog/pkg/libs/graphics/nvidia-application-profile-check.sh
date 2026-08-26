#!/bin/sh
# Verify the proprietary NVIDIA application-profile path repair structurally
# and through the real GLX parser. The caller supplies an isolated package
# output and build work directory; the test owns every absolute fixture it
# creates and refuses to overwrite pre-existing paths.
set -eu

if [ "$#" -ne 4 ]; then
  printf '%s\n' \
    'usage: nvidia-application-profile-check.sh OUT_DIR VERSION SMOKE_SOURCE WORK_DIR' >&2
  exit 2
fi

out_dir=$1
version=$2
smoke_source=$3
work_dir=$4

python3 - "${out_dir}" "${version}" <<'PY'
import sys

out_dir, version = sys.argv[1:]
old = f"/usr/share/nvidia/nvidia-application-profiles-{version}-rc".encode()
new = b"/run/nvidia/nvidia-application-profiles-rc.d"
libs = [
    "libcuda",
    "libnvidia-eglcore",
    "libnvidia-glcore",
    "libnvidia-glsi",
    "libnvidia-opencl",
    "libnvidia-vksc-core",
]
for lib in libs:
    path = f"{out_dir}/usr/lib/{lib}.so.{version}"
    with open(path, "rb") as handle:
        data = handle.read()
    if old in data:
        sys.exit(f"{path}: old versioned-vendor path table entry still present")
    if data.count(new) != 1:
        sys.exit(f"{path}: runtime path table entry is not present exactly once")
PY
printf 'nvidia build: verified six application-profile path tables\n'

test "$(readlink "${out_dir}/usr/share/nvidia/nvidia-application-profiles-rc")" \
  = "nvidia-application-profiles-${version}-rc"
printf 'nvidia build: verified stable vendor profile link\n'

profile_smoke="${work_dir}/nvidia-application-profile-smoke"
gcc \
  -Wall -Wextra -Werror \
  "${smoke_source}" \
  -ldl \
  -o "${profile_smoke}"

admin_main=/etc/nvidia/nvidia-application-profiles-rc
admin_dir=/etc/nvidia/nvidia-application-profiles-rc.d
admin_dir_marker="${admin_dir}/zz-admin-dir-marker"
runtime_dir=/run/nvidia/nvidia-application-profiles-rc.d
runtime_marker="${runtime_dir}/zz-runtime-marker"
runtime_a="${runtime_dir}/a-first-marker"
runtime_b="${runtime_dir}/b-second-marker"
vendor_dir=/usr/share/nvidia
vendor_versioned="${vendor_dir}/nvidia-application-profiles-${version}-rc"
vendor_stable="${vendor_dir}/nvidia-application-profiles-rc"
profile_home="${work_dir}/nvidia-profile-smoke-home"
profile_log="${work_dir}/nvidia-profile-smoke.log"

for fixture_path in \
  /etc/nvidia "${admin_main}" "${admin_dir}" "${admin_dir_marker}" \
  /run/nvidia "${runtime_dir}" "${runtime_marker}" "${runtime_a}" "${runtime_b}" \
  "${vendor_dir}" "${vendor_versioned}" "${vendor_stable}"; do
  if [ -e "${fixture_path}" ] || [ -L "${fixture_path}" ]; then
    printf '%s\n' \
      "profile smoke: fixture path already exists, refusing to run: ${fixture_path}" >&2
    exit 1
  fi
done

cleanup_profile_smoke() {
  rm -f "${runtime_a}" "${runtime_b}" "${runtime_marker}"
  [ -d "${runtime_dir}" ] && rmdir "${runtime_dir}"
  [ -d /run/nvidia ] && rmdir /run/nvidia
  rm -f "${admin_dir_marker}"
  [ -d "${admin_dir}" ] && rmdir "${admin_dir}"
  rm -f "${admin_main}"
  [ -d /etc/nvidia ] && rmdir /etc/nvidia
  rm -f "${vendor_versioned}" "${vendor_stable}"
  [ -d "${vendor_dir}" ] && rmdir "${vendor_dir}"
  rm -rf "${profile_home}"
}
trap cleanup_profile_smoke EXIT

mkdir /etc/nvidia
mkdir "${vendor_dir}"
cp "${out_dir}/usr/share/nvidia/nvidia-application-profiles-${version}-rc" \
  "${vendor_versioned}"
cp -a "${out_dir}/usr/share/nvidia/nvidia-application-profiles-rc" \
  "${vendor_stable}"

write_profile() {
  mkdir -p "$(dirname "$1")"
  cat > "$1" <<PROFILE
{
    "rules": [ { "pattern": [], "profile": "$2" } ],
    "profiles": [ { "name": "$2", "settings": [ "GLConformantBlitFramebufferScissor", false ] } ]
}
PROFILE
}

fail_profile_smoke() {
  printf '%s\n' "$1" >&2
  cat "${profile_log}" >&2
  exit 1
}

run_profile_smoke() {
  rm -rf "${profile_home}"
  mkdir -p "${profile_home}/.nv"
  if ! env HOME="${profile_home}" __GL_APPLICATION_PROFILE_LOG=1 \
      LD_LIBRARY_PATH="${out_dir}/usr/lib" \
      "${profile_smoke}" "${out_dir}/usr/lib/libGLX_nvidia.so.${version}" \
      >"${profile_log}" 2>&1; then
    fail_profile_smoke "profile smoke: proprietary GLX parser failed to start"
  fi
}

# With no administrator or runtime file, the stable vendor link must win.
run_profile_smoke
grep -qF "Parsing file ${vendor_stable}" "${profile_log}" \
  || fail_profile_smoke "profile smoke: vendor fallback did not parse"
if grep -qE "Parsing file (${admin_main}|${runtime_dir})" "${profile_log}"; then
  fail_profile_smoke "profile smoke: vendor-only case parsed an unexpected file"
fi

# A runtime directory entry must precede the vendor file.
write_profile "${runtime_marker}" runtime-marker
run_profile_smoke
runtime_line=$(grep -nE "Parsing file ${runtime_dir}/+zz-runtime-marker" "${profile_log}" \
  | head -n1 | cut -d: -f1)
vendor_line=$(grep -nF "Parsing file ${vendor_stable}" "${profile_log}" \
  | head -n1 | cut -d: -f1)
if [ -z "${runtime_line:-}" ] || [ -z "${vendor_line:-}" ] \
    || [ "${runtime_line}" -ge "${vendor_line}" ]; then
  fail_profile_smoke "profile smoke: runtime directory did not parse before the vendor file"
fi

# Both administrator entries must precede the runtime directory.
write_profile "${admin_dir_marker}" admin-dir-marker
write_profile "${admin_main}" admin-main-marker
run_profile_smoke
main_line=$(grep -nF "Parsing file ${admin_main}" "${profile_log}" \
  | head -n1 | cut -d: -f1)
dir_line=$(grep -nE "Parsing file ${admin_dir}/+zz-admin-dir-marker" "${profile_log}" \
  | head -n1 | cut -d: -f1)
runtime_line=$(grep -nE "Parsing file ${runtime_dir}/+zz-runtime-marker" "${profile_log}" \
  | head -n1 | cut -d: -f1)
if [ -z "${main_line:-}" ] || [ -z "${dir_line:-}" ] \
    || [ -z "${runtime_line:-}" ] || [ "${main_line}" -ge "${dir_line}" ] \
    || [ "${dir_line}" -ge "${runtime_line}" ]; then
  fail_profile_smoke "profile smoke: administrator entries did not both precede runtime"
fi

# Entries inside one directory must load in alphanumeric order.
rm -f "${admin_main}" "${admin_dir_marker}" "${runtime_marker}"
write_profile "${runtime_a}" a-first-marker
write_profile "${runtime_b}" b-second-marker
run_profile_smoke
a_line=$(grep -nE "Parsing file ${runtime_dir}/+a-first-marker" "${profile_log}" \
  | head -n1 | cut -d: -f1)
b_line=$(grep -nE "Parsing file ${runtime_dir}/+b-second-marker" "${profile_log}" \
  | head -n1 | cut -d: -f1)
if [ -z "${a_line:-}" ] || [ -z "${b_line:-}" ] \
    || [ "${a_line}" -ge "${b_line}" ]; then
  fail_profile_smoke "profile smoke: runtime entries did not load in alphanumeric order"
fi

# Removing the runtime directory must restore vendor fallback.
rm -f "${runtime_a}" "${runtime_b}"
rmdir "${runtime_dir}"
rmdir /run/nvidia
run_profile_smoke
grep -qF "Parsing file ${vendor_stable}" "${profile_log}" \
  || fail_profile_smoke "profile smoke: vendor fallback did not run after runtime removal"
if grep -qF "Parsing file ${runtime_dir}" "${profile_log}"; then
  fail_profile_smoke "profile smoke: removed runtime directory was still parsed"
fi

cleanup_profile_smoke
trap - EXIT
printf 'nvidia application-profile smoke: PASS\n'
