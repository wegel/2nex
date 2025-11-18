#!/usr/bin/env bash
set -euo pipefail

# Reorganize manifests from flat hyphenated to hierarchical structure
# Example: sys-libs/glibc.yaml → sys/libs/glibc.yaml

cd "$(dirname "$0")/.."

echo "=== Manifest Reorganization Script ==="
echo "This will reorganize manifests to hierarchical structure"
echo ""

# Mapping of old → new structure
declare -A CATEGORY_MAP=(
    ["sys-apps"]="sys/apps"
    ["sys-devel"]="sys/devel"
    ["sys-fs"]="sys/fs"
    ["sys-kernel"]="sys/kernel"
    ["sys-libs"]="sys/libs"
    ["app-arch"]="app/arch"
    ["app-containers"]="app/containers"
    ["app-misc"]="app/misc"
    ["app-shells"]="app/shells"
    ["app-text"]="app/text"
    ["dev-build"]="dev/build"
    ["dev-lang"]="dev/lang"
    ["dev-libs"]="dev/libs"
    ["dev-perl"]="dev/perl"
    ["dev-python"]="dev/python"
    ["dev-util"]="dev/util"
    ["dev-vcs"]="dev/vcs"
    ["net-misc"]="net/misc"
)

# Create new directory structure
echo "Creating new directory structure..."
for new_cat in "${CATEGORY_MAP[@]}"; do
    mkdir -p "manifests/$new_cat"
done

# Track all changes for reporting
declare -a MOVED_FILES=()
declare -a UPDATED_MANIFESTS=()

# Move manifests and update flavor field
echo ""
echo "Moving and updating manifests..."
for old_cat in "${!CATEGORY_MAP[@]}"; do
    new_cat="${CATEGORY_MAP[$old_cat]}"

    if [[ ! -d "manifests/$old_cat" ]]; then
        continue
    fi

    echo "  $old_cat/ → $new_cat/"

    for manifest in manifests/"$old_cat"/*.yaml; do
        if [[ ! -f "$manifest" ]]; then
            continue
        fi

        filename=$(basename "$manifest")
        new_path="manifests/$new_cat/$filename"

        # Update flavor field in manifest (use | as delimiter since new_cat contains /)
        sed -i "s|flavor: \"$old_cat\"|flavor: \"$new_cat\"|" "$manifest"

        # Git mv to new location
        git mv "$manifest" "$new_path"

        MOVED_FILES+=("$old_cat/$filename → $new_cat/$filename")
        UPDATED_MANIFESTS+=("$new_path")
    done

    # Remove old empty directory
    if [[ -d "manifests/$old_cat" ]]; then
        rmdir "manifests/$old_cat" 2>/dev/null || true
    fi
done

# Update all cross-references in manifests
echo ""
echo "Updating cross-references in all manifests..."
for old_cat in "${!CATEGORY_MAP[@]}"; do
    new_cat="${CATEGORY_MAP[$old_cat]}"

    # Escape slashes for sed
    old_cat_escaped="${old_cat//\//\\/}"
    new_cat_escaped="${new_cat//\//\\/}"

    # Update commit references: x86_64/SLUG/VERSION/OLD-CAT/ → x86_64/SLUG/VERSION/NEW-CAT/
    # Match pattern: commit: x86_64/.../old-cat/...
    find manifests -name "*.yaml" -type f -exec \
        sed -i "s|commit: \(x86_64/[^/]*/[^/]*/\)$old_cat_escaped/|commit: \1$new_cat_escaped/|g" {} \;

    # Update bare dependency references (list items without commit: prefix)
    # Match pattern: - x86_64/.../old-cat/...
    find manifests -name "*.yaml" -type f -exec \
        sed -i "s|^\([[:space:]]*\)- \(x86_64/[^/]*/[^/]*/\)$old_cat_escaped/|\1- \2$new_cat_escaped/|g" {} \;

    # Update OSTree branch references in requires: and suggests: sections
    # Match pattern: - x86_64/.../old-cat/outputs/... or - x86_64/.../old-cat/bundles/...
    find manifests -name "*.yaml" -type f -exec \
        sed -i "s|\(x86_64/[^/]*/[^/]*/\)$old_cat_escaped/\(outputs\|bundles\)|\1$new_cat_escaped/\2|g" {} \;
done

# Generate summary report
echo ""
echo "=== Migration Summary ==="
echo "Moved ${#MOVED_FILES[@]} manifest files"
echo ""

echo "Category Mapping:"
for old_cat in "${!CATEGORY_MAP[@]}"; do
    new_cat="${CATEGORY_MAP[$old_cat]}"
    count=$(find "manifests/$new_cat" -name "*.yaml" 2>/dev/null | wc -l)
    printf "  %-20s → %-20s (%d files)\n" "$old_cat/" "$new_cat/" "$count"
done

echo ""
echo "New structure:"
tree -d -L 3 manifests/ || find manifests -type d | sort

echo ""
echo "=== Verification ==="
echo "Checking for any remaining old-style references..."
old_refs=0
for old_cat in "${!CATEGORY_MAP[@]}"; do
    if grep -r "flavor: \"$old_cat\"" manifests/ --include="*.yaml" 2>/dev/null; then
        echo "ERROR: Found unreplaced flavor field: $old_cat"
        old_refs=$((old_refs + 1))
    fi

    # Check for old commit paths (be careful not to match commented examples)
    if grep -r "commit:.*/$old_cat/" manifests/ --include="*.yaml" | grep -v "^#" 2>/dev/null; then
        echo "WARNING: Found old commit path reference: $old_cat"
        old_refs=$((old_refs + 1))
    fi
done

if [[ $old_refs -eq 0 ]]; then
    echo "✓ No old-style references found"
else
    echo "⚠ Found $old_refs old-style references - review needed"
fi

echo ""
echo "=== Next Steps ==="
echo "1. Review changes: git status"
echo "2. Test manifest parsing: ./src/builder/target/debug/nex test-repo manifests/sys/libs/glibc.yaml --dry-run"
echo "3. Commit changes: git commit -m 'manifests: reorganize to hierarchical structure'"
echo ""
echo "Done!"
