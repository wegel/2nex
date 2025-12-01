#!/bin/bash
# creates a reproducible source tarball for nex builder
# the tarball is placed at pkg/core/nex/nex-builder-src.tar.gz

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT="$REPO_ROOT/pkg/core/nex/nex-builder-src.tar.gz"

cd "$SCRIPT_DIR"

# copy zub into builder directory for packaging
rm -rf zub
cp -r ../zub zub
# remove zub's target directory if present
rm -rf zub/target

# update Cargo.toml to use local zub path (for tarball)
sed -i 's|path = "../zub"|path = "zub"|' Cargo.toml

# re-vendor dependencies (ensures Cargo.lock is up to date)
rm -rf vendor
cargo vendor vendor > /dev/null 2>&1

# set reproducible timestamp (matches SOURCE_DATE_EPOCH in build)
TIMESTAMP="2024-01-01T00:00:00Z"

# create tarball with zub path pointing to embedded copy
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

# restore Cargo.toml to use original path (for local development)
sed -i 's|path = "zub"|path = "../zub"|' Cargo.toml

# clean up copied zub directory
rm -rf zub

SHA256=$(sha256sum "$OUTPUT" | cut -d' ' -f1)
echo "Created: $OUTPUT"
echo "SHA256: $SHA256"

# update sha256 in nex.yaml
NEX_YAML="$REPO_ROOT/pkg/core/nex/nex.yaml"
sed -i "s/^  sha256: [a-f0-9]\{64\}$/  sha256: $SHA256/" "$NEX_YAML"
echo "Updated: $NEX_YAML"
