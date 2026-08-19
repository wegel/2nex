#!/bin/sh
# Exercise the real, newly built libssh through its three system configuration
# readers: ssh_options_parse_config() for the client config, ssh_bind_listen()
# (which itself calls ssh_bind_options_parse_config()) for the server config,
# and a real diffie-hellman-group-exchange handshake for the moduli file. The
# caller supplies an isolated package output and build work directory; this
# script owns every absolute fixture path it creates below /etc/ssh, /run/ssh,
# /usr/lib/ssh, /usr/share/ssh, and refuses to run if one already exists.
set -eu

if [ "$#" -ne 3 ]; then
  printf '%s\n' \
    'usage: libssh-uapi-config-check.sh OUT_DIR WORK_DIR SMOKE_SOURCE' >&2
  exit 2
fi

out_dir=$1
work_dir=$2
smoke_source=$3

libssh_so="${out_dir}/usr/lib/libssh.so.4"

# Point instructions: enchant2 lost four full builds to a smoke that
# referenced an internal symbol the shared library does not export. Check
# every symbol this smoke calls before compiling against it.
for sym in \
  ssh_new ssh_free ssh_options_set ssh_options_get \
  ssh_options_parse_config ssh_get_error ssh_string_free_char \
  ssh_bind_new ssh_bind_free ssh_bind_options_set ssh_bind_listen \
  ssh_bind_get_fd ssh_bind_accept_fd ssh_pki_generate ssh_key_free \
  ssh_handle_key_exchange ssh_connect ssh_disconnect; do
  if ! nm -D "${libssh_so}" | grep -qE "[[:space:]]T[[:space:]]${sym}(@|\$)"; then
    printf 'libssh-uapi-config-check: %s is not an exported symbol of %s\n' \
      "${sym}" "${libssh_so}" >&2
    nm -D "${libssh_so}" | grep "${sym}" >&2 || true
    exit 1
  fi
done

smoke_bin="${work_dir}/libssh-uapi-config-smoke"
gcc \
  -Wall -Wextra -Werror \
  -I "${out_dir}/usr/include" \
  "${smoke_source}" \
  -L"${out_dir}/usr/lib" \
  -lssh \
  -o "${smoke_bin}"

fixture_home="${work_dir}/libssh-uapi-config-home"
mkdir -p "${fixture_home}/.ssh"

run_smoke() {
  env -i \
    PATH="${PATH}" \
    HOME="${fixture_home}" \
    LD_LIBRARY_PATH="${out_dir}/usr/lib" \
    "${smoke_bin}" "$@"
}

assert_eq() {
  got=$1
  want=$2
  label=$3
  if [ "${got}" != "${want}" ]; then
    printf 'libssh UAPI smoke (%s): got "%s", want "%s"\n' \
      "${label}" "${got}" "${want}" >&2
    exit 1
  fi
}

etc_dir=/etc/ssh
run_dir=/run/ssh
vendor_dir=/usr/lib/ssh
vendor_share_dir=/usr/share/ssh

cleanup_uapi_check() {
  rm -f "${etc_dir}/ssh_config" "${run_dir}/ssh_config" \
    "${vendor_dir}/ssh_config"
  rm -f "${etc_dir}/libssh_server_config" "${run_dir}/libssh_server_config" \
    "${vendor_dir}/libssh_server_config"
  rm -f "${etc_dir}/moduli" "${run_dir}/moduli" "${vendor_share_dir}/moduli"
  rmdir "${etc_dir}" "${run_dir}" "${vendor_dir}" "${vendor_share_dir}" \
    2>/dev/null || true
}
trap cleanup_uapi_check EXIT HUP INT TERM

test ! -e "${etc_dir}"
test ! -e "${run_dir}"
test ! -e "${vendor_dir}"
test ! -e "${vendor_share_dir}"
mkdir -p "${etc_dir}" "${run_dir}" "${vendor_dir}" "${vendor_share_dir}"

### Client ssh_config: administrator (/etc), then runtime (/run), then vendor
### tier, with the user's own ~/.ssh/config outranking every system tier.

got=$(run_smoke client-config)
assert_eq "${got}" "NONE" "client-config: every tier absent"

printf 'ProxyCommand true vendor\n' > "${vendor_dir}/ssh_config"
got=$(run_smoke client-config)
assert_eq "${got}" "true vendor" "client-config: vendor only"

printf 'ProxyCommand true run\n' > "${run_dir}/ssh_config"
got=$(run_smoke client-config)
assert_eq "${got}" "true run" "client-config: run beats vendor"

printf 'ProxyCommand true etc\n' > "${etc_dir}/ssh_config"
got=$(run_smoke client-config)
assert_eq "${got}" "true etc" "client-config: etc beats run and vendor"

printf 'ProxyCommand true user\n' > "${fixture_home}/.ssh/config"
got=$(run_smoke client-config)
assert_eq "${got}" "true user" "client-config: user beats every system tier"

rm "${fixture_home}/.ssh/config"
rm "${etc_dir}/ssh_config" "${run_dir}/ssh_config" "${vendor_dir}/ssh_config"

### Server (bind) config: vendor tier applied first, then runtime, then
### administrator (/etc) last, so /etc overrides last rather than first (see
### the precedence comment in the patched ssh_bind_options_parse_config()).
### A pre-set application port survives untouched when every tier is absent.

got=$(run_smoke bind-listen 13199)
assert_eq "${got}" "13199" "bind-listen: every tier absent, app value kept"

printf 'Port 13100\n' > "${vendor_dir}/libssh_server_config"
got=$(run_smoke bind-listen -)
assert_eq "${got}" "13100" "bind-listen: vendor only"

printf 'Port 13101\n' > "${run_dir}/libssh_server_config"
got=$(run_smoke bind-listen -)
assert_eq "${got}" "13101" "bind-listen: run beats vendor"

printf 'Port 13102\n' > "${etc_dir}/libssh_server_config"
got=$(run_smoke bind-listen -)
assert_eq "${got}" "13102" "bind-listen: etc beats run and vendor"

rm "${etc_dir}/libssh_server_config" "${run_dir}/libssh_server_config" \
  "${vendor_dir}/libssh_server_config"

### Moduli file: first-existing-file selection, not a merge. A tier that
### exists but is empty has zero usable entries, so the server fails the DH
### group exchange instead of silently falling through to a lower tier; a
### missing tier is skipped and the client falls back to a built-in group.

write_group14() {
  printf '20240101000000 2 4 100 2047 02 %s\n' \
    'FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74020BBEA63B139B22514A08798E3404DDEF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7EDEE386BFB5A899FA5AE9F24117C4B1FE649286651ECE45B3DC2007CB8A163BF0598DA48361C55D39A69163FA8FD24CF5F83655D23DCA3AD961C62F356208552BB9ED529077096966D670C354E4ABC9804F1746C08CA18217C32905E462E36CE3BE39E772C180E86039B2783A2EC07A28FB5C55DF06F4C52C9DE2BCBF6955817183995497CEA956AE515D2261898FA051015728E5A8AACAA68FFFFFFFFFFFFFFFF' \
    > "$1"
}

got=$(run_smoke moduli)
assert_eq "${got}" "CONNECT_OK" "moduli: every tier absent falls back"

: > "${vendor_share_dir}/moduli"
got=$(run_smoke moduli)
assert_eq "${got}" "CONNECT_FAIL" "moduli: empty vendor tier takes effect"

write_group14 "${run_dir}/moduli"
got=$(run_smoke moduli)
assert_eq "${got}" "CONNECT_OK" "moduli: run overrides vendor"

: > "${etc_dir}/moduli"
got=$(run_smoke moduli)
assert_eq "${got}" "CONNECT_FAIL" "moduli: etc overrides run and vendor"

rm "${etc_dir}/moduli"
got=$(run_smoke moduli)
assert_eq "${got}" "CONNECT_OK" "moduli: run resolves once etc is gone"

rm "${run_dir}/moduli" "${vendor_share_dir}/moduli"

cleanup_uapi_check
trap - EXIT HUP INT TERM

printf 'libssh UAPI smoke: client config, bind config, and moduli selection all honoured /etc, /run, and vendor in order, and the user config still outranked every system tier\n'
