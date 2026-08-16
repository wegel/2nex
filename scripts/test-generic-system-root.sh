#!/usr/bin/env bash
# Check that a finished reusable Nex system contains no product test policy.
set -euo pipefail

root=${1:-}
root=$(realpath "$root")
factory="$root/usr/share/factory/etc"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

[[ -d "$root" ]] || fail "usage: $0 ROOT"
[[ -d "$factory" ]] || fail "factory /etc is missing: $factory"

for account_file in passwd group shadow; do
    [[ -s "$factory/$account_file" ]] || fail "factory $account_file is missing"
done

grep -q '^root:!\*:' "$factory/shadow" || fail 'root is not locked'
if grep -q 'testuser\|nex-test' "$factory/passwd" "$factory/group" "$factory/shadow"; then
    fail 'factory account files contain a test identity'
fi

while IFS=: read -r user_name _ user_id _; do
    [[ "$user_id" =~ ^[0-9]+$ ]] || continue
    if ((user_id >= 1000 && user_id < 65534)); then
        fail "factory passwd contains interactive account $user_name with UID $user_id"
    fi
done < "$factory/passwd"

for path in \
    "$factory/subuid" \
    "$factory/subgid" \
    "$factory/ssh/sshd_config" \
    "$factory/systemd/system/multi-user.target.wants/sshd.service" \
    "$factory/systemd/system/multi-user.target.wants/sshd-keygen.service" \
    "$root/usr/local/bin/accept-any-key" \
    "$root/usr/local/bin/nm-autoconnect" \
    "$root/usr/local/bin/nex-boot-dump" \
    "$root/home/testuser" \
    "$root/home/nex-test"
do
    if [[ -e "$path" || -L "$path" ]]; then
        fail "generic root contains $path"
    fi
done

if [[ -d "$factory/NetworkManager/system-connections" ]] &&
    find "$factory/NetworkManager/system-connections" -mindepth 1 -print -quit |
        grep -q .; then
    fail 'generic root contains a provisioned NetworkManager connection'
fi

for name in wegelnet accept-any-key nm-autoconnect nex-boot-dump home-testuser.conf; do
    if find "$factory" "$root/usr/local/bin" -name "*$name*" -print -quit 2>/dev/null |
        grep -q .; then
        fail "generic root contains product artifact $name"
    fi
done

unshare --user --map-root-user --root "$root" \
    /usr/bin/test -x /usr/bin/sshd || fail 'OpenSSH server is missing for later provisioning'
unshare --user --map-root-user --root "$root" \
    /usr/bin/test -s /usr/lib/ssh/sshd_config || fail 'OpenSSH vendor policy is missing'
unshare --user --map-root-user --root "$root" \
    /usr/bin/test -f /usr/lib/systemd/system/sshd.service || fail 'OpenSSH unit is missing'

sshd_output=$(mktemp)
host_key="$root/tmp/nex-generic-root-test-key"
for path in "$root/dev/null" "$root/etc/passwd" "$root/etc/group" "$root/etc/shadow"; do
    [[ ! -e "$path" && ! -L "$path" ]] || fail "stored deployment unexpectedly owns $path"
    mkdir -p "$(dirname "$path")"
    : > "$path"
done
cleanup() {
    rm -f \
        "$sshd_output" \
        "$host_key" \
        "$host_key.pub" \
        "$root/dev/null" \
        "$root/etc/passwd" \
        "$root/etc/group" \
        "$root/etc/shadow"
}
trap cleanup EXIT
ssh-keygen -q -t ed25519 -N '' -f "$host_key"
unshare --user --map-root-user --mount --pid --fork sh -c '
    mount --bind /dev/null "$1/dev/null"
    mount --bind "$1/usr/share/factory/etc/passwd" "$1/etc/passwd"
    mount --bind "$1/usr/share/factory/etc/group" "$1/etc/group"
    mount --bind "$1/usr/share/factory/etc/shadow" "$1/etc/shadow"
    chroot "$1" /usr/bin/sshd -T -h /tmp/nex-generic-root-test-key
' sh "$root" > "$sshd_output"
grep -qx 'permitemptypasswords no' "$sshd_output" ||
    fail 'OpenSSH vendor policy permits empty passwords'
grep -Eq '^permitrootlogin (without-password|prohibit-password)$' "$sshd_output" ||
    fail 'OpenSSH vendor policy permits root password login'
grep -qx 'authorizedkeyscommand none' "$sshd_output" ||
    fail 'OpenSSH vendor policy runs an authorized-key bypass'

printf 'PASS: finished reusable system has generic accounts, network, and SSH policy\n'
