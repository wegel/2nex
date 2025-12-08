#!/bin/sh
set -eu

# compare ELF sections between two builds from zub repo
# usage: diff_elf_sections.sh <ref> [binary_path]
# example: diff_elf_sections.sh x86_64/pkg/cli/editors/neovim/0.11.0/outputs/bin /usr/bin/nvim

LOCAL_REPO=${REPO:-.nex/repo}
ZUB=${ZUB:-zub}
REF="${1}"
BINARY="${2:-}"

cleanup() {
    [ -n "${dir1:-}" ] && [ -d "$dir1" ] && rm -rf "$dir1"
    [ -n "${dir2:-}" ] && [ -d "$dir2" ] && rm -rf "$dir2"
    [ -n "${tmpdir:-}" ] && [ -d "$tmpdir" ] && rm -rf "$tmpdir"
    exit "${1:-0}"
}
trap 'cleanup $?' EXIT INT TERM

mkdir -p .nex/tmp
dir1="$(mktemp -d -p .nex/tmp)"
dir2="$(mktemp -d -p .nex/tmp)"
tmpdir="$(mktemp -d -p .nex/tmp)"

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

# find the binary to compare
if [ -z "$BINARY" ]; then
    # find the first ELF binary
    BINARY=$(find "$dir1" -type f -exec file {} \; | grep 'ELF' | head -1 | cut -d: -f1)
    BINARY="${BINARY#$dir1}"
fi

bin1="${dir1}/${BINARY}"
bin2="${dir2}/${BINARY}"

echo ""
echo "Comparing: ${BINARY}"

if cmp -s "$bin1" "$bin2"; then
    echo "BINARIES ARE IDENTICAL!"
    exit 0
fi

echo "Binaries differ. Analyzing sections..."
echo ""

# get list of sections
sections=$(llvm-readelf -S "$bin1" | grep '\[' | grep -v 'Nr\|NULL' | awk '{print $2}' | grep -v '^$')

for section in $sections; do
    # skip sections that can't be dumped
    case "$section" in
        .bss|.tbss|.symtab|.strtab|.shstrtab|.rela*|.dynsym|.dynstr|.hash|.gnu.hash)
            continue
            ;;
    esac

    sec1="${tmpdir}/sec1_${section}"
    sec2="${tmpdir}/sec2_${section}"

    llvm-objcopy --dump-section="${section}=${sec1}" "$bin1" 2>/dev/null || continue
    llvm-objcopy --dump-section="${section}=${sec2}" "$bin2" 2>/dev/null || continue

    if [ ! -f "$sec1" ] || [ ! -f "$sec2" ]; then
        continue
    fi

    if cmp -s "$sec1" "$sec2"; then
        echo "  ${section}: IDENTICAL"
    else
        size1=$(stat -c%s "$sec1" 2>/dev/null || echo 0)
        size2=$(stat -c%s "$sec2" 2>/dev/null || echo 0)
        first_diff=$(cmp -l "$sec1" "$sec2" 2>/dev/null | head -1 | awk '{print $1}')
        echo "  ${section}: DIFFERS (size: ${size1}/${size2}, first diff at byte ${first_diff})"

        # show context for differing sections
        if [ -n "$first_diff" ]; then
            offset=$((first_diff - 32))
            [ $offset -lt 0 ] && offset=0
            echo "    Build 1 context (offset $offset):"
            xxd -s $offset -l 64 "$sec1" | sed 's/^/      /'
            echo "    Build 2 context (offset $offset):"
            xxd -s $offset -l 64 "$sec2" | sed 's/^/      /'
            echo ""
        fi
    fi
done

echo ""
echo "Summary: Binaries are NOT identical"
