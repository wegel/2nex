#!/bin/sh
# store-upgrade.sh: test 5, "a machine installed before the store was
# renamed keeps its packages after upgrading."
#
# The machine's content store used to sit at /nex/repo and now sits at
# /nex/store. The store is not part of a deployment: it lives on the
# persistent /var and the initramfs binds it into whichever deployment
# boots. So a machine installed under the old name still holds its objects
# under /var/nex/repo, and every deployment built after the rename must
# still find them.
#
# Runs in two phases across one reboot. Phase one renames /var/nex/store
# back to /var/nex/repo, which is exactly the layout an older install left
# behind, and records what the store holds. Phase two boots that machine and
# proves the same objects are reachable, this time under the new name.
#
# The facts compared across the reboot are read from the machine both times
# (object and ref counts, and the commit id a seeded ref points at), never
# supplied by this script. The final check is functional: nex deploy
# --dry-run opens the store and resolves a ref through it, so it fails if
# the store is missing rather than merely misnamed.
#
# Prints key=value facts on stdout and exits non-zero on failure.
set -u

PROBE_REF=systems/nex-systemd/0.0.1
FACTS=/var/lib/store-upgrade-facts
PHASE="${1:?usage: store-upgrade.sh PHASE}"

# Where the store bound onto /nex/store came from, as the kernel reports it:
# field 4 of a mountinfo line is the path inside the source filesystem, so a
# bind of /var/nex/repo reads back as /nex/repo. Nothing this script wrote.
store_mount_root() {
    local root=""
    local mnt_root
    local mnt_point
    while read -r _id _parent _dev mnt_root mnt_point _rest; do
        [ "$mnt_point" = /nex/store ] && root="$mnt_root"
    done < /proc/self/mountinfo
    printf '%s\n' "$root"
}

is_store() {
    [ -d "$1/objects" ] && [ -d "$1/refs" ]
}

object_count() {
    find /nex/store/objects -type f 2>/dev/null | wc -l
}

ref_count() {
    find /nex/store/refs -type f 2>/dev/null | wc -l
}

probe_ref_commit() {
    cat "/nex/store/refs/heads/$PROBE_REF" 2>/dev/null || echo missing
}

run_step() {
    local label=$1
    shift
    local out
    out=$("$@" 2>&1)
    local exit_code=$?
    printf '%s\n' "$out"
    printf '%s-exit=%s\n' "$label" "$exit_code"
    if [ "$exit_code" -ne 0 ]; then
        local error_line
        error_line=$(printf '%s\n' "$out" | grep -iE 'error' | head -n1)
        [ -n "$error_line" ] || error_line=$(printf '%s\n' "$out" | tail -n1)
        printf 'error-line=%s\n' "$error_line"
    fi
    return "$exit_code"
}

printf 'phase=%s\n' "$PHASE"
printf 'store-mount-root=%s\n' "$(store_mount_root)"

case "$PHASE" in
to-legacy)
    if ! is_store /nex/store; then
        printf 'error-line=%s\n' "/nex/store does not hold a store before the rename"
        exit 1
    fi
    if [ "$(store_mount_root)" != /nex/store ]; then
        printf 'error-line=%s\n' "a fresh machine should serve /nex/store from /var/nex/store, not $(store_mount_root)"
        exit 1
    fi

    mkdir -p "$(dirname "$FACTS")"
    {
        printf 'objects=%s\n' "$(object_count)"
        printf 'refs=%s\n' "$(ref_count)"
        printf 'probe=%s\n' "$(probe_ref_commit)"
    } > "$FACTS"
    cat "$FACTS"

    if [ "$(probe_ref_commit)" = missing ]; then
        printf 'error-line=%s\n' "$PROBE_REF is not in the store to begin with"
        exit 1
    fi

    # The store is bind-mounted onto /nex/store, so release it before moving
    # the directory it is served from.
    if ! run_step umount umount /nex/store; then
        exit 1
    fi
    if ! run_step rename mv /var/nex/store /var/nex/repo; then
        exit 1
    fi
    printf 'renamed-to-legacy=true\n'
    ;;
verify-legacy)
    before_objects=$(sed -n 's/^objects=//p' "$FACTS" 2>/dev/null)
    before_refs=$(sed -n 's/^refs=//p' "$FACTS" 2>/dev/null)
    before_probe=$(sed -n 's/^probe=//p' "$FACTS" 2>/dev/null)
    if [ -z "$before_objects" ] || [ -z "$before_refs" ] || [ -z "$before_probe" ]; then
        printf 'error-line=%s\n' "no recorded store facts from before the rename"
        exit 1
    fi

    printf 'legacy-dir-present=%s\n' "$( [ -d /var/nex/repo ] && echo true || echo false )"
    printf 'objects=%s\n' "$(object_count)"
    printf 'refs=%s\n' "$(ref_count)"
    printf 'probe=%s\n' "$(probe_ref_commit)"

    if ! is_store /nex/store; then
        printf 'error-line=%s\n' "/nex/store does not hold a store after upgrading a machine whose store is at /var/nex/repo"
        exit 1
    fi
    if [ "$(store_mount_root)" != /nex/repo ]; then
        printf 'error-line=%s\n' "expected /nex/store to be served from the legacy /var/nex/repo, got $(store_mount_root)"
        exit 1
    fi
    if [ "$(object_count)" != "$before_objects" ]; then
        printf 'error-line=%s\n' "object count changed across the upgrade: $before_objects then $(object_count)"
        exit 1
    fi
    if [ "$(ref_count)" != "$before_refs" ]; then
        printf 'error-line=%s\n' "ref count changed across the upgrade: $before_refs then $(ref_count)"
        exit 1
    fi
    if [ "$(probe_ref_commit)" != "$before_probe" ]; then
        printf 'error-line=%s\n' "$PROBE_REF moved across the upgrade: $before_probe then $(probe_ref_commit)"
        exit 1
    fi

    # The store is not merely present but usable: this resolves $PROBE_REF
    # and reads its checksum metadata through the store itself.
    if ! run_step deploy-dry-run nex deploy --dry-run "$PROBE_REF"; then
        exit 1
    fi

    printf 'PASS: store-upgrade\n'
    ;;
*)
    printf 'error-line=%s\n' "unknown phase: $PHASE"
    exit 1
    ;;
esac
