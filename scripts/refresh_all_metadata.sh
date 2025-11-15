#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILDER="${BUILDER:-${ROOT_DIR}/src/builder/target/debug/nex}"
REPO="${REPO:-${ROOT_DIR}/bootstrap_store}"

if [[ ! -x "${BUILDER}" ]]; then
  echo "Builder not found at ${BUILDER}. Build it (e.g., cargo build) or set BUILDER=/path/to/nex." >&2
  exit 1
fi

find "${ROOT_DIR}/manifests" -type f -name '*.yaml' | sort | while read -r manifest; do
  if grep -qi '^kind:\s*system' "${manifest}"; then
    echo "Skipping system manifest ${manifest}"
    continue
  fi
  echo "Refreshing metadata for ${manifest}"
  "${BUILDER}" --refresh-ostree-metadata "${REPO}" "${manifest}"
done
