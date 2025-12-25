#!/bin/bash
# delete all non-bootstrap refs and artifacts from the zub store

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
REPO="${REPO:-.nex/repo}"
ZUB="zub"

# delete non-bootstrap refs (anything not containing "bootstrap" in the path)
$ZUB --repo="$REPO" refs | awk '{print $2}' | grep -vE "bootstrap" | while read ref; do
  echo "Deleting ref: $ref"
  $ZUB --repo="$REPO" delete-ref "$ref"
done

# delete non-bootstrap artifacts
# new format: arch/namespace/.../manifest_hash/output (use */pkg/* pattern)
# old format: manifest_hash/output (delete all, they're a cache)
echo "Deleting non-bootstrap artifacts..."
$ZUB --repo="$REPO" delete-artifacts "*/pkg/*" 2>/dev/null || true

# clean up old-format artifacts (pre-migration)
ARTIFACTS_DIR="$REPO/refs/artifacts"
if [ -d "$ARTIFACTS_DIR" ]; then
  # old artifacts start with a 64-char hash, new ones start with arch (x86_64/...)
  for dir in "$ARTIFACTS_DIR"/[0-9a-f]*; do
    if [ -d "$dir" ]; then
      echo "Deleting old-format artifacts: $(basename "$dir")"
      rm -rf "$dir"
    fi
  done
fi

echo "Done. Running garbage collection..."
$ZUB --repo="$REPO" gc
