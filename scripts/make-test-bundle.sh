#!/usr/bin/env bash
# Generate the Git bundle that tests/nex-test-fixture.yaml ships.
#
# The bundle carries this repository's full history so a test machine can
# resolve the historical environment blobs that package manifests pin. It is
# NOT committed: a bundle of this repository, stored in this repository, would
# add its whole size to history on every regeneration.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT_DIR/tests/nex-test-fixture.bundle}"
COMMIT="${2:-HEAD}"

resolved=$(git -C "$ROOT_DIR" rev-parse --verify "${COMMIT}^{commit}")

# git bundle create needs a named ref: passing a bare commit SHA fails with
# "empty bundle", and passing a branch name produces a bundle whose clone
# leaves an empty tree. A detached worktree gives us a HEAD to bundle.
work_dir=$(mktemp -d)
cleanup() {
    git -C "$ROOT_DIR" worktree remove --force "$work_dir" >/dev/null 2>&1 || true
    rm -rf "$work_dir"
}
trap cleanup EXIT

git -C "$ROOT_DIR" worktree add --detach --quiet "$work_dir" "$resolved"

# pack.threads=1 makes the output byte-reproducible. Three default runs produce
# three different sha256 values; parallel work-splitting is the nondeterminism,
# not the content, so delta compression stays on and the size is unchanged.
git -C "$work_dir" -c pack.threads=1 bundle create "$OUT" HEAD >/dev/null

git bundle verify "$OUT" >/dev/null

printf 'bundle: %s\n' "$OUT"
printf 'commit: %s\n' "$resolved"
printf 'sha256: %s\n' "$(sha256sum "$OUT" | awk '{print $1}')"
printf 'bytes:  %s\n' "$(stat -c %s "$OUT")"
