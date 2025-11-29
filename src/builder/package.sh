#!/bin/bash
# creates a reproducible source tarball for nex builder
# the tarball is placed at pkg/core/nex/nex-builder-src.tar.gz

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT="$REPO_ROOT/pkg/core/nex/nex-builder-src.tar.gz"

cd "$SCRIPT_DIR"

# re-vendor dependencies (ensures Cargo.lock is up to date)
rm -rf vendor
cargo vendor vendor > /dev/null 2>&1

# set reproducible timestamp (matches SOURCE_DATE_EPOCH in build)
TIMESTAMP="2024-01-01T00:00:00Z"

# create tarball with reproducible settings:
# - exclude .git directory
# - sort files deterministically
# - use fixed mtime for all files
# - use fixed owner/group
# - use gzip with no timestamp
find . -type f -o -type l | \
    grep -v '^\./\.git' | \
    grep -v '^\./target' | \
    LC_ALL=C sort | \
    tar --create \
        --file=- \
        --directory="$SCRIPT_DIR" \
        --mtime="$TIMESTAMP" \
        --owner=0 --group=0 \
        --numeric-owner \
        --no-recursion \
        --files-from=- | \
    gzip -n -9 > "$OUTPUT"

# also include directories (tar needs them for extraction)
find . -type d | \
    grep -v '^\./\.git' | \
    grep -v '^\./target' | \
    LC_ALL=C sort | \
    tar --create \
        --file=- \
        --directory="$SCRIPT_DIR" \
        --mtime="$TIMESTAMP" \
        --owner=0 --group=0 \
        --numeric-owner \
        --no-recursion \
        --files-from=- | \
    gzip -n -9 > "${OUTPUT}.dirs"

# combine: extract dirs tarball first, then files
# actually simpler: just do it all in one pass
rm -f "${OUTPUT}.dirs"

# proper approach: include everything in sorted order
(
    find . -type d | grep -v '^\./\.git' | grep -v '^\./target'
    find . -type f -o -type l | grep -v '^\./\.git' | grep -v '^\./target'
) | LC_ALL=C sort -u | \
    tar --create \
        --file=- \
        --directory="$SCRIPT_DIR" \
        --mtime="$TIMESTAMP" \
        --owner=0 --group=0 \
        --numeric-owner \
        --no-recursion \
        --files-from=- | \
    gzip -n -9 > "$OUTPUT"

SHA256=$(sha256sum "$OUTPUT" | cut -d' ' -f1)
echo "Created: $OUTPUT"
echo "SHA256: $SHA256"
echo ""
echo "Update pkg/core/nex/nex.yaml with:"
echo "  sha256: $SHA256"
