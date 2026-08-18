#!/bin/sh
# Exercise the real, newly built libvulkan through vkCreateInstance(), which
# always runs the default driver, implicit-layer, and vk_loader_settings.json
# searches before it can tell whether a usable driver exists. With
# VK_LOADER_DEBUG=all set, the real readers log the absolute path of every
# manifest and settings file each search selected, which this script inspects
# instead of needing a working ICD. The caller supplies an isolated package
# output and build work directory; this script owns every absolute fixture
# path it creates and refuses to run if one already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: vulkan-loader-uapi-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

smoke_bin="${work_dir}/vulkan-loader-uapi-smoke"
gcc \
  -Wall -Wextra -Werror \
  "${smoke_source}" \
  -L"${out_dir}/usr/lib" \
  -lvulkan \
  -o "${smoke_bin}"

smoke_log="${work_dir}/vulkan-loader-uapi-smoke.log"
empty_home="${work_dir}/vulkan-loader-uapi-smoke-home"
mkdir -p "${empty_home}"

run_smoke() {
  : > "${smoke_log}"
  # The smoke binary itself always exits 0 regardless of the VkResult it
  # observed; a nonzero exit here means the process could not even start
  # (for example a dynamic linker failure), which must fail the build.
  env -i \
    PATH="${PATH}" \
    HOME="${empty_home}" \
    XDG_CONFIG_HOME="${empty_home}/xdg-config" \
    XDG_DATA_HOME="${empty_home}/xdg-data" \
    LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    VK_LOADER_DEBUG=all \
    "$@" \
    "${smoke_bin}" >>"${smoke_log}" 2>&1
}

assert_contains() {
  if ! grep -qF -- "$1" "${smoke_log}"; then
    printf 'vulkan-loader UAPI smoke: expected to find "%s" in the smoke log\n' "$1" >&2
    cat "${smoke_log}" >&2
    exit 1
  fi
}
assert_not_contains() {
  if grep -qF -- "$1" "${smoke_log}"; then
    printf 'vulkan-loader UAPI smoke: did not expect to find "%s" in the smoke log\n' "$1" >&2
    cat "${smoke_log}" >&2
    exit 1
  fi
}
# Assert that the first log line naming entry one ($1) comes before the first
# log line naming entry two ($2), pinning caller order rather than only set
# membership.
assert_order() {
  first_line=$(grep -nF -- "$1" "${smoke_log}" | head -n1 | cut -d: -f1)
  second_line=$(grep -nF -- "$2" "${smoke_log}" | head -n1 | cut -d: -f1)
  if [ -z "${first_line:-}" ] || [ -z "${second_line:-}" ] || [ "${first_line}" -ge "${second_line}" ]; then
    printf 'vulkan-loader UAPI smoke: expected "%s" before "%s" in the smoke log\n' "$1" "$2" >&2
    cat "${smoke_log}" >&2
    exit 1
  fi
}

write_driver_json() {
  library_path=${2:-libplaceholder.so}
  printf '{\n  "file_format_version" : "1.0.0",\n  "ICD" : {\n    "library_path" : "%s",\n    "api_version" : "1.0.0"\n  }\n}\n' \
    "${library_path}" > "$1"
}
write_layer_json() {
  printf '{\n  "file_format_version" : "1.2.1",\n  "layer" : {\n    "name" : "VK_LAYER_nex_placeholder",\n    "type" : "GLOBAL",\n    "library_path" : "libplaceholder.so",\n    "api_version" : "1.0.0",\n    "implementation_version" : "1"\n  }\n}\n' > "$1"
}
write_settings_json() {
  # Valid JSON missing the required file_format_version key: the real
  # settings reader logs the exact winning path when this field is absent.
  printf '{}\n' > "$1"
}

drv_etc=/etc/vulkan/icd.d
drv_run=/run/vulkan/icd.d
drv_share=/usr/share/vulkan/icd.d
lay_etc=/etc/vulkan/implicit_layer.d
lay_run=/run/vulkan/implicit_layer.d
lay_share=/usr/share/vulkan/implicit_layer.d
set_etc=/etc/vulkan/loader_settings.d
set_run=/run/vulkan/loader_settings.d
set_share=/usr/share/vulkan/loader_settings.d

cleanup_uapi_check() {
  rm -f "${drv_etc}"/*.json "${drv_run}"/*.json "${drv_share}"/*.json
  rm -f "${lay_etc}"/*.json "${lay_run}"/*.json "${lay_share}"/*.json
  rm -f "${set_etc}"/*.json "${set_run}"/*.json "${set_share}"/*.json
  # /usr/share/vulkan itself belongs to the vulkan-headers dependency (it
  # already carries registry/ content), so remove only the subdirectories
  # this check created, plus /etc/vulkan and /run/vulkan, which it owns
  # entirely.
  rmdir "${drv_etc}" "${drv_run}" "${drv_share}" \
    "${lay_etc}" "${lay_run}" "${lay_share}" \
    "${set_etc}" "${set_run}" "${set_share}" \
    /etc/vulkan /run/vulkan 2>/dev/null || true
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e /etc/vulkan
test ! -e /run/vulkan
test ! -e "${drv_share}"
test ! -e "${lay_share}"
test ! -e "${set_share}"
mkdir -p "${drv_etc}" "${drv_run}" "${drv_share}" \
  "${lay_etc}" "${lay_run}" "${lay_share}" \
  "${set_etc}" "${set_run}" "${set_share}"

### Default driver search: /etc, then /run, then /usr/share, basename-masked.

# Vendor-only default driver discovery.
write_driver_json "${drv_share}/10-shared.json"
run_smoke
assert_contains "${drv_share}/10-shared.json"

# Runtime beats vendor for the same basename.
write_driver_json "${drv_run}/10-shared.json"
run_smoke
assert_contains "${drv_run}/10-shared.json"
assert_not_contains "${drv_share}/10-shared.json"

# Administrator beats runtime beats vendor for the same basename.
write_driver_json "${drv_etc}/10-shared.json"
run_smoke
assert_contains "${drv_etc}/10-shared.json"
assert_not_contains "${drv_run}/10-shared.json"
assert_not_contains "${drv_share}/10-shared.json"

# An empty administrator file still masks the lower runtime and vendor files
# with the same basename.
: > "${drv_etc}/10-shared.json"
run_smoke
assert_contains "${drv_etc}/10-shared.json"
assert_not_contains "${drv_run}/10-shared.json"
assert_not_contains "${drv_share}/10-shared.json"

# An empty runtime file still masks the lower vendor file once the
# administrator file is gone.
rm "${drv_etc}/10-shared.json"
: > "${drv_run}/10-shared.json"
run_smoke
assert_contains "${drv_run}/10-shared.json"
assert_not_contains "${drv_share}/10-shared.json"

# A symlink to /dev/null at the administrator tier masks the vendor file the
# same way, even though it fails to parse once selected.
rm "${drv_run}/10-shared.json"
ln -s /dev/null "${drv_etc}/10-shared.json"
run_smoke
assert_contains "${drv_etc}/10-shared.json"
assert_not_contains "${drv_share}/10-shared.json"
rm "${drv_etc}/10-shared.json" "${drv_share}/10-shared.json"

# Distinct basenames across all three tiers all merge together.
write_driver_json "${drv_etc}/10-etc.json"
write_driver_json "${drv_run}/20-run.json"
write_driver_json "${drv_share}/30-share.json"
run_smoke
assert_contains "${drv_etc}/10-etc.json"
assert_contains "${drv_run}/20-run.json"
assert_contains "${drv_share}/30-share.json"
rm "${drv_etc}/10-etc.json" "${drv_run}/20-run.json" "${drv_share}/30-share.json"

### Default layer search shares the same reader: prove implicit_layer.d gets
### the same /etc, then /run, then /usr/share basename-masked priority,
### incrementally, the same way the driver tiers were proven above.

write_layer_json "${lay_share}/10-shared.json"
run_smoke
assert_contains "${lay_share}/10-shared.json"

write_layer_json "${lay_run}/10-shared.json"
run_smoke
assert_contains "${lay_run}/10-shared.json"
assert_not_contains "${lay_share}/10-shared.json"

write_layer_json "${lay_etc}/10-shared.json"
run_smoke
assert_contains "${lay_etc}/10-shared.json"
assert_not_contains "${lay_run}/10-shared.json"
assert_not_contains "${lay_share}/10-shared.json"

rm "${lay_etc}/10-shared.json" "${lay_run}/10-shared.json" "${lay_share}/10-shared.json"

### VK_DRIVER_FILES stays on upstream's exact, unmasked, ordered behavior:
### two explicit entries with the same basename both load instead of the
### first one shadowing the second.

override_dir_one="${work_dir}/vulkan-uapi-override-one"
override_dir_two="${work_dir}/vulkan-uapi-override-two"
mkdir -p "${override_dir_one}" "${override_dir_two}"
write_driver_json "${override_dir_one}/10-shared.json" libnex-override-one.so
write_driver_json "${override_dir_two}/10-shared.json" libnex-override-two.so
run_smoke VK_DRIVER_FILES="${override_dir_one}/10-shared.json:${override_dir_two}/10-shared.json"
assert_contains "${override_dir_one}/10-shared.json"
assert_contains "${override_dir_two}/10-shared.json"
assert_order "${override_dir_one}/10-shared.json" "${override_dir_two}/10-shared.json"
assert_contains 'Searching for ICD drivers named libnex-override-one.so'
assert_contains 'Searching for ICD drivers named libnex-override-two.so'
assert_order 'Searching for ICD drivers named libnex-override-one.so' \
  'Searching for ICD drivers named libnex-override-two.so'

### VK_ADD_DRIVER_FILES is additive, not an override: every entry loads even
### when two additive entries share a basename, and an additive entry still
### masks a same-named implicit system file.

additive_dir_one="${work_dir}/vulkan-uapi-additive-one"
additive_dir_two="${work_dir}/vulkan-uapi-additive-two"
mkdir -p "${additive_dir_one}" "${additive_dir_two}"
write_driver_json "${additive_dir_one}/10-shared.json" libnex-additive-one.so
write_driver_json "${additive_dir_two}/10-shared.json" libnex-additive-two.so
write_driver_json "${drv_share}/10-shared.json" libnex-masked-vendor.so
run_smoke VK_ADD_DRIVER_FILES="${additive_dir_one}/10-shared.json:${additive_dir_two}/10-shared.json"
assert_contains "${additive_dir_one}/10-shared.json"
assert_contains "${additive_dir_two}/10-shared.json"
assert_order "${additive_dir_one}/10-shared.json" "${additive_dir_two}/10-shared.json"
assert_not_contains "${drv_share}/10-shared.json"
assert_contains 'Searching for ICD drivers named libnex-additive-one.so'
assert_contains 'Searching for ICD drivers named libnex-additive-two.so'
assert_order 'Searching for ICD drivers named libnex-additive-one.so' \
  'Searching for ICD drivers named libnex-additive-two.so'
assert_not_contains libnex-masked-vendor.so
rm "${drv_share}/10-shared.json"

### vk_loader_settings.json: vendor only, then runtime over vendor, then
### administrator over runtime over vendor. XDG_DATA_DIRS is left unset for
### these three cases so the ordinary fallback ("/usr/local/share:/usr/share")
### exercises check_if_settings_path_exists()'s multi-segment parsing fix:
### /usr/share is the second, non-first segment of that fallback string.

write_settings_json "${set_share}/vk_loader_settings.json"
run_smoke
assert_contains "${set_share}/vk_loader_settings.json"

write_settings_json "${set_run}/vk_loader_settings.json"
run_smoke
assert_contains "${set_run}/vk_loader_settings.json"
assert_not_contains "${set_share}/vk_loader_settings.json"

write_settings_json "${set_etc}/vk_loader_settings.json"
run_smoke
assert_contains "${set_etc}/vk_loader_settings.json"
assert_not_contains "${set_run}/vk_loader_settings.json"
assert_not_contains "${set_share}/vk_loader_settings.json"

rm "${set_etc}/vk_loader_settings.json" "${set_run}/vk_loader_settings.json"

# Pin the segment-parsing fix directly with an explicit, non-default
# two-entry XDG_DATA_DIRS: a nonexistent first entry, then /usr/share as the
# second entry. A regression of the off-by-one would glue a stray leading
# separator onto this second segment and make it unreachable regardless of
# which strings happen to appear in the fallback constant.
nonexistent_data_dir="${work_dir}/vulkan-uapi-nonexistent-xdg-data-dir"
run_smoke XDG_DATA_DIRS="${nonexistent_data_dir}:/usr/share"
assert_contains "${set_share}/vk_loader_settings.json"
rm "${set_share}/vk_loader_settings.json"

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'vulkan-loader UAPI smoke: default driver, implicit-layer, and vk_loader_settings.json searches honoured /etc, /run, /usr/share in order; VK_ADD_DRIVER_FILES stayed additive and masked the vendor tier; VK_DRIVER_FILES stayed unmasked\n'
