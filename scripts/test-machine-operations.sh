#!/usr/bin/env bash
# test-machine-operations.sh: prove the operations a user performs on an
# installed Nex machine, driven entirely from inside a booted guest over SSH.
#
# See .agents/execplans/017-machine-operation-tests.md for the design. This
# implements the harness and all four journeys: build-package,
# temporary-install, persistent-install, deploy-and-rollback.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
JOURNEY_DIR="$SCRIPT_DIR/machine-journeys"
TEST_IDENTITY_HELPER="$SCRIPT_DIR/prepare-qemu-test-identity.sh"

ZUB_BIN="${ZUB_BIN:-/home/wegel/work/perso/zub/target/debug/zub}"
ZUB_REPO="${ZUB_REPO:-$ROOT_DIR/.nex/repo}"
FROM_REF="${FROM_REF:-systems/nex-test-fixture/0.0.1}"
EXPECTED_CHECKSUM="${EXPECTED_CHECKSUM:-a22e42ab383d0c108c3fb6f2ada6899c6f3a21ff31aed3c05e61b112aee6b507}"

SSH_PORT_BASE="${SSH_PORT_BASE:-10040}"
TIMEOUT_SECS="${TIMEOUT_SECS:-240}"
MEMORY="${MEMORY:-2048}"
SMP="${SMP:-2}"
KEEP_WORK="${KEEP_MACHINE_TEST_WORK:-0}"

MKE2FS_BIN="${MKE2FS_BIN:-/usr/bin/mke2fs}"
QEMU_IMG_BIN="${QEMU_IMG_BIN:-/usr/bin/qemu-img}"

WORK_DIR="${WORK_DIR:-$ROOT_DIR/.nex/tmp/machine-tests-work}"
ARTIFACT_ROOT="${ARTIFACT_ROOT:-$ROOT_DIR/.nex/tmp/machine-tests}"
BACKING_IMG="$WORK_DIR/backing.img"
ASSERT_KEY="$WORK_DIR/machine-test-ed25519"

BOOTLOADER_SRC="$ROOT_DIR/src/bootloader"
BOOTLOADER_EFI="$BOOTLOADER_SRC/target/x86_64-unknown-uefi/debug/nex-bootloader.efi"
KCMDLINE_SRC="$BOOTLOADER_SRC/kcmdline.vm.txt"
ESP_SIZE_MB=64
ROOT_MARGIN_MB=2048
VAR_MARGIN_MB=1536

# Refs pulled from the host's build store into the guest's system store
# (/nex/repo) before boot, so journeys that install or deploy never need to
# build. tig/outputs/bin is a small already-built package outside the
# fixture's own package list, for journeys 2-3. nex-systemd/0.0.1 is a
# different already-built system, for journey 4's deploy target.
SEED_REFS=(
    "x86_64/pkg/dev/vcs/tig/2.6.0/outputs/bin"
    "systems/nex-systemd/0.0.1"
)

# All journeys this harness knows about, in run order.
ALL_JOURNEYS=(build-package temporary-install persistent-install deploy-and-rollback)

CURRENT_SSH_PORT=""
CURRENT_ASSERT_KEY="$ASSERT_KEY"
QEMU_PID=""
BOOT_WAITED_SECS=""

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
    local status=$?
    if [[ -n "$QEMU_PID" ]]; then
        kill "$QEMU_PID" 2>/dev/null || true
        wait "$QEMU_PID" 2>/dev/null || true
    fi
    if [[ "$status" -eq 0 && "$KEEP_WORK" != 1 ]]; then
        rm -rf "$WORK_DIR"
    else
        printf 'work dir: %s\n' "$WORK_DIR" >&2
    fi
}
trap cleanup EXIT

require_tools() {
    need_tool cargo
    need_tool dd
    need_tool mcopy
    need_tool mmd
    need_tool mkfs.vfat
    need_tool qemu-system-x86_64
    need_tool scp
    need_tool sfdisk
    need_tool ssh
    need_tool ssh-keygen
    need_tool truncate
    [[ -x "$MKE2FS_BIN" ]] || die "mke2fs not found at $MKE2FS_BIN"
    [[ -x "$QEMU_IMG_BIN" ]] || die "qemu-img not found at $QEMU_IMG_BIN"
    [[ -x "$TEST_IDENTITY_HELPER" ]] || die "$TEST_IDENTITY_HELPER not found"
    [[ -x "$ZUB_BIN" ]] || die "zub binary not found or not executable: $ZUB_BIN"
    [[ -d "$ZUB_REPO" ]] || die "zub repo not found: $ZUB_REPO"
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
    checksum=$(metadata_value "$ref" "nex.build.checksum")
    if [[ -z "$checksum" ]]; then
        checksum=$(metadata_value "$ref" "nex.system.checksum")
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
        -i "$CURRENT_ASSERT_KEY" \
        -p "$CURRENT_SSH_PORT" \
        -o BatchMode=yes \
        -o ConnectTimeout=5 \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        -o LogLevel=ERROR \
        root@127.0.0.1 "$@"
}

scp_to_guest() {
    local src=$1
    local dst=$2
    scp \
        -i "$CURRENT_ASSERT_KEY" \
        -P "$CURRENT_SSH_PORT" \
        -o BatchMode=yes \
        -o ConnectTimeout=5 \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        -o LogLevel=ERROR \
        "$src" "root@127.0.0.1:$dst"
}

wait_for_ssh() {
    local end_time
    local start_time
    start_time=$(date +%s)
    end_time=$((start_time + TIMEOUT_SECS))
    while [[ "$(date +%s)" -lt "$end_time" ]]; do
        if ssh_probe true >/dev/null 2>&1; then
            printf '%s\n' "$(($(date +%s) - start_time))"
            return 0
        fi
        sleep 2
    done
    return 1
}

write_zub_config() {
    local repo_dir=$1
    cat > "$repo_dir/config.toml" <<'EOF'
[namespace]
uid_map = []
gid_map = []
EOF
}

# seed_system_repo REPO_DIR
# Pre-populates the guest's system store with refs journeys 2-4 need already
# built, so the guest never has to build a package (which the environment bug
# recorded in .agents/knowledge/machine-self-hosting.md would make fail). This
# is host-side test setup, not the operation under test: the guest still runs
# every stage/install/discard/commit/deploy/rollback command itself. Objects
# are pulled from the host's own build store, so nothing is fetched from the
# network and nothing is built here.
seed_system_repo() {
    local repo_dir=$1
    local ref

    for ref in "${SEED_REFS[@]}"; do
        "$ZUB_BIN" --repo "$repo_dir" pull "$ZUB_REPO" "$ref" >/dev/null
    done
}

prepare_qemu_network() {
    local var_content=$1
    local connection_dir="$var_content/etc/NetworkManager/system-connections"
    local networkd_dir="$var_content/etc/systemd/network"

    mkdir -p "$var_content/etc/modules-load.d" "$connection_dir" "$networkd_dir"
    printf '%s\n' virtio_net > "$var_content/etc/modules-load.d/00-nex-machine-test-network.conf"
    cat > "$connection_dir/nex-machine-test.nmconnection" <<'EOF'
[connection]
id=nex-machine-test
type=ethernet
autoconnect=true

[ethernet]

[ipv4]
method=auto

[ipv6]
method=disabled
EOF
    chmod 0600 "$connection_dir/nex-machine-test.nmconnection"

    cat > "$networkd_dir/20-nex-machine-test.network" <<'EOF'
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
        "$var_content/nex/manifests"
    chmod 1777 "$var_content/tmp"
    chmod 700 "$var_content/lib/sshd"
    cp -a "$deploy_dir/etc/." "$var_content/etc/"
    touch "$var_content/etc/.initialized"
    "$ZUB_BIN" init "$var_content/nex/repo" >/dev/null
    write_zub_config "$var_content/nex/repo"
    seed_system_repo "$var_content/nex/repo"

    factory_etc="$deploy_dir/usr/share/factory/etc"
    if [[ ! -d "$factory_etc" ]]; then
        factory_etc="$deploy_dir/etc"
    fi
    "$TEST_IDENTITY_HELPER" "$factory_etc" "$var_content" "$ASSERT_KEY.pub" root
    prepare_qemu_network "$var_content"
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

build_backing_image() {
    local source_ref=$1
    local source_checksum=$2
    local root_content="$WORK_DIR/root-content"
    local var_content="$WORK_DIR/var-content"
    local esp_img="$WORK_DIR/esp.img"
    local root_img="$WORK_DIR/root.img"
    local var_img="$WORK_DIR/var.img"
    local root_size_mb root_payload_mb var_payload_mb var_size_mb
    local root_start root_sectors var_start var_sectors disk_size_mb

    rm -rf "$WORK_DIR"
    mkdir -p "$root_content" "$var_content"

    ensure_assert_key
    log "building bootloader"
    (cd "$BOOTLOADER_SRC" && cargo build) >/dev/null
    [[ -f "$BOOTLOADER_EFI" ]] || die "bootloader was not created"

    log "staging fixture from $source_ref (checksum $source_checksum)"
    stage_root_and_var "$source_ref" "$source_checksum" "$root_content" "$var_content"
    create_esp "$esp_img"

    root_payload_mb=$(du -sm "$root_content" | awk '{ print $1 }')
    var_payload_mb=$(du -sm "$var_content" | awk '{ print $1 }')
    root_size_mb=$((root_payload_mb * 2 + ROOT_MARGIN_MB))
    var_size_mb=$((var_payload_mb + VAR_MARGIN_MB))

    log "creating root filesystem (${root_size_mb}MiB)"
    log "creating var filesystem (${var_size_mb}MiB)"
    if command -v fakeroot >/dev/null 2>&1; then
        fakeroot -- bash -c "chown -R 0:0 '$root_content' '$var_content' && '$TEST_IDENTITY_HELPER' --set-ownership '$var_content' && '$MKE2FS_BIN' -q -t ext4 -L nex -d '$root_content' '$root_img' ${root_size_mb}M && '$MKE2FS_BIN' -q -t ext4 -L nex-var -d '$var_content' '$var_img' ${var_size_mb}M"
    else
        "$MKE2FS_BIN" -q -t ext4 -L nex -d "$root_content" "$root_img" "${root_size_mb}M"
        "$MKE2FS_BIN" -q -t ext4 -L nex-var -d "$var_content" "$var_img" "${var_size_mb}M"
    fi

    root_start=$((ESP_SIZE_MB * 2048 + 2048))
    root_sectors=$((root_size_mb * 2048))
    var_start=$((root_start + root_sectors))
    var_sectors=$((var_size_mb * 2048))
    disk_size_mb=$((ESP_SIZE_MB + root_size_mb + var_size_mb + 64))

    log "creating backing image (${disk_size_mb}MiB)"
    truncate -s "${disk_size_mb}M" "$BACKING_IMG"
    sfdisk "$BACKING_IMG" >/dev/null <<EOF
label: gpt
unit: sectors

start=2048, size=$((ESP_SIZE_MB * 2048)), type=uefi, name="EFI"
start=${root_start}, size=${root_sectors}, type=linux, name="nex"
start=${var_start}, size=${var_sectors}, type=linux, name="nex-var"
EOF
    dd if="$esp_img" of="$BACKING_IMG" bs=1M seek=1 conv=notrunc,sparse status=none
    dd if="$root_img" of="$BACKING_IMG" bs=1M seek="$((root_start / 2048))" \
        conv=notrunc,sparse status=none
    dd if="$var_img" of="$BACKING_IMG" bs=1M seek="$((var_start / 2048))" \
        conv=notrunc,sparse status=none
    rm -f "$esp_img" "$root_img" "$var_img"
    rm -rf "$root_content" "$var_content"
}

qemu_args() {
    local ovmf_code=$1
    local overlay_img=$2
    local ssh_port=$3
    local ovmf_vars=$4
    local serial_log=$5
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
        -drive "file=$overlay_img,format=qcow2,if=none,id=disk" \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk,bus=ahci.0 \
        -netdev "user,id=net0,hostfwd=tcp:127.0.0.1:${ssh_port}-:22" \
        -device virtio-net-pci,netdev=net0 \
        -serial "file:$serial_log" \
        -display none \
        -no-reboot
}

# --- harness verbs -----------------------------------------------------

# boot OVERLAY_IMG SSH_PORT ARTIFACT_DIR
# Starts QEMU from a qcow2 overlay and waits for SSH. Sets QEMU_PID and
# CURRENT_SSH_PORT as globals, and BOOT_WAITED_SECS to how long SSH took.
# Must be called directly, never inside a command substitution: the globals
# it sets would be lost when the subshell exits.
boot() {
    local overlay_img=$1
    local ssh_port=$2
    local artifact_dir=$3
    local ovmf_code
    local ovmf_vars="$artifact_dir/OVMF_VARS.fd"
    local ovmf_vars_template
    local serial_log="$artifact_dir/serial.log"
    local qemu_log="$artifact_dir/qemu.log"
    local qemu_argv=()
    local waited

    ovmf_code=$(find_ovmf_code) || die "OVMF firmware not found"
    ovmf_vars_template="${ovmf_code/OVMF_CODE/OVMF_VARS}"
    rm -f "$ovmf_vars"
    [[ -f "$ovmf_vars_template" ]] && cp "$ovmf_vars_template" "$ovmf_vars"
    : > "$serial_log"
    : > "$qemu_log"

    CURRENT_SSH_PORT="$ssh_port"
    mapfile -d '' -t qemu_argv < <(qemu_args "$ovmf_code" "$overlay_img" "$ssh_port" "$ovmf_vars" "$serial_log")

    printf 'qemu-command=' >> "$qemu_log"
    printf '%q ' qemu-system-x86_64 "${qemu_argv[@]}" >> "$qemu_log"
    printf '\n' >> "$qemu_log"
    qemu-system-x86_64 "${qemu_argv[@]}" >> "$qemu_log" 2>&1 &
    QEMU_PID=$!

    if ! waited=$(wait_for_ssh); then
        stop_guest
        die "guest SSH probe did not become ready within ${TIMEOUT_SECS}s (serial log: $serial_log, qemu log: $qemu_log)"
    fi
    BOOT_WAITED_SECS="$waited"
}

stop_guest() {
    if [[ -n "$QEMU_PID" ]]; then
        kill "$QEMU_PID" 2>/dev/null || true
        wait "$QEMU_PID" 2>/dev/null || true
        QEMU_PID=""
    fi
}

# run CMD...
# Runs a command on the booted guest over SSH and echoes its output.
run() {
    ssh_probe "$@"
}

# reboot ARTIFACT_DIR OVERLAY_IMG SSH_PORT
# Tells the guest to reboot, waits for QEMU to exit (it runs with -no-reboot,
# so a guest reboot ends the process), then boots the same overlay again.
reboot() {
    local artifact_dir=$1
    local overlay_img=$2
    local ssh_port=$3
    local qemu_pid=$QEMU_PID

    ssh_probe 'systemctl reboot' >/dev/null 2>&1 || true
    if [[ -n "$qemu_pid" ]]; then
        wait "$qemu_pid" 2>/dev/null || true
    fi
    QEMU_PID=""
    boot "$overlay_img" "$ssh_port" "$artifact_dir"
}

# expect_deployment EXPECTED
# Asserts, from the guest's own /proc/cmdline, which deployment it actually
# booted into. Never checks a value the test supplied.
expect_deployment() {
    local expected=$1
    local cmdline
    local actual

    cmdline=$(ssh_probe 'cat /proc/cmdline')
    actual=$(printf '%s\n' "$cmdline" | sed -n 's/.*zub=\([^ ]*\).*/\1/p')
    actual=${actual##*/}
    printf 'cmdline=%s\n' "$cmdline"
    [[ "$actual" == "$expected" ]] ||
        die "expected deployment $expected, got $actual"
}

# --- journeys ------------------------------------------------------------

# Phase list per journey, "|"-separated. "_" means run the guest script with
# no argument. "REBOOT" means the host reboots the guest (killing and
# restarting QEMU on the same overlay) before the next phase. Journeys that
# cross a reboot pass state to their later phases through the guest's own
# persistent files, never through the host.
declare -A JOURNEY_PHASES=(
    [build-package]="_"
    [temporary-install]="_"
    [persistent-install]="before-reboot|REBOOT|after-reboot"
    [deploy-and-rollback]="deploy|REBOOT|verify-deployed|rollback|REBOOT|verify-rolled-back"
)

run_journey() {
    local journey=$1
    local artifact_dir="$ARTIFACT_ROOT/$journey"
    local overlay_img="$artifact_dir/overlay.qcow2"
    local guest_script="$JOURNEY_DIR/$journey.sh"
    local ssh_port=$((SSH_PORT_BASE + RANDOM % 1000))
    local stdout_log="$artifact_dir/guest-stdout.log"
    local journal_log="$artifact_dir/guest-journal.log"
    local phases_spec="${JOURNEY_PHASES[$journey]:-_}"
    local phase_list=()
    local phase
    local phase_arg
    local guest_exit=0
    local failed=0
    local reboot_count=0

    [[ -f "$guest_script" ]] || die "no journey script for $journey ($guest_script)"

    rm -rf "$artifact_dir"
    mkdir -p "$artifact_dir"
    : > "$stdout_log"

    log "[$journey] creating overlay over $BACKING_IMG"
    "$QEMU_IMG_BIN" create -q -f qcow2 -F raw -b "$BACKING_IMG" "$overlay_img" >/dev/null

    log "[$journey] booting on port $ssh_port"
    boot "$overlay_img" "$ssh_port" "$artifact_dir"
    log "[$journey] SSH ready after ${BOOT_WAITED_SECS}s"

    log "[$journey] copying $guest_script to guest"
    scp_to_guest "$guest_script" "/root/$journey.sh"

    IFS='|' read -r -a phase_list <<< "$phases_spec"
    for phase in "${phase_list[@]}"; do
        if [[ "$phase" == "REBOOT" ]]; then
            reboot_count=$((reboot_count + 1))
            log "[$journey] rebooting (reboot #$reboot_count)"
            printf '\n=== reboot #%s ===\n' "$reboot_count" >> "$stdout_log"
            reboot "$artifact_dir" "$overlay_img" "$ssh_port"
            log "[$journey] SSH ready after reboot (${BOOT_WAITED_SECS}s)"
            continue
        fi

        phase_arg=""
        [[ "$phase" != "_" ]] && phase_arg="$phase"
        log "[$journey] running $journey.sh $phase_arg"
        printf '\n=== phase: %s ===\n' "${phase_arg:-default}" >> "$stdout_log"
        set +e
        run "chmod +x /root/$journey.sh && /root/$journey.sh $phase_arg" >> "$stdout_log" 2>&1
        guest_exit=$?
        set -e
        if [[ "$guest_exit" -ne 0 ]]; then
            failed=1
            break
        fi
    done

    cat "$stdout_log"

    if [[ "$failed" -ne 0 ]]; then
        run 'journalctl -b --no-pager' > "$journal_log" 2>&1 || true
    fi

    stop_guest

    if [[ "$failed" -eq 0 ]]; then
        printf 'PASS: %s\n' "$journey"
        return 0
    else
        printf 'FAIL: %s (guest exit %s)\n' "$journey" "$guest_exit"
        printf 'artifacts: %s\n' "$artifact_dir"
        return 1
    fi
}

usage() {
    cat <<EOF
usage: $(basename "$0") [--journey NAME]

Runs the machine-operation journeys against a freshly overlaid boot of the
$FROM_REF fixture. With no --journey, runs every journey this harness knows:
${ALL_JOURNEYS[*]}
EOF
}

main() {
    local only_journey=""
    local journeys=()
    local journey
    local failures=0
    local from_checksum

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --journey)
                only_journey=$2
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                die "unknown argument: $1"
                ;;
        esac
    done

    require_tools

    ensure_ref "$FROM_REF"
    from_checksum=$(checksum_for_ref "$FROM_REF")
    [[ "$from_checksum" == "$EXPECTED_CHECKSUM" ]] ||
        die "fixture checksum drifted: expected $EXPECTED_CHECKSUM, got $from_checksum for $FROM_REF"

    log "fixture ref: $FROM_REF"
    log "fixture checksum: $from_checksum"

    mkdir -p "$ARTIFACT_ROOT"
    build_backing_image "$FROM_REF" "$from_checksum"

    if [[ -n "$only_journey" ]]; then
        journeys=("$only_journey")
    else
        journeys=("${ALL_JOURNEYS[@]}")
    fi

    for journey in "${journeys[@]}"; do
        if ! run_journey "$journey"; then
            failures=$((failures + 1))
        fi
    done

    printf '\n=== summary ===\n'
    printf 'journeys run: %d\n' "${#journeys[@]}"
    printf 'failures: %d\n' "$failures"

    [[ "$failures" -eq 0 ]]
}

main "$@"
