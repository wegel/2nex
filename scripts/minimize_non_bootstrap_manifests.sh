#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/minimize_non_bootstrap_manifests.sh [options] [-- <extra minimizer args>]

Minimizes dependencies for every non-bootstrap package manifest using GNU parallel.

Options:
  -j, --jobs N          Number of manifests to process in parallel (default: 16)
  -b, --builder PATH    Path to builder executable passed to dependency-minimizer
                        (default: src/builder/target/debug/nex)
  -m, --minimizer PATH  Path to dependency-minimizer script
                        (default: scripts/dependency-minimizer.py)
  -h, --help            Show this help message

All other arguments are forwarded to dependency-minimizer, so you can pass flags
such as --overwrite, --timeout, etc.
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

JOBS=16
BUILDER_DEFAULT="${REPO_ROOT}/src/builder/target/debug/nex"
MINIMIZER_DEFAULT="${REPO_ROOT}/scripts/dependency-minimizer.py"
BUILDER_PATH="${BUILDER_DEFAULT}"
MINIMIZER_PATH="${MINIMIZER_DEFAULT}"
FORWARD_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        -j|--jobs)
            [[ $# -ge 2 ]] || { echo "Missing value for $1" >&2; exit 1; }
            JOBS="$2"
            shift 2
            ;;
        -b|--builder)
            [[ $# -ge 2 ]] || { echo "Missing value for $1" >&2; exit 1; }
            BUILDER_PATH="$2"
            shift 2
            ;;
        -m|--minimizer)
            [[ $# -ge 2 ]] || { echo "Missing value for $1" >&2; exit 1; }
            MINIMIZER_PATH="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            while [[ $# -gt 0 ]]; do
                FORWARD_ARGS+=("$1")
                shift
            done
            break
            ;;
        *)
            FORWARD_ARGS+=("$1")
            shift
            ;;
    esac
done

if ! command -v parallel >/dev/null 2>&1; then
    echo "GNU parallel not found in PATH" >&2
    exit 1
fi

cd "${REPO_ROOT}"

mapfile -d '' MANIFESTS < <(python3 - <<'PY'
import sys
from pathlib import Path

try:
    import yaml
except ImportError as exc:
    sys.stderr.write(f"Failed to import PyYAML: {exc}\n")
    sys.exit(1)

manifest_dir = Path('manifests')
if not manifest_dir.exists():
    sys.stderr.write('manifests/ directory not found\n')
    sys.exit(1)

paths = sorted(manifest_dir.rglob('*.yaml'))
for path in paths:
    try:
        with path.open('r') as handle:
            data = yaml.safe_load(handle)
    except Exception as exc:
        print(f"Skipping {path}: failed to parse ({exc})", file=sys.stderr)
        continue

    pkg = data.get('package') if isinstance(data, dict) else None
    if not isinstance(pkg, dict):
        continue

    flavor = pkg.get('flavor')
    if isinstance(flavor, str) and flavor.startswith('bootstrap/'):
        continue

    if pkg.get('bootstrap') is True:
        continue

    sys.stdout.write(str(path))
    sys.stdout.write('\0')
PY
)

if [[ ${#MANIFESTS[@]} -eq 0 ]]; then
    echo "No non-bootstrap package manifests found" >&2
    exit 0
fi

echo "Found ${#MANIFESTS[@]} non-bootstrap manifests. Running ${JOBS} job(s) in parallel."

printf '%s\0' "${MANIFESTS[@]}" | \
    parallel -0 --will-cite --keep-order --line-buffer -j "${JOBS}" -- \
        python3 "${MINIMIZER_PATH}" {} --builder "${BUILDER_PATH}" "${FORWARD_ARGS[@]}"
