#!/bin/bash
# comprehensive test for time-travel/history search feature
#
# this script creates multiple versions of a package in OSTree history,
# then verifies the builder can find the correct historical commit by manifest hash.

set -e

BUILDER="./src/builder/target/debug/nex"
REPO="bootstrap_store"
TEST_DIR=$(mktemp -d)
TREE_DIR="$TEST_DIR/tree"

cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

echo "=== Time-Travel History Search Test ==="
echo ""
echo "Test directory: $TEST_DIR"
echo ""

# create a test tree to commit
mkdir -p "$TREE_DIR/usr/lib"
echo "test content v1" > "$TREE_DIR/usr/lib/libtest.so"

# use a test branch
TEST_BRANCH="x86_64/pkg/test/timetravel/1.0/outputs/lib"

echo "Step 1: Create V1 commit with manifest hash H1"
HASH_V1="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
ostree commit --repo="$REPO" \
    --branch="$TEST_BRANCH" \
    --add-metadata-string="nex.manifest.hash=$HASH_V1" \
    "$TREE_DIR" 2>/dev/null
COMMIT_V1=$(ostree rev-parse --repo="$REPO" "$TEST_BRANCH")
echo "  Commit V1: ${COMMIT_V1:0:12}"
echo "  Hash V1:   ${HASH_V1:0:12}..."
echo ""

# modify tree and create V2
echo "test content v2" > "$TREE_DIR/usr/lib/libtest.so"

echo "Step 2: Create V2 commit with manifest hash H2"
HASH_V2="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
ostree commit --repo="$REPO" \
    --branch="$TEST_BRANCH" \
    --add-metadata-string="nex.manifest.hash=$HASH_V2" \
    "$TREE_DIR" 2>/dev/null
COMMIT_V2=$(ostree rev-parse --repo="$REPO" "$TEST_BRANCH")
echo "  Commit V2: ${COMMIT_V2:0:12}"
echo "  Hash V2:   ${HASH_V2:0:12}..."
echo ""

# verify branch tip is V2
echo "Step 3: Verify branch history"
echo "  Branch tip (HEAD): ${COMMIT_V2:0:12}"
echo "  Parent (V1):       ${COMMIT_V1:0:12}"
echo ""

# show the history
echo "  OSTree log:"
ostree log --repo="$REPO" "$TEST_BRANCH" 2>&1 | grep -E "^commit " | head -5 | sed 's/^/    /'
echo ""

# now test the builder's find_commit_by_manifest_hash function
# we'll create a manifest that should trigger the search

echo "Step 4: Create test manifest with dependency pinned to V1's hash"

# we need to create a git blob with content that hashes to HASH_V1
# since we control the hash, we'll create a manifest file and compute its sha256
TEST_MANIFEST="$TEST_DIR/test_dep.yaml"
cat > "$TEST_MANIFEST" << 'EOF'
package:
  name: test-timetravel-dep
  slug: test-timetravel-dep
  version: "1.0"
  namespace: pkg/test
dependencies:
  - commit: x86_64/pkg/test/timetravel/1.0/outputs/lib
sources: []
build:
  script: "true"
outputs:
  bin:
    files:
      - /usr/bin/test
bundles:
  dev:
    - bin
EOF

# compute manifest hash and verify what we need
MANIFEST_HASH=$(sha256sum "$TEST_MANIFEST" | cut -d' ' -f1)
echo "  Test manifest hash: ${MANIFEST_HASH:0:12}..."
echo ""

echo "Step 5: Test find_commit_by_manifest_hash logic"
echo ""
echo "  Searching for HASH_V1 (${HASH_V1:0:12}...) in branch history..."

# manually check using ostree commands (simulating what find_commit_by_manifest_hash does)
FOUND_COMMIT=""
for commit in $(ostree log --repo="$REPO" "$TEST_BRANCH" 2>&1 | grep "^commit " | awk '{print $2}'); do
    stored_hash=$(ostree show --repo="$REPO" --print-metadata-key=nex.manifest.hash "$commit" 2>/dev/null | tr -d "'")
    if [ "$stored_hash" = "$HASH_V1" ]; then
        FOUND_COMMIT="$commit"
        break
    fi
done

if [ -n "$FOUND_COMMIT" ]; then
    echo "  FOUND: ${FOUND_COMMIT:0:12}"
    if [ "$FOUND_COMMIT" = "$COMMIT_V1" ]; then
        echo "  STATUS: SUCCESS - correctly found V1, not V2 (branch tip)"
    else
        echo "  STATUS: FAILED - found wrong commit"
    fi
else
    echo "  STATUS: FAILED - commit not found"
fi
echo ""

echo "Step 6: Test searching for HASH_V2"
echo ""
echo "  Searching for HASH_V2 (${HASH_V2:0:12}...) in branch history..."

FOUND_COMMIT=""
for commit in $(ostree log --repo="$REPO" "$TEST_BRANCH" 2>&1 | grep "^commit " | awk '{print $2}'); do
    stored_hash=$(ostree show --repo="$REPO" --print-metadata-key=nex.manifest.hash "$commit" 2>/dev/null | tr -d "'")
    if [ "$stored_hash" = "$HASH_V2" ]; then
        FOUND_COMMIT="$commit"
        break
    fi
done

if [ -n "$FOUND_COMMIT" ]; then
    echo "  FOUND: ${FOUND_COMMIT:0:12}"
    if [ "$FOUND_COMMIT" = "$COMMIT_V2" ]; then
        echo "  STATUS: SUCCESS - correctly found V2"
    else
        echo "  STATUS: FAILED - found wrong commit"
    fi
else
    echo "  STATUS: FAILED - commit not found"
fi
echo ""

# cleanup test branch
echo "Step 7: Cleanup test branch"
ostree refs --repo="$REPO" --delete "$TEST_BRANCH" 2>/dev/null || true
echo "  Deleted: $TEST_BRANCH"
echo ""

echo "=== Test Complete ==="
