#!/bin/sh
set -eu

LOCAL_REPO=${REPO:-.nex/repo}
ZUB=${ZUB:-zub}
REF="${1}"
# example: x86_64/pkg/cli/editors/neovim/0.11.0/outputs/bin

# cleanup temporary directories at the end
cleanup() {
    echo "Cleaning up..."
    [ -n "${dir1:-}" ] && [ -d "$dir1" ] && rm -rf "$dir1"
    [ -n "${dir2:-}" ] && [ -d "$dir2" ] && rm -rf "$dir2"
    exit "${1:-0}"
}
trap 'cleanup $?' EXIT INT TERM

echo "Comparing two local commits for ${REF}..."

# base directories to compare (use .nex/tmp to avoid cross-device link issues)
mkdir -p .nex/tmp
dir1="$(mktemp -d -p .nex/tmp)"
dir2="$(mktemp -d -p .nex/tmp)"

# get the last two commits from zub log
# zub log format: "commit <hash>" on each commit line
REF1="$($ZUB --repo=$LOCAL_REPO log "${REF}" | grep '^commit ' | head -1 | awk '{print $2}')"
REF2="$($ZUB --repo=$LOCAL_REPO log "${REF}" | grep '^commit ' | head -2 | tail -1 | awk '{print $2}')"

echo "Commit 1: ${REF1}"
echo "Commit 2: ${REF2}"

if [ "$REF1" = "$REF2" ]; then
    echo "Only one commit found, nothing to compare"
    exit 0
fi

$ZUB --repo=${LOCAL_REPO} checkout ${REF1} ${dir1}
$ZUB --repo=${LOCAL_REPO} checkout ${REF2} ${dir2}

# find differing files
echo ""
echo "=== Differing files ==="
diff_found=0
for file1 in $(find "$dir1" -type f); do
    file2="${dir2}${file1#$dir1}"
    if [ -f "$file2" ]; then
        if ! diff -q "$file1" "$file2" > /dev/null 2>&1; then
            echo "DIFF: ${file1#$dir1/}"
            diff_found=1
        fi
    else
        echo "MISSING in build2: ${file1#$dir1/}"
        diff_found=1
    fi
done

# check for files only in dir2
for file2 in $(find "$dir2" -type f); do
    file1="${dir1}${file2#$dir2}"
    if [ ! -f "$file1" ]; then
        echo "MISSING in build1: ${file2#$dir2/}"
        diff_found=1
    fi
done

if [ "$diff_found" -eq 0 ]; then
    echo "All files match!"
else
    echo ""
    echo "Builds are NOT reproducible"
    exit 1
fi
