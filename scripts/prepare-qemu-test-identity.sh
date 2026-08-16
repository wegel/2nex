#!/bin/sh
# Add disposable SSH access and an optional desktop user to a QEMU var tree.
set -eu

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

set_ownership() {
    var_root=$1

    chown 0:0 \
        "$var_root/etc/passwd" \
        "$var_root/etc/group" \
        "$var_root/etc/shadow" \
        "$var_root/etc/ssh/sshd_config"
    chmod 0644 "$var_root/etc/passwd" "$var_root/etc/group" "$var_root/etc/ssh/sshd_config"
    chmod 0600 "$var_root/etc/shadow"

    if [ -f "$var_root/home/nex-test/.ssh/authorized_keys" ]; then
        chown -R 1000:1000 "$var_root/home/nex-test"
        chmod 0700 "$var_root/home/nex-test/.ssh"
        chmod 0600 "$var_root/home/nex-test/.ssh/authorized_keys"
    fi
    if [ -f "$var_root/root/.ssh/authorized_keys" ]; then
        chown -R 0:0 "$var_root/root/.ssh"
        chmod 0700 "$var_root/root/.ssh"
        chmod 0600 "$var_root/root/.ssh/authorized_keys"
    fi
}

copy_accounts() {
    factory_etc=$1
    var_root=$2
    ssh_user=$3

    for name in passwd group shadow; do
        [ -s "$factory_etc/$name" ] || die "factory account file missing: $factory_etc/$name"
        cp "$factory_etc/$name" "$var_root/etc/$name"
    done
    grep -q '^root:!\*:' "$var_root/etc/shadow" ||
        die 'factory root account is not locked'

    if [ "$ssh_user" = nex-test ]; then
        printf '%s\n' 'nex-test:x:1000:1000:Nex QEMU Test:/home/nex-test:/bin/bash' \
            >> "$var_root/etc/passwd"
        printf '%s\n' 'nex-test:x:1000:' >> "$var_root/etc/group"
        printf '%s\n' 'nex-test:x:19735:0:99999:7:::' >> "$var_root/etc/shadow"
    fi
}

write_sshd_config() {
    var_root=$1
    ssh_user=$2

    mkdir -p "$var_root/etc/ssh"
    cat > "$var_root/etc/ssh/sshd_config" <<EOF
PermitRootLogin $(if [ "$ssh_user" = root ]; then printf prohibit-password; else printf no; fi)
PermitEmptyPasswords no
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
AuthorizedKeysFile .ssh/authorized_keys
AllowUsers $ssh_user
HostKey /etc/ssh/ssh_host_ed25519_key
HostKey /etc/ssh/ssh_host_rsa_key
HostKey /etc/ssh/ssh_host_ecdsa_key
Subsystem sftp /usr/libexec/sftp-server
EOF
    chmod 0644 "$var_root/etc/ssh/sshd_config"
}

enable_sshd() {
    var_root=$1
    wants=$var_root/etc/systemd/system/multi-user.target.wants

    mkdir -p "$wants"
    ln -sfn /usr/lib/systemd/system/sshd-keygen.service "$wants/sshd-keygen.service"
    ln -sfn /usr/lib/systemd/system/sshd.service "$wants/sshd.service"
}

prepare_identity() {
    factory_etc=$1
    var_root=$2
    public_key_file=$3
    ssh_user=$4

    [ "$ssh_user" = root ] || [ "$ssh_user" = nex-test ] ||
        die 'SSH user must be root or nex-test'
    [ -s "$public_key_file" ] || die "public key missing: $public_key_file"

    mkdir -p "$var_root/etc"
    if [ "$ssh_user" = root ]; then
        mkdir -p "$var_root/root/.ssh"
    else
        mkdir -p "$var_root/home/nex-test/.ssh"
    fi
    copy_accounts "$factory_etc" "$var_root" "$ssh_user"
    write_sshd_config "$var_root" "$ssh_user"
    enable_sshd "$var_root"

    if [ "$ssh_user" = root ]; then
        awk -F: 'BEGIN { OFS = ":" } $1 == "root" { $2 = "x" } { print }' \
            "$var_root/etc/shadow" > "$var_root/etc/shadow.tmp"
        mv "$var_root/etc/shadow.tmp" "$var_root/etc/shadow"
        cp "$public_key_file" "$var_root/root/.ssh/authorized_keys"
    else
        cp "$public_key_file" "$var_root/home/nex-test/.ssh/authorized_keys"
    fi

    chmod 0600 "$var_root/etc/shadow"
}

self_test() {
    test_root=$(mktemp -d)
    trap 'rm -rf "$test_root"' EXIT HUP INT TERM
    mkdir -p "$test_root/factory" "$test_root/user-var" "$test_root/root-var"

    printf '%s\n' \
        'root:x:0:0:root:/root:/bin/bash' \
        'sshd:x:74:74:SSH:/var/lib/sshd:/usr/bin/nologin' \
        > "$test_root/factory/passwd"
    printf '%s\n' 'root:x:0:' 'sshd:x:74:' > "$test_root/factory/group"
    printf '%s\n' 'root:!*:19735:0:99999:7:::' > "$test_root/factory/shadow"
    printf '%s\n' 'ssh-ed25519 AAAATEST nex-qemu-test' > "$test_root/key.pub"

    prepare_identity "$test_root/factory" "$test_root/user-var" "$test_root/key.pub" nex-test
    grep -q '^root:!\*:' "$test_root/user-var/etc/shadow" ||
        die 'self-test unlocked root for the desktop test user'
    grep -q '^nex-test:x:1000:1000:' "$test_root/user-var/etc/passwd" ||
        die 'self-test did not create the desktop test user'
    grep -q '^PermitRootLogin no$' "$test_root/user-var/etc/ssh/sshd_config" ||
        die 'self-test allowed root in desktop mode'
    grep -q '^PasswordAuthentication no$' "$test_root/user-var/etc/ssh/sshd_config" ||
        die 'self-test allowed SSH passwords'
    cmp "$test_root/key.pub" "$test_root/user-var/home/nex-test/.ssh/authorized_keys" ||
        die 'self-test did not install the desktop test key'
    [ -L "$test_root/user-var/etc/systemd/system/multi-user.target.wants/sshd.service" ] ||
        die 'self-test did not enable test-only SSH'

    prepare_identity "$test_root/factory" "$test_root/root-var" "$test_root/key.pub" root
    if grep -q '^nex-test:' "$test_root/root-var/etc/passwd"; then
        die 'self-test created the desktop test user in root mode'
    fi
    grep -q '^root:x:' "$test_root/root-var/etc/shadow" ||
        die 'self-test did not unlock root only in the disposable root-access tree'
    grep -q '^PermitRootLogin prohibit-password$' "$test_root/root-var/etc/ssh/sshd_config" ||
        die 'self-test did not restrict root to key authentication'
    cmp "$test_root/key.pub" "$test_root/root-var/root/.ssh/authorized_keys" ||
        die 'self-test did not install the disposable root key'

    printf 'PASS: QEMU test identity helper self-test\n'
}

case "${1:-}" in
    --self-test)
        [ "$#" -eq 1 ] || die 'usage: prepare-qemu-test-identity.sh --self-test'
        self_test
        ;;
    --set-ownership)
        [ "$#" -eq 2 ] || die 'usage: prepare-qemu-test-identity.sh --set-ownership VAR_ROOT'
        set_ownership "$2"
        ;;
    '')
        die 'usage: prepare-qemu-test-identity.sh FACTORY_ETC VAR_ROOT PUBLIC_KEY [root|nex-test]'
        ;;
    *)
        [ "$#" -eq 4 ] || die 'usage: prepare-qemu-test-identity.sh FACTORY_ETC VAR_ROOT PUBLIC_KEY [root|nex-test]'
        prepare_identity "$1" "$2" "$3" "$4"
        ;;
esac
