#!/bin/sh
# build-package.sh: test 1, "build a package on my machine."
#
# Runs on the guest, copied there by scripts/test-machine-operations.sh. Prints
# key=value facts on stdout and exits non-zero on failure. The host parses no
# prose, only these lines.
#
# What this checks: whether a person sitting at an installed Nex machine can
# type `nex build pkg/cli/archive/gzip.yaml --single` and have it work, using
# only what the machine itself carries (/nex/manifests and whatever worktree
# nex derives from it). It does not stage anything from outside the guest.
set -u

PKG=pkg/cli/archive/gzip.yaml
MANIFESTS_SYSTEM=/nex/manifests
USER_NAME=$(whoami)
WORKTREE="/nex/users/${USER_NAME}/manifests"

printf 'user=%s\n' "$USER_NAME"
printf 'nex-bin=%s\n' "$(command -v nex || echo missing)"
printf 'manifests-system-exists=%s\n' "$( [ -d "$MANIFESTS_SYSTEM" ] && echo true || echo false )"
printf 'manifests-system-is-git=%s\n' "$( [ -d "$MANIFESTS_SYSTEM/.git" ] && echo true || echo false )"
printf 'manifests-commit-count=%s\n' "$(git -C "$MANIFESTS_SYSTEM" rev-list --count HEAD 2>/dev/null || echo unknown)"
# 476 manifests name their build environment by this historical blob, so a
# machine that clones only recent history cannot build any of them. Ask for
# the object's type rather than its mere presence: the answer must be `blob`.
pinned_env_blob_type=$(git -C "$MANIFESTS_SYSTEM" cat-file -t 27b6e5dc7ad152c9a17c2cabfcc5ee93daa9bbf0 2>/dev/null || echo missing)
printf 'manifests-pinned-env-blob-type=%s\n' "$pinned_env_blob_type"
if [ "$pinned_env_blob_type" != blob ]; then
    printf 'error-line=%s\n' "the pinned build environment is not in this machine's manifests: cat-file -t says $pinned_env_blob_type"
    exit 1
fi
printf 'manifests-worktree-exists=%s\n' "$( [ -d "$WORKTREE" ] && echo true || echo false )"

build_cwd=""
if [ -d "$WORKTREE" ]; then
    build_cwd="$WORKTREE"
elif [ -d "$MANIFESTS_SYSTEM" ]; then
    build_cwd="$MANIFESTS_SYSTEM"
fi

if [ -z "$build_cwd" ]; then
    printf 'build-cwd=none\n'
    printf 'error-line=%s\n' "no manifests directory exists on this machine"
    printf 'build-exit=127\n'
    exit 127
fi

printf 'build-cwd=%s\n' "$build_cwd"
if ! cd "$build_cwd"; then
    printf 'error-line=%s\n' "cd $build_cwd failed"
    printf 'build-exit=126\n'
    exit 126
fi

printf 'pkg-manifest-exists=%s\n' "$( [ -f "$PKG" ] && echo true || echo false )"

out=$(nex build "$PKG" --single 2>&1)
build_exit=$?

printf '%s\n' "$out"
printf 'build-exit=%s\n' "$build_exit"

if [ "$build_exit" -ne 0 ]; then
    error_line=$(printf '%s\n' "$out" | grep -i 'error' | head -n1)
    if [ -z "$error_line" ]; then
        error_line=$(printf '%s\n' "$out" | tail -n1)
    fi
    printf 'error-line=%s\n' "$error_line"
    exit "$build_exit"
fi

output_ref=$(printf '%s\n' "$out" | grep -Eom1 '[0-9a-f]{40,64}' | head -n1)
printf 'output-ref=%s\n' "${output_ref:-unknown}"
printf 'PASS: build-package\n'
