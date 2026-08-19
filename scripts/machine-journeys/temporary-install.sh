#!/bin/sh
# temporary-install.sh: journey 2, "install a package temporarily, then throw
# it away."
#
# Uses the staging path a real user would type: nex stage, nex install
# <pkg>, then nex discard. Package chosen: pkg/dev/vcs/tig.yaml
# (outputs/bin) -- already built on the host and pulled into this machine's
# store by the harness before boot (see seed_system_repo() in
# test-machine-operations.sh), so nothing here needs to build, and tig is
# not part of this fixture's own package list, so its presence or absence is
# a real before/after signal, not something already on the machine anyway.
#
# Prints key=value facts on stdout and exits non-zero on failure.
set -u

PKG=pkg/dev/vcs/tig.yaml
TARGET=outputs/bin
BIN=tig
MANIFESTS_SYSTEM=/nex/manifests
WORKTREE=/nex/users/root/manifests

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

cd "$WORKTREE" 2>/dev/null || cd "$MANIFESTS_SYSTEM" || {
    printf 'error-line=%s\n' "no manifests directory to install from"
    exit 1
}

printf 'pre-install-binary-present=%s\n' "$(command -v $BIN >/dev/null 2>&1 && echo true || echo false)"

if ! run_step stage nex stage; then
    exit 1
fi

if ! run_step install nex install "$PKG" "$TARGET" --system; then
    nex discard --force >/dev/null 2>&1
    exit 1
fi

run_out=$($BIN --version 2>&1)
run_exit=$?
printf '%s\n' "$run_out"
printf 'run-exit=%s\n' "$run_exit"
if [ "$run_exit" -ne 0 ]; then
    printf 'error-line=%s\n' "installed binary did not run: $run_out"
    nex discard --force >/dev/null 2>&1
    exit 1
fi

if ! run_step discard nex discard --force; then
    exit 1
fi

post_present=$(command -v $BIN >/dev/null 2>&1 && echo true || echo false)
staging_active=$([ -e /nex/staging/active ] && echo true || echo false)
printf 'post-discard-binary-present=%s\n' "$post_present"
printf 'post-discard-staging-active=%s\n' "$staging_active"

if [ "$post_present" = true ]; then
    printf 'error-line=%s\n' "$BIN still runs after discard"
    exit 1
fi
if [ "$staging_active" = true ]; then
    printf 'error-line=%s\n' "staging still active after discard"
    exit 1
fi

printf 'PASS: temporary-install\n'
