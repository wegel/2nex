#!/bin/sh
set -eu

rootfs=${1:?usage: smartmontools-uapi-config.sh ROOTFS}
rootfs=$(realpath "${rootfs}")

unshare --user --map-root-user chroot "${rootfs}" /bin/sh -c '
set -eu

etc_config=/etc/smartd.conf
run_config=/run/smartmontools/smartd.conf
vendor_config=/usr/lib/smartmontools/smartd.conf
reload_log=/tmp/smartd-uapi-reload.log
reload_input=/tmp/smartd-uapi-reload.input
smartd_pid=

test ! -e "${etc_config}"
test ! -L "${etc_config}"
test ! -e "${run_config}"
test ! -L "${run_config}"
test -f "${vendor_config}"

cleanup() {
    if [ -n "${smartd_pid}" ]; then
        kill -TERM "${smartd_pid}" 2>/dev/null || :
        wait "${smartd_pid}" 2>/dev/null || :
    fi
    rm -f "${etc_config}" "${run_config}"
    rm -f "${reload_log}" "${reload_input}"
}
trap cleanup EXIT HUP INT TERM

check_path() {
    expected=$1
    shift
    output=$(LC_ALL=C "$@" 2>&1)
    case "${output}" in
        *"Opened configuration file ${expected}"*) ;;
        *)
            printf "%s\n" "${output}" >&2
            exit 1
            ;;
    esac
}

check_path "${vendor_config}" /usr/bin/smartd -d -q nodev0

mkdir -p /run/smartmontools
printf "%s\n" "DEVICESCAN -d removable -a" > "${run_config}"
check_path "${run_config}" /usr/bin/smartd -d -q nodev0

printf "%s\n" "DEVICESCAN -d removable -a" > "${etc_config}"
check_path "${etc_config}" /usr/bin/smartd -d -q nodev0
check_path "${vendor_config}" /usr/bin/smartd -d -q nodev0 -c "${vendor_config}"

: > "${etc_config}"
check_path "${etc_config}" /usr/bin/smartd -d -q nodev0

rm -f "${etc_config}" "${run_config}"
printf "%s\n" "DEVICESCAN -d removable -a" > "${run_config}"
: > "${reload_input}"
LC_ALL=C /usr/bin/smartd -d -q never \
    < "${reload_input}" > "${reload_log}" 2>&1 &
smartd_pid=$!

log_has() {
    wanted=$1
    test -f "${reload_log}" || return 1
    while IFS= read -r line; do
        case "${line}" in
            *"${wanted}"*) return 0 ;;
        esac
    done < "${reload_log}"
    return 1
}

attempt=0
while ! log_has "Opened configuration file ${run_config}"; do
    if ! kill -0 "${smartd_pid}" 2>/dev/null; then
        cat "${reload_log}" >&2
        exit 1
    fi
    attempt=$((attempt + 1))
    if [ "${attempt}" -ge 50 ]; then
        cat "${reload_log}" >&2
        exit 1
    fi
    sleep 0.1
done

printf "%s\n" "DEVICESCAN -d removable -a" > "${etc_config}"
kill -HUP "${smartd_pid}"
attempt=0
while ! log_has "Opened configuration file ${etc_config}"; do
    if ! kill -0 "${smartd_pid}" 2>/dev/null; then
        cat "${reload_log}" >&2
        exit 1
    fi
    attempt=$((attempt + 1))
    if [ "${attempt}" -ge 50 ]; then
        cat "${reload_log}" >&2
        exit 1
    fi
    sleep 0.1
done

kill -TERM "${smartd_pid}"
wait "${smartd_pid}"
smartd_pid=
'

printf 'smartd UAPI config precedence: ok\n'
