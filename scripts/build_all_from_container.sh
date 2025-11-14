#!/bin/sh
set -eux

if [ -z "${CONTAINER_WITH_GCC:-}" ]; then
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

STORE=${STORE:-bootstrap_store}

rm -rf ${STORE}
ostree --repo=${STORE} init --mode=bare-user
podman run --privileged --rm -it -v $(pwd):/mnt -w /mnt -e STORE=${STORE} -e PHASES="0 1" ${CONTAINER_WITH_GCC} ./scripts/build.sh

if [ -z "${CONTAINER_WITHOUT_GCC:-}" ]; then
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

podman run --privileged --rm -it -v $(pwd):/mnt -w /mnt -e STORE=${STORE} -e PHASES="2 3" ${CONTAINER_WITHOUT_GCC} ./scripts/build.sh
