#!/bin/bash
set -euvo pipefail

cd "$(dirname "$0")/../../.."

if [ -z "${SKIP_PREPARE:-}" ]; then
	echo "=== preparing linux.yaml for output regeneration ==="
	./pkg/core/kernel/toggle-outputs.py prepare
fi

echo "=== building kernel ==="
time ./nex build pkg/core/kernel/linux.yaml \
	--single --update-checksum --record-profile --verbose --force

echo "=== extracting checksum ==="
checksum=$(grep "checksum:" pkg/core/kernel/linux.yaml | awk '{print $2}')
version=$(awk '$1 == "version:" { print $2; exit }' pkg/core/kernel/linux.yaml)
echo "checksum: $checksum"
echo "version: $version"

echo "=== categorizing modules ==="
zub ls-tree -r "x86_64/pkg/core/kernel/linux/${version}/${checksum}/files" \
	| python3 pkg/core/kernel/categorize-modules.py >> pkg/core/kernel/linux.yaml

echo "=== restoring bundles ==="
./pkg/core/kernel/toggle-outputs.py restore

echo "=== refreshing metadata ==="
./nex build pkg/core/kernel/linux.yaml --refresh-metadata

echo "=== done ==="
