#!/usr/bin/env bash
# Verify scripts/dogfood-upgrade pulls a built ref and gates apply behind --apply.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TMPDIR="$(mktemp -d)"

cleanup() {
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

FAKEBIN="$TMPDIR/bin"
SOURCE_REPO="$TMPDIR/source-repo"
TARGET_REPO="$TMPDIR/target-repo"
SYSROOT="$TMPDIR/sysroot"
LOG="$TMPDIR/commands.log"
SYSTEM_REF="systems/desktop-vwl-nvidia-580/0.0.1"
SYSTEM_COMMIT="c04279c83d1caa49f1956623ed8f8e77d0db8c67bea34afa96043619701af3d1"

mkdir -p \
    "$FAKEBIN" \
    "$SOURCE_REPO/refs/systems/desktop-vwl-nvidia-580" \
    "$TARGET_REPO/refs/systems/desktop-vwl-nvidia-580" \
    "$SYSROOT/nex/deployments"
printf '%s\n' "$SYSTEM_COMMIT" > "$SOURCE_REPO/refs/$SYSTEM_REF"
: > "$LOG"

cat > "$FAKEBIN/zub" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
repo=
if [[ "${1:-}" == "--repo" ]]; then
    repo=$2
    shift 2
elif [[ "${1:-}" == --repo=* ]]; then
    repo=${1#--repo=}
    shift
fi
cmd=$1
shift
case "$cmd" in
    show-ref)
        ref=$1
        cat "$repo/refs/$ref"
        ;;
    pull)
        source_repo=$1
        ref=$2
        mkdir -p "$repo/refs/$(dirname "$ref")"
        cp "$source_repo/refs/$ref" "$repo/refs/$ref"
        printf 'pulled %s from %s\n' "$(cat "$repo/refs/$ref")" "$source_repo"
        ;;
    *)
        printf 'unexpected zub command: %s\n' "$cmd" >&2
        exit 1
        ;;
esac
EOF

cat > "$FAKEBIN/nex" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "nex \$*" >> "$LOG"
case "\${1:-}" in
    build)
        exit 0
        ;;
    check)
        exit 0
        ;;
    upgrade)
        dry_run=0
        ref=\$2
        repo=
        sysroot=
        while [[ \$# -gt 0 ]]; do
            case "\$1" in
                --dry-run) dry_run=1 ;;
                --repo) shift; repo=\$1 ;;
                --sysroot) shift; sysroot=\$1 ;;
            esac
            shift
        done
        test -f "\$repo/refs/\$ref"
        if [[ "\$dry_run" -eq 1 ]]; then
            printf 'Target: %s/nex/deployments/%s.1\n' "\$sysroot" "$SYSTEM_COMMIT"
        else
            mkdir -p "\$sysroot/nex/deployments/$SYSTEM_COMMIT.1"
            printf 'Deployed: %s.1\n' "$SYSTEM_COMMIT"
        fi
        ;;
    *)
        printf 'unexpected nex command: %s\n' "\${1:-}" >&2
        exit 1
        ;;
esac
EOF

cat > "$FAKEBIN/ssh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "ssh \$*" >> "$LOG"
if [[ "\${1:-}" == "-i" ]]; then
    shift 2
fi
shift
"\$@"
EOF

chmod 0755 "$FAKEBIN/zub" "$FAKEBIN/nex" "$FAKEBIN/ssh"

PATH="$FAKEBIN:$PATH" \
NEX_BIN="$FAKEBIN/nex" \
ZUB_BIN="$FAKEBIN/zub" \
SOURCE_REPO="$SOURCE_REPO" \
TARGET_REPO="$TARGET_REPO" \
SYSROOT="$SYSROOT" \
FALLBACK_REPOS="$TARGET_REPO" \
NEX_DOGFOOD_ALLOW_NON_ROOT=1 \
    "$ROOT_DIR/scripts/dogfood-upgrade" > "$TMPDIR/dry-run.out"

test "$(cat "$TARGET_REPO/refs/$SYSTEM_REF")" = "$SYSTEM_COMMIT"
grep -F "dry-run passed" "$TMPDIR/dry-run.out" >/dev/null
grep -F "nex build pkg/core/nex/nex.yaml" "$LOG" >/dev/null
grep -F -- "--compute-deps" "$LOG" >/dev/null
grep -F -- "--fallback-repo $TARGET_REPO" "$LOG" >/dev/null
grep -F "nex build asm/desktop-vwl-nvidia-580.yaml" "$LOG" >/dev/null
grep -F "nex upgrade $SYSTEM_REF --repo $TARGET_REPO --sysroot $SYSROOT --dry-run" "$LOG" >/dev/null
! test -d "$SYSROOT/nex/deployments/$SYSTEM_COMMIT.1"

PATH="$FAKEBIN:$PATH" \
NEX_BIN="$FAKEBIN/nex" \
ZUB_BIN="$FAKEBIN/zub" \
SOURCE_REPO="$SOURCE_REPO" \
TARGET_REPO="$TARGET_REPO" \
SYSROOT="$SYSROOT" \
TARGET_SSH="root@127.0.0.1" \
NEX_DOGFOOD_ALLOW_NON_ROOT=1 \
    "$ROOT_DIR/scripts/dogfood-upgrade" --no-build --apply > "$TMPDIR/apply.out"

test -d "$SYSROOT/nex/deployments/$SYSTEM_COMMIT.1"
grep -F "deployment created" "$TMPDIR/apply.out" >/dev/null
grep -F "ssh root@127.0.0.1 $FAKEBIN/zub --repo $TARGET_REPO pull $SOURCE_REPO $SYSTEM_REF" "$LOG" >/dev/null
grep -F "nex upgrade $SYSTEM_REF --repo $TARGET_REPO --sysroot $SYSROOT" "$LOG" >/dev/null

printf 'dogfood-upgrade test passed\n'
