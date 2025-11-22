#!/bin/bash
# update_all_requires.sh
# updates the runtime requires for all non-bootstrap packages

set -e

REPO_PATH="${1:-bootstrap_store}"
BUILDER="./src/builder/target/debug/nex"

# ensure builder is built
if [ ! -f "$BUILDER" ]; then
    echo "builder not found, building..."
    cargo build --manifest-path src/builder/Cargo.toml
fi

# find all non-bootstrap manifests
find pkg -name "*.yaml" -type f | \
    grep -v "pkg/bootstrap/" | \
    sort | \
while read -r manifest; do
    echo "=== processing $manifest ==="

    # update requires (will auto-build if not built, or just update if already built)
    if ! "$BUILDER" "$REPO_PATH" "$manifest" --single --update-outputs-requires 2>&1 | tail -5; then
        echo "failed to update $manifest"
        continue
    fi

    echo ""
done

echo "=== done updating all requires ==="
