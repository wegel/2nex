#!/bin/sh

FORCE_TARGETS=${FORCE_TARGETS:-}

for PHASE in bootstrap/phase0 bootstrap/phase1 bootstrap/phase2 bootstrap/phase3 base embedded; do
  echo "Phase: $PHASE"
  MANIFEST_DIR="manifests/${PHASE}"

  for M in $(ls "${MANIFEST_DIR}"/*.yaml | sort -V); do
    # extract package info to check if already built
    SLUG=$(grep "slug:" "$M" | head -1 | awk '{print $2}' | sed "s/[\"']//g")
    VERSION=$(grep "version:" "$M" | head -1 | awk '{print $2}' | sed "s/[\"']//g")
    FLAVOR=$(grep "flavor:" "$M" | head -1 | awk '{print $2}' | sed "s/[\"']//g")
    CHECKSUM=$(grep -E "^[[:space:]]+checksum:" "$M" | head -1 | awk '{print $2}' | sed "s/[\"']//g")

    # get the first bundle name from the manifest
    BUNDLE_NAME=$(awk '
      /^bundles:/ {found=1; next}
      found && NF {
        gsub(":","",$1);
        print $1;
        exit
      }' "$M")
    if [ -z "$BUNDLE_NAME" ]; then
      echo "Failed to determine bundle name for ${M}"
      exit 1
    fi

    # check if the bundle already exists in ostree
    BUNDLE_REF="x86_64/${SLUG}/${VERSION}/${FLAVOR}/bundles/${BUNDLE_NAME}"

    FORCE_REBUILD=0
    if [ -n "$FORCE" ] && printf '%s\n' "$FORCE" | tr ',' '\n' | grep -Fxq "$M"; then
      FORCE_REBUILD=1
      echo "Forcing rebuild of ${M}"
    fi

    if [ $FORCE_REBUILD -eq 0 ] && ostree --repo=bootstrap_store rev-parse "$BUNDLE_REF" >/dev/null 2>&1; then
      if [ -n "$CHECKSUM" ]; then
        if ! EXISTING_CHECKSUM=$(ostree --repo=bootstrap_store show --print-metadata-key=nex.build.checksum "$BUNDLE_REF" 2>/dev/null | tr -d "'" | tr -d '[:space:]'); then
          EXISTING_CHECKSUM=""
        fi
        if [ "$EXISTING_CHECKSUM" = "$CHECKSUM" ]; then
          echo "Skipping ${M} (already built with matching checksum: ${BUNDLE_REF})"
          continue
        fi
        echo "Checksum mismatch for ${M} (manifest: ${CHECKSUM}, built: ${EXISTING_CHECKSUM:-none}), rebuilding"
      else
        echo "Skipping ${M} (already built: ${BUNDLE_REF})"
        continue
      fi
    fi

    echo "Building: ${M}"
    B=""
    PHASE_NAME=$(basename "$PHASE")
    case "$PHASE_NAME" in
      0|phase0|1|phase1)
        B="--bootstrap"
        ;;
    esac
    if ! src/builder/target/debug/nex $B bootstrap_store $M > /tmp/build_log 2>&1; then
      cat /tmp/build_log
      echo "Failed ${M}"
      exit 1
    fi
    rm -rf build_rootfs
  done
done
