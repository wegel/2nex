#!/bin/sh
# check if builds are reproducible by comparing tree hashes
#
# usage:
#   ./check_deterministic.sh <package-ref>         # auto-find checksum refs to compare
#   ./check_deterministic.sh <ref1> <ref2>         # compare two specific refs
#
# examples:
#   ./check_deterministic.sh x86_64/pkg/core/kernel/linux/6.12.58/files
#   ./check_deterministic.sh x86_64/pkg/.../d77f9a5b.../files x86_64/pkg/.../843bbf17.../files
#
# the script compares by tree hash first (fast), then shows file diffs if needed.
# for ELF files that differ, it analyzes which sections are non-deterministic.

set -eu

LOCAL_REPO=${REPO:-.nex/repo}
ZUB=${ZUB:-zub}
MAX_ELF_ANALYSIS=${MAX_ELF_ANALYSIS:-5}

cleanup() {
    [ -n "${dir1:-}" ] && [ -d "$dir1" ] && rm -rf "$dir1"
    [ -n "${dir2:-}" ] && [ -d "$dir2" ] && rm -rf "$dir2"
    [ -n "${tmpdir:-}" ] && [ -d "$tmpdir" ] && rm -rf "$tmpdir"
    exit "${1:-0}"
}
trap 'cleanup $?' EXIT INT TERM

get_tree_hash() {
    $ZUB --repo="$LOCAL_REPO" show "$1" 2>/dev/null | grep '^tree ' | head -1 | awk '{print $2}'
}

get_commit_for_ref() {
    $ZUB --repo="$LOCAL_REPO" log "$1" 2>/dev/null | grep '^commit ' | head -1 | awk '{print $2}'
}

# analyze why an ELF file differs between builds
analyze_elf_diff() {
    bin1="$1"
    bin2="$2"
    rel_path="$3"

    # decompress if needed
    actual_bin1="$bin1"
    actual_bin2="$bin2"

    if echo "$bin1" | grep -q '\.zst$'; then
        actual_bin1="${tmpdir}/elf1_decompressed"
        actual_bin2="${tmpdir}/elf2_decompressed"
        zstd -d -f -q "$bin1" -o "$actual_bin1" 2>/dev/null || return
        zstd -d -f -q "$bin2" -o "$actual_bin2" 2>/dev/null || return
    fi

    # verify it's actually ELF
    if ! file "$actual_bin1" 2>/dev/null | grep -q "ELF"; then
        return
    fi

    echo ""
    echo "  Analyzing ELF: $rel_path"

    # check for module signature (kernel modules)
    if strings "$actual_bin1" 2>/dev/null | grep -q "Module signature appended"; then
        echo "    NOTE: module has appended signature (non-deterministic without fixed key)"
    fi

    # check for build-id
    buildid1=$(llvm-readelf -n "$actual_bin1" 2>/dev/null | grep "Build ID:" | awk '{print $3}' || true)
    buildid2=$(llvm-readelf -n "$actual_bin2" 2>/dev/null | grep "Build ID:" | awk '{print $3}' || true)
    if [ -n "$buildid1" ] && [ -n "$buildid2" ]; then
        if [ "$buildid1" = "$buildid2" ]; then
            echo "    Build IDs: identical ($buildid1)"
        else
            echo "    Build IDs: DIFFER ($buildid1 vs $buildid2)"
        fi
    fi

    # get list of sections
    sections=$(llvm-readelf -S "$actual_bin1" 2>/dev/null | grep '\[' | grep -v 'Nr\|NULL' | awk '{print $2}' | grep -v '^$' || true)

    if [ -z "$sections" ]; then
        echo "    (could not read sections)"
        return
    fi

    for section in $sections; do
        # skip sections that can't be dumped
        case "$section" in
            .bss|.tbss|.symtab|.strtab|.shstrtab|.rela*|.dynsym|.dynstr|.hash|.gnu.hash)
                continue
                ;;
        esac

        sec1="${tmpdir}/sec1_$(echo "$section" | tr '/' '_')"
        sec2="${tmpdir}/sec2_$(echo "$section" | tr '/' '_')"

        llvm-objcopy --dump-section="${section}=${sec1}" "$actual_bin1" 2>/dev/null || continue
        llvm-objcopy --dump-section="${section}=${sec2}" "$actual_bin2" 2>/dev/null || continue

        if [ ! -f "$sec1" ] || [ ! -f "$sec2" ]; then
            continue
        fi

        if ! cmp -s "$sec1" "$sec2"; then
            size1=$(stat -c%s "$sec1" 2>/dev/null || echo 0)
            size2=$(stat -c%s "$sec2" 2>/dev/null || echo 0)
            echo "    ${section}: DIFFERS (size: ${size1}/${size2})"
        fi

        rm -f "$sec1" "$sec2"
    done

    # check file sizes (appended data like signatures)
    file_size1=$(stat -c%s "$actual_bin1" 2>/dev/null || echo 0)
    file_size2=$(stat -c%s "$actual_bin2" 2>/dev/null || echo 0)

    if [ "$file_size1" != "$file_size2" ]; then
        echo "    File sizes differ: $file_size1 vs $file_size2 (likely appended data)"
    fi

    rm -f "${tmpdir}/elf1_decompressed" "${tmpdir}/elf2_decompressed" 2>/dev/null || true
}

compare_commits() {
    commit1="$1"
    commit2="$2"
    ref1="${3:-$commit1}"
    ref2="${4:-$commit2}"

    tree1=$(get_tree_hash "$commit1")
    tree2=$(get_tree_hash "$commit2")

    echo "Build 1: $ref1"
    echo "  Commit: $commit1"
    echo "  Tree:   $tree1"
    echo "Build 2: $ref2"
    echo "  Commit: $commit2"
    echo "  Tree:   $tree2"
    echo ""

    if [ "$tree1" = "$tree2" ]; then
        echo "Tree hashes match - builds are REPRODUCIBLE"
        return 0
    fi

    echo "Tree hashes DIFFER - analyzing..."
    echo ""

    # checkout and compare
    mkdir -p .nex/tmp
    dir1="$(mktemp -d -p .nex/tmp)"
    dir2="$(mktemp -d -p .nex/tmp)"
    tmpdir="$(mktemp -d -p .nex/tmp)"

    $ZUB --repo="$LOCAL_REPO" checkout "$commit1" "$dir1"
    $ZUB --repo="$LOCAL_REPO" checkout "$commit2" "$dir2"

    echo "=== Differing files ==="
    diff_count=0
    elf_count=0

    # compare files in dir1
    for file1 in $(find "$dir1" -type f | head -5000); do
        rel_path="${file1#$dir1/}"
        file2="$dir2/$rel_path"
        if [ -f "$file2" ]; then
            if ! cmp -s "$file1" "$file2"; then
                echo "DIFF: $rel_path"
                diff_count=$((diff_count + 1))

                # analyze ELF files (including compressed .ko.zst)
                if [ $elf_count -lt $MAX_ELF_ANALYSIS ]; then
                    case "$rel_path" in
                        *.ko|*.ko.zst|*.so|*.so.*|*/bin/*|*/sbin/*)
                            analyze_elf_diff "$file1" "$file2" "$rel_path"
                            elf_count=$((elf_count + 1))
                            ;;
                    esac
                fi
            fi
        else
            echo "ONLY IN BUILD1: $rel_path"
            diff_count=$((diff_count + 1))
        fi
    done

    # check for files only in dir2
    for file2 in $(find "$dir2" -type f | head -5000); do
        rel_path="${file2#$dir2/}"
        file1="$dir1/$rel_path"
        if [ ! -f "$file1" ]; then
            echo "ONLY IN BUILD2: $rel_path"
            diff_count=$((diff_count + 1))
        fi
    done

    if [ $elf_count -ge $MAX_ELF_ANALYSIS ]; then
        echo ""
        echo "(analyzed first $MAX_ELF_ANALYSIS ELF files, set MAX_ELF_ANALYSIS=N for more)"
    fi

    echo ""
    echo "=== Summary ==="
    echo "Files differ: $diff_count"
    echo "Builds are NOT reproducible"
    return 1
}

# extract package path from ref
# e.g., x86_64/pkg/core/kernel/linux/6.12.58/files -> pkg/core/kernel/linux/6.12.58
# e.g., x86_64/pkg/core/kernel/linux/6.12.58/<checksum>/files -> pkg/core/kernel/linux/6.12.58
extract_package_from_ref() {
    echo "$1" | sed -E 's|^x86_64/||; s|/[a-f0-9]{64}/files$||; s|/files$||; s|/outputs/.*||; s|/bundles/.*||'
}

# main logic
if [ $# -eq 2 ]; then
    # direct comparison mode
    ref1="$1"
    ref2="$2"

    commit1=$(get_commit_for_ref "$ref1")
    commit2=$(get_commit_for_ref "$ref2")

    if [ -z "$commit1" ]; then
        echo "Could not find commit for: $ref1"
        exit 1
    fi
    if [ -z "$commit2" ]; then
        echo "Could not find commit for: $ref2"
        exit 1
    fi

    compare_commits "$commit1" "$commit2" "$ref1" "$ref2"
    exit $?
fi

if [ $# -ne 1 ]; then
    echo "usage: $0 <package-ref>         # auto-find checksum refs to compare"
    echo "       $0 <ref1> <ref2>         # compare two specific refs"
    echo ""
    echo "examples:"
    echo "  $0 x86_64/pkg/core/kernel/linux/6.12.58/files"
    echo "  $0 d77f9a5b.../files 843bbf17.../files"
    exit 1
fi

REF="$1"
echo "Checking reproducibility for: $REF"
echo ""

# extract package path to filter checksum refs
TARGET_PACKAGE=$(extract_package_from_ref "$REF")
echo "Package: $TARGET_PACKAGE"
echo ""

# new ref structure: x86_64/{pkg}/{checksum}/files
# find all refs for this package and filter to checksum-keyed ones
echo "Scanning for checksum-keyed refs..."
base_ref=$(echo "$REF" | sed 's|/files$||')  # e.g., x86_64/pkg/core/kernel/linux/6.12.58
package_refs=$($ZUB --repo="$LOCAL_REPO" refs 2>/dev/null | grep "^${base_ref}/" | grep '/files$' || true)

# build list of refs with their tree hashes
matching_refs=""
matching_trees=""

for ref in $package_refs; do
    # extract checksum part: x86_64/pkg/.../version/<checksum>/files -> <checksum>
    checksum_part=$(echo "$ref" | sed "s|^${base_ref}/||; s|/files$||")

    # verify it looks like a checksum (64 hex chars)
    if echo "$checksum_part" | grep -qE '^[a-f0-9]{64}$'; then
        commit=$(get_commit_for_ref "$ref")
        if [ -n "$commit" ]; then
            tree=$(get_tree_hash "$commit")
            if [ -n "$tree" ]; then
                matching_refs="$matching_refs $ref:$commit:$tree"
                if ! echo "$matching_trees" | grep -q "$tree"; then
                    matching_trees="$matching_trees $tree"
                fi
            fi
        fi
    fi
done

# count matches
ref_count=$(echo "$matching_refs" | wc -w)
tree_count=$(echo "$matching_trees" | wc -w)

echo "Found $ref_count checksum-keyed refs for $TARGET_PACKAGE with $tree_count unique tree(s)"
echo ""

if [ "$ref_count" -eq 0 ]; then
    echo "No checksum-keyed refs found for this package."
    echo "Build may not have been run yet, or no checksum mismatches occurred."
    exit 0
fi

if [ "$tree_count" -lt 2 ]; then
    # check canonical ref too
    canonical_commit=$(get_commit_for_ref "$REF" 2>/dev/null || true)
    if [ -n "$canonical_commit" ]; then
        canonical_tree=$(get_tree_hash "$canonical_commit")
        echo "Canonical ref tree: $canonical_tree"

        # check if any checksum ref has different tree
        for entry in $matching_refs; do
            ref=$(echo "$entry" | cut -d: -f1)
            commit=$(echo "$entry" | cut -d: -f2)
            tree=$(echo "$entry" | cut -d: -f3)

            if [ "$tree" != "$canonical_tree" ]; then
                echo "Found different tree in checksum ref!"
                echo ""
                compare_commits "$canonical_commit" "$commit" "$REF" "$ref"
                exit $?
            fi
        done
    fi

    echo "All builds have the same tree hash - builds are REPRODUCIBLE"
    exit 0
fi

# found multiple trees - compare two with different hashes
echo "Found $tree_count different tree hashes - comparing..."
echo ""

first_ref=""
first_commit=""
first_tree=""
second_ref=""
second_commit=""

for entry in $matching_refs; do
    ref=$(echo "$entry" | cut -d: -f1)
    commit=$(echo "$entry" | cut -d: -f2)
    tree=$(echo "$entry" | cut -d: -f3)

    if [ -z "$first_ref" ]; then
        first_ref="$ref"
        first_commit="$commit"
        first_tree="$tree"
    elif [ "$tree" != "$first_tree" ]; then
        second_ref="$ref"
        second_commit="$commit"
        break
    fi
done

if [ -n "$second_ref" ]; then
    compare_commits "$first_commit" "$second_commit" "$first_ref" "$second_ref"
    exit $?
fi

echo "Could not find two refs with different trees to compare"
exit 1
