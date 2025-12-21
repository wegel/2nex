#!/bin/bash
set -euvo pipefail

cd "$(dirname "$0")/../../.."

echo "=== preparing linux.yaml for output regeneration ==="
./pkg/core/kernel/toggle-outputs.py prepare

echo "=== building kernel ==="
time ./nex build pkg/core/kernel/linux.yaml --update-checksum --record-profile --verbose --force --single

echo "=== extracting checksum ==="
checksum=$(grep "checksum:" pkg/core/kernel/linux.yaml | awk '{print $2}')
echo "checksum: $checksum"

echo "=== categorizing modules ==="
zub ls-tree -r "x86_64/pkg/core/kernel/linux/6.12.58/${checksum}/files" \
    | grep '\.ko' \
    | awk '{print $4}' \
    | python3 pkg/core/kernel/categorize-modules.py >> pkg/core/kernel/linux.yaml

echo "=== restoring bundles ==="
./pkg/core/kernel/toggle-outputs.py restore

echo "=== refreshing metadata ==="
./nex build pkg/core/kernel/linux.yaml --refresh-metadata

echo "=== done ==="
