#!/bin/sh
set -eu

# Scenario: a Nex root runs glibc's ldd against a dynamically linked command.
# ldd invokes /lib64/ld-linux-x86-64.so.2 as a command instead of asking the
# kernel to use it as PT_INTERP.
#
# Wanted behavior: nex-ld-shim finds the command's package capsule, executes
# that capsule's real loader, and prints its resolved libc dependency.

root=${1:?usage: check-nex-ld-shim-direct.sh ROOTFS}
output=$(
    unshare --user --map-root-user --mount --pid --fork \
        chroot "${root}" /usr/bin/ldd /usr/bin/nex
)

printf '%s\n' "${output}"
printf '%s\n' "${output}" | grep -Fq 'libc.so.6 =>'

# Scenario: ldd gives the direct loader a relative target path.
#
# Wanted behavior: the shim resolves that path from the process working
# directory and does not loop while looking for the package capsule.
relative_output=$(
    unshare --user --map-root-user --mount --pid --fork \
        chroot "${root}" /usr/bin/bash -c \
        'cd /usr/bin && /usr/bin/ldd ./nex'
)
printf '%s\n' "${relative_output}"
printf '%s\n' "${relative_output}" | grep -Fq 'libc.so.6 =>'

# Scenario: glibc's ldd asks the shim to verify a shell script rather than an
# ELF executable.
#
# Wanted behavior: no non-ELF loader argument replaces the resolved shim path.
# The real loader reports the script in the same way it does outside Nex.
set +e
script_output=$(
    unshare --user --map-root-user --mount --pid --fork \
        chroot "${root}" /usr/bin/ldd /usr/bin/ldd 2>&1
)
script_status=$?
set -e

printf '%s\n' "${script_output}"
test "${script_status}" -ne 0
printf '%s\n' "${script_output}" | grep -Fq 'not a dynamic executable'
if printf '%s\n' "${script_output}" | grep -Fq 'nex-ld-shim:'; then
    exit 1
fi

# Scenario: an ELF outside every package capsule receives a capsule path as an
# ordinary program argument.
#
# Wanted behavior: PT_INTERP mode never mistakes that argument for a direct
# loader target. The shim rejects the outside binary instead of executing the
# argument through another package's loader.
mkdir -p "${root}/usr/local/bin"
cp -L "${root}/usr/bin/nex" "${root}/usr/local/bin/nex-outside"
set +e
outside_output=$(
    unshare --user --map-root-user --mount --pid --fork \
        chroot "${root}" /usr/local/bin/nex-outside /usr/bin/nex 2>&1
)
outside_status=$?
set -e
rm "${root}/usr/local/bin/nex-outside"

printf '%s\n' "${outside_output}"
test "${outside_status}" -ne 0
printf '%s\n' "${outside_output}" | grep -Fq '.nex-app-root not found for'
