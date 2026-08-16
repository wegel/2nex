#!/usr/bin/env bash
# qemu-test-live-upgrade.sh: prove upgrade and rollback on an installed disk.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
NEX_BIN="${NEX_BIN:-$ROOT_DIR/src/cli/target/debug/nex}"
TEST_IDENTITY_HELPER="$SCRIPT_DIR/prepare-qemu-test-identity.sh"
ZUB_BIN="${ZUB_BIN:-zub}"
ZUB_REPO="${ZUB_REPO:-$ROOT_DIR/.nex/repo}"
FROM_REF="${FROM_REF:-systems/nex-systemd/0.0.1}"
TO_REF="${TO_REF:-systems/desktop-vwl/0.0.1}"
SSH_PORT="${SSH_PORT:-10024}"
TIMEOUT_SECS="${TIMEOUT_SECS:-240}"
MEMORY="${MEMORY:-4096}"
SMP="${SMP:-2}"
KEEP_WORK="${KEEP_LIVE_UPGRADE_WORK:-0}"
HARDLINK_PROBE_PATH="${HARDLINK_PROBE_PATH:-usr/lib/os-release}"

WORK_DIR="${WORK_DIR:-$ROOT_DIR/.nex/tmp/live-upgrade}"
ARTIFACT_DIR="${ARTIFACT_DIR:-$ROOT_DIR/.nex/tmp/live-upgrade-artifacts}"
TARGET_IMG="${TARGET_IMG:-$WORK_DIR/live-upgrade-target.img}"
ASSERT_KEY="$WORK_DIR/qemu-live-upgrade-ed25519"
SERIAL_LOG="$ARTIFACT_DIR/serial.log"
PROBE_LOG="$ARTIFACT_DIR/probe.log"
QEMU_LOG="$ARTIFACT_DIR/qemu.log"
CURRENT_SSH_PORT="$SSH_PORT"
QEMU_PID=""

BOOTLOADER_SRC="$ROOT_DIR/src/bootloader"
BOOTLOADER_EFI="$BOOTLOADER_SRC/target/x86_64-unknown-uefi/debug/nex-bootloader.efi"
KCMDLINE_SRC="$BOOTLOADER_SRC/kcmdline.vm.txt"
ESP_SIZE_MB=64
ROOT_MARGIN_MB=2048
VAR_MARGIN_MB=1024
VAR_PULL_MARGIN_MB=2048

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

log() {
    printf '==> %s\n' "$*"
}

need_tool() {
    command -v "$1" >/dev/null 2>&1 || die "$1 not found"
}

cleanup() {
    status=$?
    if [[ "$status" -eq 0 && "$KEEP_WORK" != 1 ]]; then
        rm -rf "$WORK_DIR"
    else
        printf 'work dir: %s\n' "$WORK_DIR" >&2
        printf 'artifacts: %s\n' "$ARTIFACT_DIR" >&2
    fi
}
trap cleanup EXIT

require_tools() {
    need_tool cargo
    need_tool dd
    need_tool mcopy
    need_tool mke2fs
    need_tool mkfs.vfat
    need_tool qemu-system-x86_64
    need_tool sfdisk
    need_tool ssh
    need_tool ssh-keygen
    need_tool truncate
    need_tool "$TEST_IDENTITY_HELPER"
    need_tool "$ZUB_BIN"
}

find_ovmf_code() {
    local path
    for path in \
        /usr/share/edk2/x64/OVMF_CODE.4m.fd \
        /usr/share/edk2/x64/OVMF_CODE.fd \
        /usr/share/edk2-ovmf/x64/OVMF_CODE.fd \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/qemu/OVMF.fd
    do
        [[ -f "$path" ]] && printf '%s\n' "$path" && return 0
    done
    return 1
}

metadata_value() {
    local ref=$1
    local key=$2
    "$ZUB_BIN" --repo "$ZUB_REPO" show "$ref" 2>/dev/null |
        awk -v key="${key}:" '$1 == key { print $2; exit }'
}

checksum_for_ref() {
    local ref=$1
    local checksum
    checksum=$(metadata_value "$ref" "nex.system.checksum")
    if [[ -z "$checksum" ]]; then
        checksum=$(metadata_value "$ref" "nex.build.checksum")
    fi
    if [[ -z "$checksum" ]]; then
        checksum=$("$ZUB_BIN" --repo "$ZUB_REPO" rev-parse "$ref" 2>/dev/null || true)
    fi
    [[ -n "$checksum" ]] || die "could not get checksum for $ref"
    printf '%s\n' "$checksum"
}

ensure_ref() {
    local ref=$1
    "$ZUB_BIN" --repo "$ZUB_REPO" rev-parse "$ref" >/dev/null 2>&1 ||
        die "$ref not found in $ZUB_REPO"
}

ensure_assert_key() {
    if [[ ! -f "$ASSERT_KEY" ]]; then
        ssh-keygen -q -t ed25519 -N "" -f "$ASSERT_KEY"
    fi
}

ssh_probe() {
    ssh \
        -i "$ASSERT_KEY" \
        -p "$CURRENT_SSH_PORT" \
        -o BatchMode=yes \
        -o ConnectTimeout=5 \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        root@127.0.0.1 "$@"
}

wait_for_ssh() {
    local end_time
    end_time=$(($(date +%s) + TIMEOUT_SECS))
    while [[ "$(date +%s)" -lt "$end_time" ]]; do
        if ssh_probe true >/dev/null 2>&1; then
            return 0
        fi
        sleep 2
    done
    return 1
}

print_logs() {
    printf '\n=== live upgrade paths ===\n'
    printf 'target image: %s\n' "$TARGET_IMG"
    printf 'serial log: %s\n' "$SERIAL_LOG"
    printf 'probe log: %s\n' "$PROBE_LOG"
    printf 'qemu log: %s\n' "$QEMU_LOG"
    if [[ -f "$SERIAL_LOG" ]]; then
        printf '\n=== serial log ===\n'
        cat "$SERIAL_LOG"
    fi
    if [[ -f "$PROBE_LOG" ]]; then
        printf '\n=== probe log ===\n'
        cat "$PROBE_LOG"
    fi
}

write_zub_config() {
    local repo_dir=$1
    local remote_path=$2
    cat > "$repo_dir/config.toml" <<EOF
[namespace]
uid_map = []
gid_map = []

[[remotes]]
name = "upgrade-source"
url = "$remote_path"
EOF
}

build_guest_repo() {
    local dest_repo=$1
    local remote_repo=$2

    "$ZUB_BIN" init "$dest_repo" >/dev/null
    write_zub_config "$dest_repo" "/var/nex/upgrade-source"

    "$ZUB_BIN" init "$remote_repo" >/dev/null
    "$ZUB_BIN" --repo "$remote_repo" pull "$ZUB_REPO" "$TO_REF" >/dev/null
}

prepare_qemu_network() {
    local var_content=$1
    local connection_dir="$var_content/etc/NetworkManager/system-connections"
    local networkd_dir="$var_content/etc/systemd/network"

    mkdir -p "$var_content/etc/modules-load.d" "$connection_dir" "$networkd_dir"
    printf '%s\n' virtio_net > "$var_content/etc/modules-load.d/00-nex-qemu-network.conf"
    cat > "$connection_dir/nex-qemu-test.nmconnection" <<'EOF'
[connection]
id=nex-qemu-test
type=ethernet
autoconnect=true

[ethernet]

[ipv4]
method=auto

[ipv6]
method=disabled
EOF
    chmod 0600 "$connection_dir/nex-qemu-test.nmconnection"

    cat > "$networkd_dir/20-nex-qemu-test.network" <<'EOF'
[Match]
Name=en*

[Network]
DHCP=yes
EOF
}

stage_root_and_var() {
    local source_ref=$1
    local source_checksum=$2
    local root_content=$3
    local var_content=$4
    local deploy_dir="$root_content/nex/deployments/${source_checksum}.0"
    local factory_etc

    mkdir -p "$deploy_dir"
    "$ZUB_BIN" --repo "$ZUB_REPO" checkout --copy "$source_ref" "$deploy_dir"

    mkdir -p \
        "$root_content/nex/staging" \
        "$root_content/proc" \
        "$root_content/sys" \
        "$root_content/dev" \
        "$root_content/run" \
        "$root_content/tmp"
    ln -sfn "deployments/${source_checksum}.0" "$root_content/nex/current"
    ln -sfn "current/nex/pkg" "$root_content/nex/pkg"
    ln -sfn "current/nex/db" "$root_content/nex/db"

    mkdir -p \
        "$deploy_dir/usr/lib" \
        "$deploy_dir/usr/lib/systemd/system" \
        "$deploy_dir/sysroot" \
        "$deploy_dir/var" \
        "$deploy_dir/etc" \
        "$deploy_dir/home" \
        "$deploy_dir/root" \
        "$deploy_dir/proc" \
        "$deploy_dir/sys" \
        "$deploy_dir/dev" \
        "$deploy_dir/run" \
        "$deploy_dir/tmp" \
        "$deploy_dir/nex/repo" \
        "$deploy_dir/nex/deployments" \
        "$deploy_dir/nex/staging" \
        "$deploy_dir/nex/users" \
        "$deploy_dir/nex/manifests"

    ln -sf "nex/deployments/${source_checksum}.0/usr" "$root_content/usr"
    ln -sf /usr/lib "$root_content/lib"
    ln -sf "nex/deployments/${source_checksum}.0/lib64" "$root_content/lib64"
    ln -sf /usr/bin "$root_content/bin"
    ln -sf /usr/bin "$root_content/sbin"
    ln -sf /var/etc "$root_content/etc"
    ln -sf /var/home "$root_content/home"
    ln -sf /var/root "$root_content/root"
    ln -sf usr/bin/init "$root_content/init"

    mkdir -p \
        "$var_content/etc/systemd/system/multi-user.target.wants" \
        "$var_content/home" \
        "$var_content/root" \
        "$var_content/log/journal" \
        "$var_content/lib/sshd" \
        "$var_content/lib/systemd/random-seed" \
        "$var_content/lib/systemd/timers" \
        "$var_content/lib/systemd/coredump" \
        "$var_content/cache/fontconfig" \
        "$var_content/tmp" \
        "$var_content/nex/repo" \
        "$var_content/nex/users" \
        "$var_content/nex/manifests" \
        "$var_content/nex/upgrade-source"
    chmod 1777 "$var_content/tmp"
    chmod 700 "$var_content/lib/sshd"
    cp -a "$deploy_dir/etc/." "$var_content/etc/"
    touch "$var_content/etc/.initialized"

    factory_etc="$deploy_dir/usr/share/factory/etc"
    if [[ ! -d "$factory_etc" ]]; then
        factory_etc="$deploy_dir/etc"
    fi
    if [[ "$source_ref" == "$FROM_REF" ]]; then
        printf '%s\n' 'version-one machine seed' \
            > "$factory_etc/nex-upgrade-retired-seed"
        ln -s \
            /nex/pkg/core/init/systemd/257.5/legacy123/usr/share/factory/etc/issue \
            "$var_content/etc/issue"
    fi
    "$TEST_IDENTITY_HELPER" "$factory_etc" "$var_content" "$ASSERT_KEY.pub" root
    prepare_qemu_network "$var_content"

    build_guest_repo "$var_content/nex/repo" "$var_content/nex/upgrade-source"
}

create_esp() {
    local esp_img=$1
    dd if=/dev/zero of="$esp_img" bs=1M count="$ESP_SIZE_MB" status=none
    mkfs.vfat -F 32 "$esp_img" >/dev/null
    mmd -i "$esp_img" ::/EFI
    mmd -i "$esp_img" ::/EFI/BOOT
    mcopy -i "$esp_img" "$BOOTLOADER_EFI" ::/EFI/BOOT/BOOTX64.EFI
    if [[ -f "$KCMDLINE_SRC" ]]; then
        mcopy -i "$esp_img" "$KCMDLINE_SRC" ::/EFI/BOOT/kcmdline.txt
    fi
}

build_disk() {
    local source_ref=$1
    local source_checksum=$2
    local reset_artifacts=${3:-1}
    local root_content="$WORK_DIR/root-content"
    local var_content="$WORK_DIR/var-content"
    local esp_img="$WORK_DIR/esp.img"
    local root_img="$WORK_DIR/root.img"
    local var_img="$WORK_DIR/var.img"
    local root_size_mb
    local root_payload_mb
    local remote_repo_mb
    local var_payload_mb
    local var_size_mb
    local root_start
    local root_sectors
    local var_start
    local var_sectors
    local disk_size_mb

    rm -rf "$WORK_DIR"
    if [[ "$reset_artifacts" == 1 ]]; then
        rm -rf "$ARTIFACT_DIR"
    fi
    mkdir -p "$root_content" "$var_content" "$ARTIFACT_DIR"
    if [[ "$reset_artifacts" == 1 ]]; then
        : > "$SERIAL_LOG"
        : > "$PROBE_LOG"
        : > "$QEMU_LOG"
    fi

    ensure_assert_key
    log "building bootloader"
    (cd "$BOOTLOADER_SRC" && cargo build) >/dev/null
    [[ -f "$BOOTLOADER_EFI" ]] || die "bootloader was not created"

    log "staging installed system from $source_ref"
    stage_root_and_var "$source_ref" "$source_checksum" "$root_content" "$var_content"
    create_esp "$esp_img"

    remote_repo_mb=$(du -sm "$var_content/nex/upgrade-source" | awk '{ print $1 }')
    root_payload_mb=$(du -sm "$root_content" | awk '{ print $1 }')
    var_payload_mb=$(du -sm "$var_content" | awk '{ print $1 }')
    root_size_mb=$((root_payload_mb * 2 + remote_repo_mb + ROOT_MARGIN_MB))
    # The fixture keeps one complete source repo in /var, then pulls that repo
    # into the writable system repo. Leave another repo-sized working area for
    # temporary object files and filesystem overhead during the pull.
    var_size_mb=$((var_payload_mb + remote_repo_mb * 2 + VAR_PULL_MARGIN_MB))

    log "creating root filesystem (${root_size_mb}MiB)"
    log "creating var filesystem (${var_size_mb}MiB)"
    if command -v fakeroot >/dev/null 2>&1; then
        fakeroot -- bash -c "chown -R 0:0 '$root_content' '$var_content' && '$TEST_IDENTITY_HELPER' --set-ownership '$var_content' && mke2fs -q -t ext4 -L nex -d '$root_content' '$root_img' ${root_size_mb}M && mke2fs -q -t ext4 -L nex-var -d '$var_content' '$var_img' ${var_size_mb}M"
    else
        mke2fs -q -t ext4 -L nex -d "$root_content" "$root_img" "${root_size_mb}M"
        mke2fs -q -t ext4 -L nex-var -d "$var_content" "$var_img" "${var_size_mb}M"
    fi

    root_start=$((ESP_SIZE_MB * 2048 + 2048))
    root_sectors=$((root_size_mb * 2048))
    var_start=$((root_start + root_sectors))
    var_sectors=$((var_size_mb * 2048))
    disk_size_mb=$((ESP_SIZE_MB + root_size_mb + var_size_mb + 64))

    log "creating target image (${disk_size_mb}MiB)"
    truncate -s "${disk_size_mb}M" "$TARGET_IMG"
    sfdisk "$TARGET_IMG" >/dev/null <<EOF
label: gpt
unit: sectors

start=2048, size=$((ESP_SIZE_MB * 2048)), type=uefi, name="EFI"
start=${root_start}, size=${root_sectors}, type=linux, name="nex"
start=${var_start}, size=${var_sectors}, type=linux, name="nex-var"
EOF
    dd if="$esp_img" of="$TARGET_IMG" bs=1M seek=1 conv=notrunc,sparse status=none
    dd if="$root_img" of="$TARGET_IMG" bs=1M seek="$((root_start / 2048))" \
        conv=notrunc,sparse status=none
    dd if="$var_img" of="$TARGET_IMG" bs=1M seek="$((var_start / 2048))" \
        conv=notrunc,sparse status=none
}

qemu_args() {
    local ovmf_code=$1
    local ssh_port=$2
    local ovmf_vars=$3
    local accel_args=()

    if [[ -e /dev/kvm ]]; then
        accel_args=(-enable-kvm -cpu host)
    else
        accel_args=(-accel tcg -cpu max)
    fi

    printf '%s\0' \
        "${accel_args[@]}" \
        -machine q35 \
        -m "$MEMORY" \
        -smp "$SMP" \
        -drive "if=pflash,format=raw,readonly=on,file=$ovmf_code"
    if [[ -f "$ovmf_vars" ]]; then
        printf '%s\0' -drive "if=pflash,format=raw,file=$ovmf_vars"
    fi
    printf '%s\0' \
        -drive "file=$TARGET_IMG,format=raw,if=none,id=disk" \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk,bus=ahci.0 \
        -netdev "user,id=net0,hostfwd=tcp:127.0.0.1:${ssh_port}-:22" \
        -device virtio-net-pci,netdev=net0 \
        -serial "file:$SERIAL_LOG" \
        -display none \
        -no-reboot
}

start_guest() {
    local ovmf_code=$1
    local ssh_port=$2
    local ovmf_vars_template="${ovmf_code/OVMF_CODE/OVMF_VARS}"
    local ovmf_vars="$WORK_DIR/OVMF_VARS.fd"
    local qemu_argv=()

    rm -f "$ovmf_vars"
    [[ -f "$ovmf_vars_template" ]] && cp "$ovmf_vars_template" "$ovmf_vars"
    mapfile -d '' -t qemu_argv < <(qemu_args "$ovmf_code" "$ssh_port" "$ovmf_vars")

    printf 'qemu-command=' >> "$QEMU_LOG"
    printf '%q ' qemu-system-x86_64 "${qemu_argv[@]}" >> "$QEMU_LOG"
    printf '\n' >> "$QEMU_LOG"
    qemu-system-x86_64 "${qemu_argv[@]}" >> "$QEMU_LOG" 2>&1 &
    QEMU_PID=$!
}

stop_guest() {
    local qemu_pid=$1
    kill "$qemu_pid" 2>/dev/null || true
    wait "$qemu_pid" 2>/dev/null || true
}

boot_and_wait() {
    local ovmf_code=$1
    local ssh_port=$2

    CURRENT_SSH_PORT="$ssh_port"
    start_guest "$ovmf_code" "$ssh_port"
    if ! wait_for_ssh; then
        stop_guest "$QEMU_PID"
        print_logs
        die "guest SSH probe did not become ready"
    fi
}

assert_current_deployment() {
    local expected=$1
    local cmdline
    local actual

    cmdline=$(ssh_probe 'cat /proc/cmdline')
    actual=$(printf '%s\n' "$cmdline" | sed -n 's/.*zub=\([^ ]*\).*/\1/p')
    actual=${actual##*/}
    printf 'cmdline=%s\n' "$cmdline" | tee -a "$PROBE_LOG"
    [[ "$actual" == "$expected" ]] ||
        die "expected deployment $expected, got $actual"
}

run_guest_cmd() {
    printf '$ %s\n' "$*" | tee -a "$PROBE_LOG"
    ssh_probe "$@" 2>&1 | tee -a "$PROBE_LOG"
}

assert_vendor_release() {
    local expected=$1

    run_guest_cmd "hostnamectl status | grep -Fq 'Operating System: ${expected}'"
}

assert_systemd_network_policy() {
    run_guest_cmd "
        set -eu
        systemctl is-active --quiet systemd-networkd.service
        test -e /usr/lib/systemd/system/systemd-networkd.service
        if test -L /usr/lib/systemd/system/systemd-networkd.service; then
            test \"\$(readlink /usr/lib/systemd/system/systemd-networkd.service)\" != /dev/null
        fi
    "
}

assert_desktop_network_policy() {
    run_guest_cmd "
        set -eu
        systemctl is-active --quiet NetworkManager.service
        ! systemctl is-active --quiet systemd-networkd.service
        test \"\$(readlink /usr/lib/systemd/system/systemd-networkd.service)\" = /dev/null
    "
}

assert_administrator_state() {
    run_guest_cmd "
        set -eu
        test \"\$(cat /etc/nex-admin-choice)\" = administrator
        systemd-analyze cat-config systemd/logind.conf \
            | grep -Fq 'HandlePowerKey=ignore'
        systemctl restart systemd-logind.service
        test \"\$(busctl get-property \
            org.freedesktop.login1 \
            /org/freedesktop/login1 \
            org.freedesktop.login1.Manager \
            HandlePowerKey)\" = 's \"ignore\"'
    "
}

assert_adapters() {
    run_guest_cmd "
        set -eu
        printf 'mtab=%s\n' \"\$(readlink /etc/mtab)\"
        printf 'resolv.conf=%s\n' \"\$(readlink /etc/resolv.conf)\"
        printf 'ssl/certs=%s\n' \"\$(readlink /etc/ssl/certs)\"
        mtab_target=\$(readlink /etc/mtab)
        test \"\$mtab_target\" = /proc/self/mounts ||
            test \"\$mtab_target\" = ../proc/self/mounts
        test \"\$(readlink /etc/resolv.conf)\" = /run/systemd/resolve/stub-resolv.conf
        test \"\$(readlink /etc/ssl/certs)\" = /run/ssl/certs
        test -e /etc/mtab
        printf '%s\n' 'mtab resolves'
        test -e /etc/resolv.conf
        printf '%s\n' 'resolv.conf resolves'
        test -e /etc/ssl/certs
        printf '%s\n' 'ssl/certs resolves'
    "
}

assert_deployment_file_materialized() {
    local deployment=$1

    run_guest_cmd "
        set -eu
        deployment_root='/sysroot/nex/deployments/${deployment}'
        repo_blobs='/nex/repo/objects/blobs'
        file=\"\$deployment_root/${HARDLINK_PROBE_PATH}\"
        test -f \"\$file\"
        links=\$(stat -c '%h' \"\$file\")
        blob=\$(find \"\$repo_blobs\" -xdev -type f -samefile \"\$file\" -print -quit 2>/dev/null || true)
        if [ -n \"\$blob\" ]; then
            printf 'repo-hardlink %s %s %s\n' \"\$file\" \"\$links\" \"\$blob\"
            exit 0
        fi
        printf 'repo-copy %s %s\n' \"\$file\" \"\$links\"
    "
}

assert_rollback_hardlinked() {
    local source_deployment=$1
    local rollback_deployment=$2

    run_guest_cmd "
        source_file=\$(find '/sysroot/nex/deployments/${source_deployment}/nex/pkg/core/nex/zub' -path '*/usr/bin/zub' -type f -print -quit)
        rollback_file=\$(find '/sysroot/nex/deployments/${rollback_deployment}/nex/pkg/core/nex/zub' -path '*/usr/bin/zub' -type f -print -quit)
        test -n \"\$source_file\"
        test -n \"\$rollback_file\"
        source_inode=\$(stat -c '%d:%i' \"\$source_file\")
        rollback_inode=\$(stat -c '%d:%i' \"\$rollback_file\")
        printf 'rollback-hardlink %s %s\n' \"\$source_inode\" \"\$rollback_inode\"
        test \"\$source_inode\" = \"\$rollback_inode\"
    "
}

run_upgrade_flow() {
    local ovmf_code=$1
    local from_checksum=$2
    local to_checksum=$3
    local qemu_pid

    log "booting old deployment through the bootloader"
    boot_and_wait "$ovmf_code" "$SSH_PORT"
    qemu_pid=$QEMU_PID
    assert_current_deployment "${from_checksum}.0"

    assert_vendor_release "nex Linux"
    assert_systemd_network_policy
    run_guest_cmd "test \"\$(cat /etc/nex-upgrade-retired-seed)\" = 'version-one machine seed'"
    run_guest_cmd "test ! -e /etc/issue && test ! -L /etc/issue"
    run_guest_cmd "
        set -eu
        mkdir -p /etc/systemd/logind.conf.d
        printf '%s\n' '[Login]' 'HandlePowerKey=ignore' \
            > /etc/systemd/logind.conf.d/99-nex-upgrade-test.conf
        printf '%s\n' administrator > /etc/nex-admin-choice
    "
    assert_administrator_state
    assert_adapters

    run_guest_cmd "! zub --repo /nex/repo rev-parse '$TO_REF' >/tmp/to-ref-before 2>&1"
    run_guest_cmd "nex upgrade '$TO_REF' --sysroot /sysroot --repo /nex/repo"
    run_guest_cmd "test -d '/sysroot/nex/deployments/${to_checksum}.1'"
    run_guest_cmd "zub --repo /nex/repo rev-parse '$TO_REF' >/dev/null"
    assert_deployment_file_materialized "${to_checksum}.1"
    stop_guest "$qemu_pid"

    log "rebooting into upgraded deployment through the bootloader"
    boot_and_wait "$ovmf_code" "$((SSH_PORT + 1))"
    qemu_pid=$QEMU_PID
    assert_current_deployment "${to_checksum}.1"
    assert_vendor_release "nex Desktop"
    assert_desktop_network_policy
    run_guest_cmd "test \"\$(cat /etc/nex-upgrade-retired-seed)\" = 'version-one machine seed'"
    assert_administrator_state
    assert_adapters
    run_guest_cmd "test -z \"\$(find /etc -type l -lname '/nex/pkg/*' -print -quit)\""
    run_guest_cmd "nex rollback --yes --sysroot /sysroot"
    run_guest_cmd "test -d '/sysroot/nex/deployments/${from_checksum}.2'"
    assert_rollback_hardlinked "${from_checksum}.0" "${from_checksum}.2"
    stop_guest "$qemu_pid"

    log "rebooting into rollback deployment through the bootloader"
    boot_and_wait "$ovmf_code" "$((SSH_PORT + 2))"
    qemu_pid=$QEMU_PID
    assert_current_deployment "${from_checksum}.2"
    assert_vendor_release "nex Linux"
    assert_systemd_network_policy
    run_guest_cmd "test \"\$(cat /etc/nex-upgrade-retired-seed)\" = 'version-one machine seed'"
    assert_administrator_state
    assert_adapters
    run_guest_cmd "nex deployments --path /sysroot/nex/deployments"
    stop_guest "$qemu_pid"

    log "building and booting a fresh version 2 machine"
    build_disk "$TO_REF" "$to_checksum" 0
    boot_and_wait "$ovmf_code" "$((SSH_PORT + 3))"
    qemu_pid=$QEMU_PID
    assert_current_deployment "${to_checksum}.0"
    assert_vendor_release "nex Desktop"
    assert_desktop_network_policy
    run_guest_cmd "test ! -e /etc/nex-upgrade-retired-seed"
    run_guest_cmd "test ! -e /etc/nex-admin-choice"
    run_guest_cmd "test ! -e /etc/systemd/logind.conf.d/99-nex-upgrade-test.conf"
    assert_adapters
    stop_guest "$qemu_pid"

    printf 'LIVE-UPGRADE-PASS\n' | tee -a "$PROBE_LOG"
}

main() {
    local ovmf_code
    local from_checksum
    local to_checksum

    require_tools
    [[ -x "$NEX_BIN" ]] || die "nex binary not found or not executable: $NEX_BIN"
    [[ -d "$ZUB_REPO" ]] || die "zub repo not found: $ZUB_REPO"
    ovmf_code=$(find_ovmf_code) || die "OVMF firmware not found"

    ensure_ref "$FROM_REF"
    ensure_ref "$TO_REF"
    from_checksum=$(checksum_for_ref "$FROM_REF")
    to_checksum=$(checksum_for_ref "$TO_REF")
    [[ "$from_checksum" != "$to_checksum" ]] ||
        die "from and to refs resolve to the same checksum"

    log "from ref: $FROM_REF"
    log "to ref:   $TO_REF"
    log "from checksum: $from_checksum"
    log "to checksum:   $to_checksum"

    build_disk "$FROM_REF" "$from_checksum"
    run_upgrade_flow "$ovmf_code" "$from_checksum" "$to_checksum"
    print_logs
}

main "$@"
