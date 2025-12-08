#!/bin/bash
# delete all non-bootstrap refs from the zub store

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
REPO="${REPO:-.nex/repo}"
ZUB="zub"

$ZUB --repo="$REPO" refs | awk '{print $2}' | grep -v "bootstrap" | while read ref; do
  echo "Deleting: $ref"
  $ZUB --repo="$REPO" delete-ref "$ref"
done

echo "Done. Running garbage collection..."
$ZUB --repo="$REPO" gc
