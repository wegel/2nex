#!/bin/sh
# Exercise the real, newly built libxml2 UAPI-config reader this package
# patches: xmlInitializeCatalog() (catalog.c), driven through the
# installed xmlcatalog program rather than a reimplemented catalog
# resolver. `xmlcatalog "" IDENTIFIER` calls xmlInitializeCatalog()
# directly (empty-string catalog specification is its documented shortcut
# for the default system catalog) and then resolves IDENTIFIER through
# the real xmlCatalogResolvePublic() reader.
#
# The caller supplies an isolated package output and build work directory;
# this script owns every absolute fixture path it creates
# (/usr/share/xml/catalog, /run/xml/catalog, /etc/xml/catalog) and refuses
# to run if one already exists.
set -eu

if [ "$#" -ne 2 ]; then
  printf '%s\n' \
    'usage: libxml2-uapi-check.sh OUT_DIR WORK_DIR' >&2
  exit 2
fi

out_dir=$1
work_dir=$2

xmlcatalog_bin="${out_dir}/usr/bin/xmlcatalog"

out_log="${work_dir}/libxml2-uapi-check.out"
err_log="${work_dir}/libxml2-uapi-check.err"

run_probe() {
  # $1 identifier to resolve, extra args ($2...) are extra env "NAME=value"
  # assignments applied only for this probe.
  identifier=$1
  shift
  set +e
  env "$@" LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "${xmlcatalog_bin}" "" "${identifier}" >"${out_log}" 2>"${err_log}"
  rc=$?
  set -e
}

assert_exit() {
  if [ "${rc}" -ne "$1" ]; then
    printf 'libxml2 UAPI smoke: expected exit %s, got %s\n' "$1" "${rc}" >&2
    printf 'stdout:\n' >&2; cat "${out_log}" >&2
    printf 'stderr:\n' >&2; cat "${err_log}" >&2
    exit 1
  fi
}
assert_stdout() {
  if ! grep -qF -- "$1" "${out_log}"; then
    printf 'libxml2 UAPI smoke: expected to find "%s" in stdout\n' "$1" >&2
    printf 'stdout:\n' >&2; cat "${out_log}" >&2
    exit 1
  fi
}
assert_stderr_empty() {
  if [ -s "${err_log}" ]; then
    printf 'libxml2 UAPI smoke: expected empty stderr, got:\n' >&2
    cat "${err_log}" >&2
    exit 1
  fi
}

vendor_dir=/usr/share/xml
run_dir=/run/xml
etc_dir=/etc/xml
env_catalog="${work_dir}/libxml2-uapi-env-catalog.xml"

cleanup_uapi_check() {
  rm -rf "${vendor_dir}" "${run_dir}" "${etc_dir}" "${env_catalog}"
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${vendor_dir}"
test ! -e "${run_dir}"
test ! -e "${etc_dir}"

write_catalog() {
  # $1 catalog file path  $2 publicId  $3 uri
  dir=$(dirname "$1")
  mkdir -p "${dir}"
  if [ ! -e "$1" ]; then
    cat > "$1" <<CATALOG_EOF
<?xml version="1.0"?>
<!DOCTYPE catalog PUBLIC "-//OASIS//DTD Entity Resolution XML Catalog V1.0//EN" "http://www.oasis-open.org/committees/entity/release/1.0/catalog.dtd">
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
</catalog>
CATALOG_EOF
  fi
  sed -i "s#</catalog>#<public publicId=\"$2\" uri=\"$3\"/>\n</catalog>#" "$1"
}

same_id="-//Nex UAPI//SAME PROBE//EN"
vendor_id="-//Nex UAPI//VENDOR ONLY//EN"
run_id="-//Nex UAPI//RUN ONLY//EN"
etc_id="-//Nex UAPI//ETC ONLY//EN"
env_id="-//Nex UAPI//ENV ONLY//EN"

### Vendor tier alone takes effect for the shared identifier.

write_catalog "${vendor_dir}/catalog" "${same_id}" vendor.dtd
run_probe "${same_id}"
assert_exit 0
assert_stdout vendor.dtd
assert_stderr_empty

### /run overrides the same identifier.

write_catalog "${run_dir}/catalog" "${same_id}" run.dtd
run_probe "${same_id}"
assert_exit 0
assert_stdout run.dtd
assert_stderr_empty

### /etc overrides both /run and the vendor tier.

write_catalog "${etc_dir}/catalog" "${same_id}" etc.dtd
run_probe "${same_id}"
assert_exit 0
assert_stdout etc.dtd
assert_stderr_empty

### Distinct identifiers from every tier all resolve, proving each tier
### is actually read rather than only the tier that wins the shared id.

write_catalog "${vendor_dir}/catalog" "${vendor_id}" vendor-only.dtd
write_catalog "${run_dir}/catalog" "${run_id}" run-only.dtd
write_catalog "${etc_dir}/catalog" "${etc_id}" etc-only.dtd
run_probe "${vendor_id}"
assert_exit 0
assert_stdout vendor-only.dtd
run_probe "${run_id}"
assert_exit 0
assert_stdout run-only.dtd
run_probe "${etc_id}"
assert_exit 0
assert_stdout etc-only.dtd

### XML_CATALOG_FILES replaces the whole compiled list: with all three
### tiers still present, an explicit XML_CATALOG_FILES catalog resolves
### its own identifier but not a tier identifier that the compiled
### default would otherwise reach.

cat > "${env_catalog}" <<CATALOG_EOF
<?xml version="1.0"?>
<!DOCTYPE catalog PUBLIC "-//OASIS//DTD Entity Resolution XML Catalog V1.0//EN" "http://www.oasis-open.org/committees/entity/release/1.0/catalog.dtd">
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
<public publicId="${env_id}" uri="env-only.dtd"/>
</catalog>
CATALOG_EOF
run_probe "${env_id}" "XML_CATALOG_FILES=${env_catalog}"
assert_exit 0
assert_stdout env-only.dtd
run_probe "${same_id}" "XML_CATALOG_FILES=${env_catalog}"
assert_exit 4
assert_stdout 'No entry for PUBLIC'

### Every tier absent produces no error and no warning on stderr; the
### identifier stays unresolved.

rm -rf "${vendor_dir}" "${run_dir}" "${etc_dir}"
run_probe "${same_id}"
assert_exit 4
assert_stdout 'No entry for PUBLIC'
assert_stderr_empty

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'libxml2 UAPI smoke: the default catalog reader selected vendor, then run, then etc for the same public identifier, resolved distinct identifiers from every tier, tolerated every tier absent with no stderr output, and XML_CATALOG_FILES replaced the whole compiled list\n'
