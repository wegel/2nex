#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/.diag-harness-funcs.sh"

require_tools
ensure_assert_key

artifact_dir="$ARTIFACT_ROOT/unshare-diag"
overlay_img="$artifact_dir/overlay.qcow2"
rm -rf "$artifact_dir"
mkdir -p "$artifact_dir"

echo "==> creating overlay over $BACKING_IMG"
"$QEMU_IMG_BIN" create -q -f qcow2 -F raw -b "$BACKING_IMG" "$overlay_img" >/dev/null

echo "==> booting"
boot "$overlay_img" 10999 "$artifact_dir"
echo "==> SSH ready after ${BOOT_WAITED_SECS}s"

echo "=== stray processes (ps aux) ==="
run 'ps aux' || true

echo "=== kernel namespace support ==="
run 'ls -la /proc/self/ns/ 2>&1'
run 'cat /proc/sys/user/max_user_namespaces 2>&1'
run 'cat /proc/sys/kernel/unprivileged_userns_clone 2>&1 || echo "(no such sysctl)"'
run 'uname -a'

echo "=== bisecting unshare flags ==="
for combo in \
  "--user" \
  "--pid" \
  "--mount" \
  "--uts" \
  "--ipc" \
  "--net" \
  "--fork" \
  "--map-root-user" \
  "--user --map-root-user" \
  "--user --pid" \
  "--user --mount" \
  "--user --uts" \
  "--user --ipc" \
  "--user --net" \
  "--user --fork" \
  "--user --pid --fork" \
  "--user --pid --mount --fork" \
  "--user --pid --mount --uts --fork" \
  "--user --pid --mount --uts --fork --ipc" \
  "--user --pid --mount --uts --fork --ipc --net" \
  "--user --pid --mount --uts --fork --ipc --net --map-root-user" \
  ; do
  echo "--- unshare $combo -- true ---"
  run "unshare $combo -- true; echo exit=\$?" 2>&1
done

echo "=== kernel .config for namespace options ==="
run 'zcat /proc/config.gz 2>&1 | grep -E "CONFIG_IPC_NS|CONFIG_SYSVIPC|CONFIG_POSIX_MQUEUE|CONFIG_UTS_NS|CONFIG_PID_NS|CONFIG_USER_NS|CONFIG_NET_NS|CONFIG_NAMESPACES" || echo NOCONFIGGZ'
run 'dmesg 2>&1 | grep -i "ipc\|namespace" | head -20 || true'

echo "=== done, stopping guest ==="
stop_guest
echo "artifact_dir=$artifact_dir"
