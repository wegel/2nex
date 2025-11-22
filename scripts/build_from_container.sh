#!/bin/bash
set -eu

MANIFEST=${1:-}
if [ -z "$MANIFEST" ]; then
    echo "Usage: $0 <manifest_path>"
    echo "Example: $0 pkg/bootstrap/phase0/binutils.yaml"
    exit 1
fi

STORE=${STORE:-bootstrap_store}
OPTS=${OPTS:-}
if [ -n "${VALIDATE:-}" ]; then
    OPTS="--validate-reproducibility"
fi

        podman build . -t builder-with-gcc:latest -f -<<EOF
FROM ubuntu:24.04
RUN apt update
RUN apt install -y \
    bash \
    binutils \
    bison \
    coreutils \
    diffutils \
    file \
    findutils \
    gawk \
    gcc \
    gettext \
    g++ \
    grep \
    gzip \
    linux-headers-virtual-hwe-24.04 \
    m4 \
    make \
    ostree \
    patch \
    perl \
    python3 \
    sed \
    tar \
    texinfo \
    uidmap \
    util-linux \
    xz-utils
EOF
CONTAINER_WITH_GCC=builder-with-gcc:latest
CONTAINER=$CONTAINER_WITH_GCC

# Initialize the store if it doesn't exist
if [ ! -d "$STORE" ] || [ -z "$(ls -A "$STORE" 2>/dev/null)" ]; then
    echo "Initializing ostree store at $STORE ..."
    rm -rf "$STORE"
    ostree --repo="$STORE" init --mode=bare-user
fi

# Build the manifest
echo "Building manifest: $MANIFEST"
rm -rf build_rootfs* || true

podman run --privileged --rm -it \
    -v "$(pwd):/mnt" \
    -w /mnt \
    -e STORE="$STORE" \
    "$CONTAINER" \
    bash -c "src/builder/target/debug/nex $OPTS $STORE $MANIFEST"

echo "Successfully built $MANIFEST"
