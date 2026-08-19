#!/bin/sh
# deploy-and-rollback.sh: journey 4, "put a new system version on the
# machine so it boots next time, and undo that."
#
# Runs in four phases, driven by the host across two reboots (the host owns
# reboot; nothing in the guest can reboot itself and keep reporting).
# Deploy target: systems/nex-systemd/0.0.1, a different already-built system
# (pulled into this machine's store by the harness before boot; see
# seed_system_repo() in test-machine-operations.sh) -- no build needed.
#
# Content markers distinguish the two systems without reading back a value
# this test supplied: /usr/share/nex/nex.bundle and the git binary exist
# only on the fixture (systems/nex-test-fixture/0.0.1) being tested, not on
# plain nex-systemd. Which deployment actually booted is read from the
# kernel's own /proc/cmdline (the same zub= convention nex rollback itself
# uses to find "current" -- see current_deployment_from_cmdline() in
# src/cli/src/commands/rollback.rs), not from anything this script wrote.
#
# Prints key=value facts on stdout and exits non-zero on failure.
set -u

DEPLOY_REF=systems/nex-systemd/0.0.1
PHASE="${1:?usage: deploy-and-rollback.sh PHASE}"

current_deployment() {
    sed -n 's/.*zub=\([^ ]*\).*/\1/p' /proc/cmdline | sed 's#.*/##'
}

fixture_markers() {
    printf 'bundle-present=%s\n' "$( [ -e /usr/share/nex/nex.bundle ] && echo true || echo false )"
    printf 'git-present=%s\n' "$( command -v git >/dev/null 2>&1 && echo true || echo false )"
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
printf 'current-deployment=%s\n' "$(current_deployment)"

case "$PHASE" in
deploy)
    fixture_markers
    if [ ! -e /usr/share/nex/nex.bundle ] || ! command -v git >/dev/null 2>&1; then
        printf 'error-line=%s\n' "starting deployment is missing its own fixture markers"
        exit 1
    fi
    run_step deploy nex deploy "$DEPLOY_REF"
    ;;
verify-deployed)
    fixture_markers
    if [ -e /usr/share/nex/nex.bundle ] || command -v git >/dev/null 2>&1; then
        printf 'error-line=%s\n' "fixture markers still present after deploying $DEPLOY_REF; wrong deployment booted"
        exit 1
    fi
    printf 'PASS: deploy-and-rollback (verify-deployed)\n'
    ;;
rollback)
    run_step rollback nex rollback --yes
    ;;
verify-rolled-back)
    fixture_markers
    if [ ! -e /usr/share/nex/nex.bundle ] || ! command -v git >/dev/null 2>&1; then
        printf 'error-line=%s\n' "fixture markers missing after rollback; did not return to the fixture deployment"
        exit 1
    fi
    printf 'PASS: deploy-and-rollback\n'
    ;;
*)
    printf 'error-line=%s\n' "unknown phase: $PHASE"
    exit 1
    ;;
esac
