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

remove_file_if_hash() {
    file=$1
    shift
    [ -f "$file" ] && [ ! -L "$file" ] || return 0

    actual=$(sha256sum "$file")
    actual=${actual%% *}
    for expected in "$@"; do
        if [ "$actual" = "$expected" ]; then
            rm -f "$file"
            return 0
        fi
    done
}

remove_legacy_link() {
    host_root=$1
    relative=$2
    file=$host_root/$relative
    [ -L "$file" ] || return 0
    target=$(readlink "$file")

    case "$relative:$target" in
        issue:/nex/pkg/*/usr/share/factory/etc/issue|\
        locale.conf:/nex/pkg/*/usr/share/factory/etc/locale.conf|\
        nsswitch.conf:/nex/pkg/*/usr/share/factory/etc/nsswitch.conf|\
        vconsole.conf:/nex/pkg/*/usr/share/factory/etc/vconsole.conf|\
        pam.d/other:/nex/pkg/*/usr/share/factory/etc/pam.d/other|\
        pam.d/system-auth:/nex/pkg/*/usr/share/factory/etc/pam.d/system-auth|\
        pam.d/systemd-run0:/usr/lib/pam.d/systemd-run0|\
        pam.d/systemd-user:/usr/lib/pam.d/systemd-user)
            rm -f "$file"
            ;;
    esac
}

migrate_legacy_factory_defaults() {
    host_root=$1

    for relative in \
        issue locale.conf nsswitch.conf vconsole.conf \
        pam.d/other pam.d/system-auth pam.d/systemd-run0 pam.d/systemd-user; do
        remove_legacy_link "$host_root" "$relative"
    done

    remove_file_if_hash "$host_root/os-release" \
        feac65c09271b0a33e1765c43338357cc9a0cb3a47a65f6f04d4a0b717ff83c9 \
        51038b95881dc8529bac04462cd4a847f7dcfc7cadc0ee7bb2599d17190c3914
    remove_file_if_hash "$host_root/fstab" \
        e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    remove_file_if_hash "$host_root/sysctl.conf" \
        e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    remove_file_if_hash "$host_root/inputrc" \
        5bf298a7e30e6938934d62f6ac42a581c833862659d8d193514b849697ae4935 \
        c19cc9c26208cc4de2039504b0078710fb3a3c126f9583a28d9bb6b46a593e61
    remove_file_if_hash "$host_root/security/namespace.conf" \
        e636821359fd3efa38627dd204522618eb95741c2b2eeec3bdf1742f6678323b
    remove_file_if_hash "$host_root/environment" \
        2b1077668be5e331ab4c060092dbc9e412852934e2a0754460c8fdc1f515c51e
    remove_file_if_hash "$host_root/systemd/network/80-dhcp.network" \
        9874e14209f363517c1cfa8d9c2f01af95328a25864dd63aaac6f1b980c70932 \
        e919a489d4139cbbbad0b6c9a001a623501be91da9c2014b9057c0b32a0775d1
    remove_file_if_hash "$host_root/systemd/system/nex-init-manifests.service" \
        9f911bb1f8f845182a6395cea42601165233595ba4836b1520e253631004a260
    remove_file_if_hash "$host_root/systemd/logind.conf" \
        e2c7e906fd58c32e9c56a57fddb7ffd2697dc97eb0e64a0eb784107325f26d90
    remove_file_if_hash "$host_root/systemd/system/systemd-logind.service.d/dbus-ordering.conf" \
        a54d7ee07f9345cddda7c18de809e88a6c7ea4b54a8cb92a3e2dde06c481d514
    remove_file_if_hash "$host_root/tmpfiles.d/xdg-runtime.conf" \
        945333615a86f8bc4a6d2b3481b461592a6788d5638b63bdcb747f1058c16d33
    remove_file_if_hash "$host_root/tmpfiles.d/utmp.conf" \
        f0028114e5705d2ed32c6372cd7bc5f158f7f61a4a39909a85456cd1921d1ac3
    remove_file_if_hash "$host_root/udev/rules.d/70-seat.rules" \
        a62ef7b78cb361f08cd89ea05c20808ae8d36654e706d4df9ceef76ca2f7b173
    remove_file_if_hash "$host_root/NetworkManager/conf.d/wifi-backend.conf" \
        654d07fff7855a7c51cf5c55478df12491e64dda5c87fbb14f1d44faf93419eb
    remove_file_if_hash "$host_root/iwd/main.conf" \
        abcb3cae686dacc4c121d4dfae70d795cc0bd9165e265c0b68db684f4fa4202f
    remove_file_if_hash "$host_root/systemd/journald.conf.d/00-persistent.conf" \
        dc49e8cda48d483b8038bab2940dd7957a52868b75f93e1e69693ddc1eb62ccf
    remove_file_if_hash "$host_root/containers/containers.conf" \
        f8d446795c5165649583615768c0c4c6a1a6883bdcb85e604b1b9f71be7511c4
    remove_file_if_hash "$host_root/containers/registries.conf" \
        ec865a599049720379ebd5b1bcc5a7ad31a341fe624f1e42ac6f98a5b5d0575e
    remove_file_if_hash "$host_root/containers/policy.json" \
        b827f6c8ec4679a05957e05e35edee4aec24dc6b88a8055e245f29d8e2dcd131
    remove_file_if_hash "$host_root/modprobe.d/nvidia.conf" \
        4444c88de4b2646df9ec31b9b84b0e6bb46f77376edebabbc59ae2359f2812cb
    remove_file_if_hash "$host_root/modules-load.d/nvidia.conf" \
        c38c9b4c776514e711a1d3a5f624da1aa564ddd35555f560f18ac4fc7dcbb41b
}

factory_etc=$1
host_etc=$2

if [ ! -d "${factory_etc}" ]; then
    exit 0
fi

mkdir -p "${host_etc}"
migrate_legacy_factory_defaults "${host_etc}"
copy_missing_tree "${factory_etc}" "${host_etc}"
