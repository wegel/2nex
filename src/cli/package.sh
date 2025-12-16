#!/bin/bash
# creates a reproducible source tarball for nex builder
# the tarball is placed at pkg/core/nex/nex-builder-src.tar.gz

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT="$REPO_ROOT/pkg/core/nex/nex-builder-src.tar.gz"

cd "$SCRIPT_DIR"

# re-vendor dependencies and generate cargo config
rm -rf vendor
mkdir -p .cargo
cargo vendor vendor 2>/dev/null > .cargo/config.toml

# set reproducible timestamp (matches SOURCE_DATE_EPOCH in build)
TIMESTAMP="2024-01-01T00:00:00Z"

# create reproducible tarball
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

# update sha256 in nex.yaml
NEX_YAML="$REPO_ROOT/pkg/core/nex/nex.yaml"
sed -i "s/^  sha256: [a-f0-9]\{64\}$/  sha256: $SHA256/" "$NEX_YAML"
echo "Updated: $NEX_YAML"
