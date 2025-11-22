#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/minimize_non_bootstrap_manifests_seq.sh [options] [-- <extra minimizer args>]

Run dependency-minimizer sequentially over every non-bootstrap manifest.

Options:
  -b, --builder PATH    Path to builder executable (default: src/builder/target/debug/nex)
  -m, --minimizer PATH  Path to dependency-minimizer script (default: scripts/dependency-minimizer.py)
  -h, --help            Show this help message

All remaining arguments (after an optional `--`) are forwarded unchanged to
dependency-minimizer.
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BUILDER_PATH="${REPO_ROOT}/src/builder/target/debug/nex"
MINIMIZER_PATH="${REPO_ROOT}/scripts/dependency-minimizer.py"
FORWARD_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
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

cd "${REPO_ROOT}"

mapfile -d '' MANIFESTS < <(python3 - <<'PY'
import sys
from pathlib import Path

try:
    import yaml
except ImportError as exc:
    sys.stderr.write(f"Failed to import PyYAML: {exc}\n")
    sys.exit(1)

manifest_dir = Path('pkg')
if not manifest_dir.exists():
    sys.stderr.write('pkg/ directory not found\n')
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

    namespace = pkg.get('namespace') or pkg.get('flavor')
    if isinstance(namespace, str) and namespace.startswith('bootstrap/'):
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

echo "Found ${#MANIFESTS[@]} non-bootstrap manifests. Processing sequentially."

for manifest in "${MANIFESTS[@]}"; do
    echo "\n=== Minimizing ${manifest} ==="
    if ! python3 "${MINIMIZER_PATH}" "${manifest}" --builder "${BUILDER_PATH}" "${FORWARD_ARGS[@]}"; then
        echo "Minimization failed for ${manifest}" >&2
        #exit 1
    fi
done

echo "\nAll manifests processed."
