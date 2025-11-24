#!/usr/bin/env bash
set -euo pipefail

# Simple smoke test for manifest_ref pinning and time-travel builds.
# Adjust these defaults to your target package.

REPO=${REPO:-bootstrap_store}
MANIFEST=${MANIFEST:-pkg/core/init/systemd.yaml}
DEP_MANIFEST=${DEP_MANIFEST:-pkg/bootstrap/phase1/bash.yaml}
# Prefix for the package you’re testing (arch/namespace/slug/version)
PREFIX=${PREFIX:-x86_64/pkg/core/init/systemd/257.5}
# Outputs to compare (trim/add as needed for the package)
OUTPUTS=("bin" "lib" "dev" "doc" "misc" "conf")

capture_commits() {
  local prefix="$1"
  local repo="$2"
  local out commits=()
  for out in "${OUTPUTS[@]}"; do
    commits+=( "$(ostree rev-parse --repo "$repo" "${prefix}/outputs/${out}" 2>/dev/null || true)" )
  done
  printf '%s\n' "${commits[@]}"
}

echo "Building builder..."
cargo build --manifest-path src/builder/Cargo.toml >/dev/null

echo "Stamping manifest_ref entries..."
./src/builder/target/debug/nex link "$MANIFEST"

echo "Baseline build..."
./src/builder/target/debug/nex "$REPO" "$MANIFEST"

echo "Capturing baseline commits..."
before=$(capture_commits "$PREFIX" "$REPO")

echo "Mutating dependency manifest on disk (but keeping manifest_ref pinned)..."
cp "$DEP_MANIFEST" "${DEP_MANIFEST}.bak"
echo "# mutation for time-travel test" >> "$DEP_MANIFEST"

echo "Re-running build (should reuse old commits via manifest_ref/manifest hash)..."
./src/builder/target/debug/nex "$REPO" "$MANIFEST"

echo "Capturing commits after mutation..."
after=$(capture_commits "$PREFIX" "$REPO")

if [[ "$before" == "$after" ]]; then
  echo "SUCCESS: Outputs still point at the same commits; manifest_ref pinning works."
else
  echo "FAILED: Output commits changed:"
  diff <(echo "$before") <(echo "$after") || true
fi

echo "Restoring dependency manifest..."
mv "${DEP_MANIFEST}.bak" "$DEP_MANIFEST"
