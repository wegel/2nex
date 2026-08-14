#!/bin/sh
set -eu

repo_root=$(git rev-parse --show-toplevel)
helper=${repo_root}/pkg/core/kernel/initramfs-populate-etc.sh
test_root=$(mktemp -d)
trap 'rm -rf "${test_root}"' EXIT HUP INT TERM

factory=${test_root}/factory
host=${test_root}/host
mkdir -p "${factory}/ssh" "${factory}/network" "${host}/ssh"
printf 'vendor hosts\n' > "${factory}/hosts"
printf 'vendor ssh\n' > "${factory}/ssh/sshd_config"
printf 'vendor network\n' > "${factory}/network/10-default.network"
printf 'host ssh\n' > "${host}/ssh/sshd_config"
: > "${host}/hosts"
ln -s /dev/null "${host}/network"
ln -s /proc/self/mounts "${factory}/mtab"
chmod 0640 "${factory}/ssh/sshd_config"

/bin/sh "${helper}" "${factory}" "${host}"

test ! -s "${host}/hosts"
test "$(cat "${host}/ssh/sshd_config")" = "host ssh"
test "$(readlink "${host}/network")" = /dev/null
test "$(readlink "${host}/mtab")" = /proc/self/mounts

printf 'new vendor file\n' > "${factory}/ssh/new.conf"
/bin/sh "${helper}" "${factory}" "${host}"

test "$(cat "${host}/ssh/new.conf")" = "new vendor file"
test "$(cat "${host}/ssh/sshd_config")" = "host ssh"
printf 'initramfs config population: ok\n'
