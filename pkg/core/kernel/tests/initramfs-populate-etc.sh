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

mkdir -p "${factory}/pam.d" "${host}/pam.d"
printf '%s\n' \
    'NAME="nex"' \
    'PRETTY_NAME="nex Linux"' \
    'ID=nex' \
    'VERSION_ID=0.0.1' \
    > "${host}/os-release"
: > "${host}/fstab"
ln -s /nex/pkg/core/init/systemd/257.5/abcdef12/usr/share/factory/etc/issue \
    "${host}/issue"
ln -s /nex/pkg/core/init/systemd/257.5/abcdef12/usr/share/factory/etc/pam.d/system-auth \
    "${host}/pam.d/system-auth"
ln -s /usr/lib/pam.d/systemd-user "${host}/pam.d/systemd-user"
printf 'new machine policy\n' > "${factory}/pam.d/system-auth"
printf 'administrator locale\n' > "${host}/locale.conf"

/bin/sh "${helper}" "${factory}" "${host}"

test ! -s "${host}/hosts"
test "$(cat "${host}/ssh/sshd_config")" = "host ssh"
test "$(readlink "${host}/network")" = /dev/null
test "$(readlink "${host}/mtab")" = /proc/self/mounts
test ! -e "${host}/os-release"
test ! -e "${host}/fstab"
test ! -e "${host}/issue"
test ! -e "${host}/pam.d/systemd-user"
test "$(cat "${host}/pam.d/system-auth")" = "new machine policy"
test "$(cat "${host}/locale.conf")" = "administrator locale"

printf 'new vendor file\n' > "${factory}/ssh/new.conf"
/bin/sh "${helper}" "${factory}" "${host}"

test "$(cat "${host}/ssh/new.conf")" = "new vendor file"
test "$(cat "${host}/ssh/sshd_config")" = "host ssh"

printf 'NAME="administrator edition"\n' > "${host}/os-release"
/bin/sh "${helper}" "${factory}" "${host}"
test "$(cat "${host}/os-release")" = 'NAME="administrator edition"'
printf 'initramfs config population: ok\n'
