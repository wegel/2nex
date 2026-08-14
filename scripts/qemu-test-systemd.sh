#!/usr/bin/env bash
# Boot a systemd assembly with the direct-initramfs assertion test.
# Usage: qemu-test-systemd.sh [system-ref] [qemu-test-installer options]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

export TARGET_REF="${1:-systems/flat-systemd/0.0.1}"
export LINUX_BOOT_REF="${LINUX_BOOT_REF:-x86_64/pkg/core/kernel/linux/6.18.24/outputs/boot}"

if (($# > 0)); then
    shift
fi

exec "$SCRIPT_DIR/qemu-test-installer.sh" \
    --direct-initramfs \
    --assert-boot \
    --headless \
    "$@"
