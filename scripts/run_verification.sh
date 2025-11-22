#!/bin/bash
set -e

cd /repo

# Build the project
cargo build --manifest-path src/builder/Cargo.toml

# Initialize OSTree repository
ostree --repo=bootstrap_store init --mode=bare-user

# Verify Project Status
mkdir -p /my-build && cp -a * /my-build/ && cd /my-build
for PHASE in 0 1 2 3; do
  echo "Phase: $PHASE"
  for M in $(ls pkg/bootstrap/phase${PHASE}/*.yaml 2>/dev/null | sort -V); do
    echo "Building: ${M}"
    B=""
    [ $PHASE -lt 2 ] && B="--bootstrap"
    if ! src/builder/target/debug/nex $B bootstrap_store $M > /tmp/build_log 2>&1; then
      cat /tmp/build_log
      echo "Failed ${M}"
      exit 1
    fi
    rm -rf build_rootfs
  done
done

echo "All verifications completed successfully!"
