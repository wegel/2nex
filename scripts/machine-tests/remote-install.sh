#!/bin/sh
# remote-install.sh: install a package whose target and runtime closure exist
# only in the configured read-only host store.
set -u

PKG=pkg/dev/vcs/tig.yaml
TARGET=outputs/bin
TARGET_REF=x86_64/pkg/dev/vcs/tig/2.6.0/outputs/bin
PRIMARY=/nex/store
REMOTE=/run/nex-host-store
WORKTREE=/nex/users/root/manifests
MANIFESTS=/nex/manifests

run_step() {
    label=$1
    shift
    output=$({ "$@"; } 2>&1)
    status=$?
    printf '%s\n' "$output"
    printf '%s-exit=%s\n' "$label" "$status"
    [ "$status" -eq 0 ] || return "$status"
}

cd "$WORKTREE" 2>/dev/null || cd "$MANIFESTS" || exit 1

remote_target=$(/usr/bin/zub --repo "$REMOTE" rev-parse "$TARGET_REF") || exit 1
printf 'remote-target-before=%s\n' "$remote_target"
if /usr/bin/zub --repo "$PRIMARY" rev-parse "$TARGET_REF" >/dev/null 2>&1; then
    printf 'error-line=%s\n' "target already present in primary store"
    exit 1
fi

closure_output=$(nex resolve tig --repo "$REMOTE" -v 2>&1) || {
    printf '%s\n' "$closure_output"
    exit 1
}
closure_refs=$(printf '%s\n' "$closure_output" |
    grep -oE 'x86_64/pkg/[^ ]+/files' | sort -u)
[ -n "$closure_refs" ] || {
    printf 'error-line=%s\n' "remote tig closure was empty"
    exit 1
}
for ref in $closure_refs; do
    /usr/bin/zub --repo "$REMOTE" rev-parse "$ref" >/dev/null || exit 1
    if /usr/bin/zub --repo "$PRIMARY" rev-parse "$ref" >/dev/null 2>&1; then
        printf 'error-line=runtime ref already present in primary store: %s\n' "$ref"
        exit 1
    fi
done
printf 'primary-before=target-and-runtime-closure-absent\n'

run_step stage nex stage || exit 1
run_step install nex install "$PKG" "$TARGET" --system || exit 1
printf '%s\n' "$output" |
    grep -Eq "Pulled ${TARGET_REF} from 'host-machine-test': [1-9][0-9]* bytes, [1-9][0-9]* objects" || {
        printf 'error-line=%s\n' "install did not report a nonzero target pull"
        exit 1
    }

primary_target=$(/usr/bin/zub --repo "$PRIMARY" rev-parse "$TARGET_REF") || exit 1
printf 'primary-target-after=%s\n' "$primary_target"
printf 'remote-target-after=%s\n' "$remote_target"
[ "$primary_target" = "$remote_target" ] || {
    printf 'error-line=%s\n' "primary and remote target commits differ"
    exit 1
}

for ref in $closure_refs; do
    /usr/bin/zub --repo "$PRIMARY" rev-parse "$ref" >/dev/null || {
        printf 'error-line=runtime ref was not pulled into primary store: %s\n' "$ref"
        exit 1
    }
done

run_step run tig --version || exit 1
printf '%s\n' "$output" | grep -q '^tig version 2\.6\.0$' || {
    printf 'error-line=%s\n' "installed tig returned the wrong version"
    exit 1
}
run_step discard nex discard --force || exit 1

printf 'PASS: remote-install\n'
