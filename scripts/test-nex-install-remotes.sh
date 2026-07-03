#!/bin/bash
# Verify nex-install seeds zub remotes from the kernel command line.
set -eu

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
TMPDIR=$(mktemp -d)

cleanup() {
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

CMDLINE="$TMPDIR/cmdline"
TARGET_ROOT="$TMPDIR/root"

cat > "$CMDLINE" <<'EOF'
quiet nex.remote=origin=ssh://root@10.0.2.2/home/builder/.nex/repo nex.remote=backup=wegel@example:/srv/nex/repo nex.remote=invalid
EOF

NEX_INSTALL_SOURCE_ONLY=1 . "$SCRIPT_DIR/nex-install"

NEX_INSTALL_CMDLINE="$CMDLINE" seed_installer_remotes "$TARGET_ROOT"

CONFIG="$TARGET_ROOT/nex/repo/config.toml"
test -f "$CONFIG"
grep -F 'name = "origin"' "$CONFIG" >/dev/null
grep -F 'url = "ssh://root@10.0.2.2/home/builder/.nex/repo"' "$CONFIG" >/dev/null
grep -F 'name = "backup"' "$CONFIG" >/dev/null
grep -F 'url = "wegel@example:/srv/nex/repo"' "$CONFIG" >/dev/null
! grep -F 'name = "invalid"' "$CONFIG" >/dev/null

test "$(cat "$TARGET_ROOT/nex/repo/remotes/origin")" = "ssh://root@10.0.2.2/home/builder/.nex/repo"
test "$(cat "$TARGET_ROOT/nex/repo/remotes/backup")" = "wegel@example:/srv/nex/repo"

printf "nex-install remote seeding passed\n"
