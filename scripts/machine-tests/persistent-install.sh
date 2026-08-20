#!/bin/sh
# persistent-install.sh: test 3, "install a package and keep it across a
# reboot."
#
# Same package and staging path as test 2 (temporary-install.sh), but
# nex commit instead of nex discard, then the host reboots the guest and
# this script runs again with "after-reboot" to check the package survived.
#
# Prints key=value facts on stdout and exits non-zero on failure.
set -u

PKG=pkg/dev/vcs/tig.yaml
TARGET=outputs/bin
BIN=tig
MANIFESTS_SYSTEM=/nex/manifests
WORKTREE=/nex/users/root/manifests
PHASE="${1:-before-reboot}"

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

if [ "$PHASE" = "before-reboot" ]; then
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

    if ! run_step commit nex commit -m "install $BIN for persistent-install test"; then
        exit 1
    fi

    printf 'PASS: persistent-install (before-reboot)\n'
    exit 0
fi

# after-reboot
current_deployment=$(sed -n 's/.*zub=\([^ ]*\).*/\1/p' /proc/cmdline | sed 's#.*/##')
printf 'current-deployment=%s\n' "$current_deployment"
post_present=$(command -v $BIN >/dev/null 2>&1 && echo true || echo false)
printf 'post-reboot-binary-present=%s\n' "$post_present"

if [ "$post_present" != true ]; then
    printf 'error-line=%s\n' "$BIN not present after reboot"
    exit 1
fi

run_out=$($BIN --version 2>&1)
run_exit=$?
printf '%s\n' "$run_out"
printf 'post-reboot-run-exit=%s\n' "$run_exit"
if [ "$run_exit" -ne 0 ]; then
    printf 'error-line=%s\n' "installed binary did not run after reboot: $run_out"
    exit 1
fi

printf 'PASS: persistent-install\n'
