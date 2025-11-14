#!/bin/bash
set -eux

MANIFEST=${1:-}
if [ -z "$MANIFEST" ]; then
    echo "Usage: $0 <manifest_path>"
    echo "Example: $0 manifests/bootstrap/phase0/binutils.yaml"
    exit 1
fi

STORE=${STORE:-bootstrap_store}
OPTS=${OPTS:-}
if [ -n "${VALIDATE:-}" ]; then
    OPTS="--validate-reproducibility"
fi

# Determine if we need bootstrap based on the phase
PHASE=$(echo "$MANIFEST" | grep -o 'phase[0-9]' | cut -c6-)
B=""
if [ -n "$PHASE" ] && [ "$PHASE" -lt 2 ]; then
    B="--bootstrap"
fi

# Determine which container to use based on the phase
CONTAINER=""
if [ -n "$PHASE" ] && [ "$PHASE" -lt 2 ]; then
    # Phase 0 or 1 requires GCC
    if [ -z "${CONTAINER_WITH_GCC:-}" ]; then
        echo "Building container with GCC..."
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
    fi
    CONTAINER=$CONTAINER_WITH_GCC
else
    # Phase 2 or 3 doesn't need GCC
    if [ -z "${CONTAINER_WITHOUT_GCC:-}" ]; then
        echo "Building container without GCC..."
        podman build . -t builder-without-gcc:latest -f -<<EOF
FROM ubuntu:24.04
RUN apt update
RUN apt install -y \
    bash \
    file \
    findutils \
    gawk \
    gettext \
    grep \
    gzip \
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
        CONTAINER_WITHOUT_GCC=builder-without-gcc:latest
    fi
    CONTAINER=$CONTAINER_WITHOUT_GCC
fi

# Initialize the store if it doesn't exist
if [ ! -d "$STORE" ] || [ -z "$(ls -A "$STORE" 2>/dev/null)" ]; then
    echo "Initializing ostree store..."
    rm -rf "$STORE"
    ostree --repo="$STORE" init --mode=bare-user
fi

# Build the manifest
echo "Building manifest: $MANIFEST"
rm -rf build_rootfs

podman run --privileged --rm -it \
    -v "$(pwd):/mnt" \
    -w /mnt \
    -e STORE="$STORE" \
    "$CONTAINER" \
    bash -c "src/builder/target/debug/nex $B $OPTS $STORE $MANIFEST"

# Record success
echo "$MANIFEST" >> .done

echo "Successfully built $MANIFEST"
