#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: nex-populate-etc FACTORY_ETC HOST_ETC" >&2
    exit 2
fi

copy_missing_tree() (
    source_root=$1
    target_root=$2

    for source in \
        "${source_root}"/* \
        "${source_root}"/.[!.]* \
        "${source_root}"/..?*; do
        if [ ! -e "${source}" ] && [ ! -L "${source}" ]; then
            continue
        fi

        name=${source##*/}
        target=${target_root}/${name}
        if [ -d "${source}" ] && [ ! -L "${source}" ]; then
            if [ ! -e "${target}" ] && [ ! -L "${target}" ]; then
                cp -a "${source}" "${target_root}/"
            elif [ -d "${target}" ] && [ ! -L "${target}" ]; then
                copy_missing_tree "${source}" "${target}"
            fi
        elif [ ! -e "${target}" ] && [ ! -L "${target}" ]; then
            cp -a "${source}" "${target}"
        fi
    done
)

factory_etc=$1
host_etc=$2

if [ ! -d "${factory_etc}" ]; then
    exit 0
fi

mkdir -p "${host_etc}"
copy_missing_tree "${factory_etc}" "${host_etc}"
