#!/usr/bin/env bash
# Prove the live-upgrade checkout path hardlinks files from /nex/repo.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
NEX_BIN="${NEX_BIN:-$ROOT_DIR/src/cli/target/debug/nex}"
TMP_PARENT="${TMP_PARENT:-$ROOT_DIR/.nex/tmp}"
KEEP_WORK="${KEEP_LIVE_UPGRADE_HARDLINK_WORK:-0}"
REF_NAME="${REF_NAME:-systems/hardlink-smoke/0.0.1}"
WORK_DIR="${NEX_HARDLINK_WORK_DIR:-}"
ZUB_BIN="${ZUB_BIN:-}"

die() {
    printf 'error: %s\n' "$*" >&2
    if [[ -n "$WORK_DIR" ]]; then
        printf 'work dir: %s\n' "$WORK_DIR" >&2
    fi
    exit 1
}

need_tool() {
    command -v "$1" >/dev/null 2>&1 || die "$1 not found"
}

cleanup() {
    if [[ "$KEEP_WORK" != 1 && -n "$WORK_DIR" ]]; then
        rm -rf "$WORK_DIR"
    elif [[ -n "$WORK_DIR" ]]; then
        printf 'kept work dir: %s\n' "$WORK_DIR" >&2
    fi
}
trap cleanup EXIT

need_tool find
need_tool stat
need_tool unshare
[[ -x "$NEX_BIN" ]] || die "nex binary is not executable: $NEX_BIN"

if [[ "${NEX_HARDLINK_IN_USERNS:-0}" != 1 ]]; then
    need_tool zub
    mkdir -p "$TMP_PARENT"
    WORK_DIR="$(mktemp -d "$TMP_PARENT/live-upgrade-hardlink.XXXXXX")"
    cp "$(command -v zub)" "$WORK_DIR/zub"
    chmod 0755 "$WORK_DIR/zub"
    export NEX_HARDLINK_WORK_DIR="$WORK_DIR"
    export ZUB_BIN="$WORK_DIR/zub"
    export NEX_HARDLINK_IN_USERNS=1
    exec unshare --user --map-root-user -- "$0" "$@"
fi
[[ -x "$ZUB_BIN" ]] || die "zub binary is not executable: $ZUB_BIN"

mkdir -p "$TMP_PARENT"
if [[ -z "$WORK_DIR" ]]; then
    WORK_DIR="$(mktemp -d "$TMP_PARENT/live-upgrade-hardlink.XXXXXX")"
fi

repo="$WORK_DIR/sysroot/nex/repo"
sysroot="$WORK_DIR/sysroot"
source_root="$WORK_DIR/source"
probe_name="nex-hardlink-probe"
probe_path="usr/bin/$probe_name"

mkdir -p "$repo" "$sysroot/nex/deployments" "$source_root/usr/bin"
cat > "$source_root/$probe_path" <<'PROBE'
#!/bin/sh
echo nex-hardlink-probe
PROBE
chmod 0755 "$source_root/$probe_path"

"$ZUB_BIN" init "$repo" >/dev/null
"$ZUB_BIN" --repo "$repo" commit --ref-name "$REF_NAME" "$source_root" >/dev/null

commit_hash="$("$ZUB_BIN" --repo "$repo" rev-parse "$REF_NAME")"
"$NEX_BIN" upgrade "$REF_NAME" \
    --repo "$repo" \
    --sysroot "$sysroot" \
    --allow-commit-hash >/dev/null

deployment="$sysroot/nex/deployments/${commit_hash}.1"
deployed_file="$deployment/$probe_path"
[[ -f "$deployed_file" ]] || die "deployed probe file is missing: $deployed_file"

link_count="$(stat -c '%h' "$deployed_file")"
repo_blob="$(find "$repo/objects/blobs" -type f -samefile "$deployed_file" -print -quit)"

if [[ "$link_count" -le 1 || -z "$repo_blob" ]]; then
    deployed_inode="$(stat -c '%d:%i' "$deployed_file")"
    printf 'deployed-file: %s\n' "$deployed_file" >&2
    printf 'deployed-inode: %s\n' "$deployed_inode" >&2
    printf 'deployed-link-count: %s\n' "$link_count" >&2
    if [[ -n "$repo_blob" ]]; then
        printf 'repo-blob: %s\n' "$repo_blob" >&2
        printf 'repo-blob-inode: %s\n' "$(stat -c '%d:%i' "$repo_blob")" >&2
    else
        printf 'repo-blob: not samefile as deployed file\n' >&2
    fi
    die "live upgrade checkout did not hardlink the deployed file to the repo blob"
fi

printf 'live-upgrade-hardlink-pass file=%s links=%s blob=%s\n' \
    "$deployed_file" \
    "$link_count" \
    "$repo_blob"
