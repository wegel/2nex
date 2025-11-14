#!/bin/sh
set -eux

LOCAL_REPO=${REPO:-bootstrap_store}
REMOTE_REPO=${REMOTE_REPO:-}
REF="${1}"
#x86_64/gcc/13.2.0/base/bundles/dev

# Cleanup temporary directories at the end
cleanup() {
    echo "Cleaning up..."
    # Remove temp directories if they exist
    [ -n "${dir1:-}" ] && [ -d "$dir1" ] && rm -rf "$dir1"
    [ -n "${dir2:-}" ] && [ -d "$dir2" ] && rm -rf "$dir2"
    [ -n "${temp_repo:-}" ] && [ -d "$temp_repo" ] && rm -rf "$temp_repo"
    exit "${1:-0}"
}
#trap 'cleanup $?' EXIT INT TERM

if [ -z "$REMOTE_REPO" ]; then
    # Compare two local commits of the same package
    echo "Comparing two local commits for ${REF}..."
    
    # Base directories to compare
    dir1="$(mktemp -d)"
    dir2="$(mktemp -d)"
    
    REF1="$(ostree --repo=$LOCAL_REPO log "${REF}" | head -1 | awk '{print $2}')"
    REF2="$(ostree --repo=$LOCAL_REPO log "${REF}" | head -2 | tail -1 | awk '{print $2}')"
    
    unshare --map-root-user ostree --repo=${LOCAL_REPO} checkout --union ${REF1} ${dir1}
    unshare --map-root-user ostree --repo=${LOCAL_REPO} checkout --union ${REF2} ${dir2}
else
    # Compare local commit with remote commit
    echo "Comparing local vs remote ${REF}..."
    
    # Base directories to compare
    dir1="$(mktemp -d)"
    dir2="$(mktemp -d)"
    
    # Get latest commit from local repo
    REF1="$(ostree --repo=$LOCAL_REPO log "${REF}" | head -1 | awk '{print $2}')"
    
    # Create a temporary local copy of the remote repo
    temp_repo="$(mktemp -d)"
    
    # Use rsync to copy remote repo directly
    echo "Copying remote repository content..."
    rsync -az --xattrs --fake-super ${REMOTE_REPO} ${temp_repo}
    
    # Get the remote ref
    REF2="$(ostree --repo=${temp_repo} log "${REF}" | head -1 | awk '{print $2}')"
    
    echo "Local commit: ${REF1}"
    echo "Remote commit: ${REF2}"
    
    # Check out both commits
    unshare --map-root-user ostree --repo=${LOCAL_REPO} checkout --union ${REF1} ${dir1}
    unshare --map-root-user ostree --repo=${temp_repo} checkout --union ${REF2} ${dir2}
fi

# Function to compare files
compare_files() {
    local file1="$1"
    local file2="$2"
    if ! diff -q "$file1" "$file2" > /dev/null; then
        echo "Differing file: ${file1} ${file2}"
    else
        echo "$file1 same"
    fi
}

# Export function and base directories for access in find command
export -f compare_files
export dir1
export dir2

# Find and compare all files in dir1 to corresponding files in dir2
find "$dir1" -type f -exec sh -c 'compare_files "{}" "${dir2}${1#$dir1}"' _ {} \;
