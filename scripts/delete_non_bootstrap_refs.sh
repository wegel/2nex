#!/bin/bash
# delete all non-bootstrap refs from the ostree store

REPO="${REPO:-bootstrap_store}"

ostree refs --repo="$REPO" | grep -v "bootstrap" | while read ref; do
  echo "Deleting: $ref"
  ostree refs --repo="$REPO" --delete "$ref"
done

echo "Done. Running prune..."
ostree prune --repo="$REPO" --refs-only
