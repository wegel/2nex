#!/usr/bin/env python3
r"""
categorize kernel modules into logical outputs based on their path structure.

usage:
    # from zub store (the parser skips directory entries):
    zub ls-tree -r <ref> | ./categorize-modules.py

    # a newline-separated path list also works:
    find <root> -type f -printf '%P\n' | ./categorize-modules.py

outputs yaml-formatted outputs section to stdout.
"""

import sys
import re
from collections import defaultdict

# category rules: (pattern, category_name)
# order matters - first match wins
CATEGORY_RULES = [
    # boot files (not modules, handled separately)

    # GPU drivers
    (r'/drivers/gpu/drm/amd/', 'drv-gpu-amd'),
    (r'/drivers/gpu/drm/i915/', 'drv-gpu-intel'),
    (r'/drivers/gpu/drm/xe/', 'drv-gpu-intel-xe'),
    (r'/drivers/gpu/drm/nouveau/', 'drv-gpu-nouveau'),
    (r'/drivers/gpu/drm/virtio/', 'drv-gpu-virtio'),
    (r'/drivers/gpu/drm/radeon/', 'drv-gpu-radeon'),
    (r'/drivers/gpu/drm/vmwgfx/', 'drv-gpu-vmware'),
    (r'/drivers/gpu/drm/qxl/', 'drv-gpu-qxl'),
    (r'/drivers/gpu/drm/bochs/', 'drv-gpu-bochs'),
    (r'/drivers/gpu/drm/ast/', 'drv-gpu-ast'),
    (r'/drivers/gpu/drm/mgag200/', 'drv-gpu-matrox'),
    (r'/drivers/gpu/drm/', 'drv-gpu-core'),

    # wireless drivers by vendor
    (r'/drivers/net/wireless/intel/', 'drv-wifi-intel'),
    (r'/drivers/net/wireless/ath/', 'drv-wifi-atheros'),
    (r'/drivers/net/wireless/realtek/', 'drv-wifi-realtek'),
    (r'/drivers/net/wireless/broadcom/', 'drv-wifi-broadcom'),
    (r'/drivers/net/wireless/mediatek/', 'drv-wifi-mediatek'),
    (r'/drivers/net/wireless/ralink/', 'drv-wifi-ralink'),
    (r'/drivers/net/wireless/marvell/', 'drv-wifi-marvell'),
    (r'/drivers/net/wireless/ti/', 'drv-wifi-ti'),
    (r'/drivers/net/wireless/zydas/', 'drv-wifi-zydas'),
    (r'/drivers/net/wireless/admtek/', 'drv-wifi-admtek'),
    (r'/drivers/net/wireless/intersil/', 'drv-wifi-intersil'),
    (r'/drivers/net/wireless/silabs/', 'drv-wifi-silabs'),
    (r'/drivers/net/wireless/purelifi/', 'drv-wifi-purelifi'),
    (r'/drivers/net/wireless/microchip/', 'drv-wifi-microchip'),
    (r'/drivers/net/wireless/quantenna/', 'drv-wifi-quantenna'),
    (r'/drivers/net/wireless/rsi/', 'drv-wifi-rsi'),
    (r'/drivers/net/wireless/', 'drv-wifi-misc'),
    (r'/net/wireless/', 'drv-wifi-core'),
    (r'/net/mac80211/', 'drv-wifi-core'),

    # ethernet drivers by vendor
    (r'/drivers/net/ethernet/intel/', 'drv-eth-intel'),
    (r'/drivers/net/ethernet/realtek/', 'drv-eth-realtek'),
    (r'/drivers/net/ethernet/broadcom/', 'drv-eth-broadcom'),
    (r'/drivers/net/ethernet/marvell/', 'drv-eth-marvell'),
    (r'/drivers/net/ethernet/mellanox/', 'drv-eth-mellanox'),
    (r'/drivers/net/ethernet/aquantia/', 'drv-eth-aquantia'),
    (r'/drivers/net/ethernet/nvidia/', 'drv-eth-nvidia'),
    (r'/drivers/net/ethernet/amd/', 'drv-eth-amd'),
    (r'/drivers/net/ethernet/atheros/', 'drv-eth-atheros'),
    (r'/drivers/net/ethernet/chelsio/', 'drv-eth-chelsio'),
    (r'/drivers/net/ethernet/qlogic/', 'drv-eth-qlogic'),
    (r'/drivers/net/ethernet/sfc/', 'drv-eth-solarflare'),
    (r'/drivers/net/ethernet/', 'drv-eth-misc'),

    # network core/virtual
    (r'/drivers/net/virtio', 'drv-net-virtio'),
    (r'/drivers/net/veth', 'drv-net-virt'),
    (r'/drivers/net/vxlan', 'drv-net-virt'),
    (r'/drivers/net/macvlan', 'drv-net-virt'),
    (r'/drivers/net/tun', 'drv-net-virt'),
    (r'/drivers/net/tap', 'drv-net-virt'),
    (r'/drivers/net/bonding', 'drv-net-virt'),
    (r'/drivers/net/team', 'drv-net-virt'),
    (r'/drivers/net/bridge', 'drv-net-virt'),
    (r'/drivers/net/dummy', 'drv-net-virt'),
    (r'/drivers/net/', 'drv-net-misc'),

    # netfilter/firewall
    (r'/net/netfilter/', 'net-netfilter'),
    (r'/net/ipv4/netfilter/', 'net-netfilter'),
    (r'/net/ipv6/netfilter/', 'net-netfilter'),
    (r'/net/bridge/netfilter/', 'net-netfilter'),

    # network protocols
    (r'/net/bluetooth/', 'net-bluetooth'),
    (r'/net/bridge/', 'net-bridge'),
    (r'/net/802/', 'net-802'),
    (r'/net/llc/', 'net-llc'),
    (r'/net/ipv4/', 'net-ipv4'),
    (r'/net/ipv6/', 'net-ipv6'),
    (r'/net/sctp/', 'net-sctp'),
    (r'/net/dccp/', 'net-dccp'),
    (r'/net/rds/', 'net-rds'),
    (r'/net/can/', 'net-can'),
    (r'/net/nfc/', 'net-nfc'),
    (r'/net/qrtr/', 'net-qrtr'),
    (r'/net/', 'net-misc'),

    # bluetooth drivers
    (r'/drivers/bluetooth/', 'drv-bluetooth'),

    # sound drivers
    (r'/sound/(?:pci/)?hda/', 'drv-sound-hda'),
    (r'/sound/pci/', 'drv-sound-pci'),
    (r'/sound/usb/', 'drv-sound-usb'),
    (r'/sound/soc/', 'drv-sound-soc'),
    (r'/sound/core/', 'drv-sound-core'),
    (r'/sound/hda/', 'drv-sound-core'),
    (r'/sound/', 'drv-sound-misc'),

    # USB drivers
    (r'/drivers/usb/serial/', 'drv-usb-serial'),
    (r'/drivers/usb/storage/', 'drv-usb-storage'),
    (r'/drivers/usb/class/', 'drv-usb-class'),
    (r'/drivers/usb/misc/', 'drv-usb-misc'),
    (r'/drivers/usb/', 'drv-usb-core'),

    # HID/input drivers
    (r'/drivers/hid/i2c-hid/', 'drv-hid-i2c'),
    (r'/drivers/hid/usbhid/', 'drv-hid-usb'),
    (r'/drivers/hid/', 'drv-hid'),
    (r'/drivers/input/mouse/', 'drv-input-mouse'),
    (r'/drivers/input/keyboard/', 'drv-input-keyboard'),
    (r'/drivers/input/joystick/', 'drv-input-joystick'),
    (r'/drivers/input/touchscreen/', 'drv-input-touchscreen'),
    (r'/drivers/input/tablet/', 'drv-input-tablet'),
    (r'/drivers/input/', 'drv-input-misc'),

    # I2C drivers
    (r'/drivers/i2c/busses/', 'drv-i2c-busses'),
    (r'/drivers/i2c/', 'drv-i2c-core'),

    # storage/block drivers
    (r'/drivers/nvme/', 'drv-nvme'),
    (r'/drivers/ata/', 'drv-ata'),
    (r'/drivers/scsi/', 'drv-scsi'),
    (r'/drivers/block/', 'drv-block'),
    (r'/drivers/md/', 'drv-md-raid'),
    (r'/drivers/mmc/', 'drv-mmc'),
    (r'/drivers/mtd/', 'drv-mtd'),

    # virtio
    (r'/drivers/virtio/', 'drv-virtio'),

    # platform drivers
    (r'/drivers/platform/x86/', 'drv-platform-x86'),
    (r'/drivers/platform/', 'drv-platform-misc'),

    # ACPI
    (r'/drivers/acpi/', 'drv-acpi'),

    # power management
    (r'/drivers/cpufreq/', 'drv-cpufreq'),
    (r'/drivers/cpuidle/', 'drv-cpuidle'),
    (r'/drivers/thermal/', 'drv-thermal'),
    (r'/drivers/powercap/', 'drv-powercap'),

    # sensors/hwmon
    (r'/drivers/hwmon/', 'drv-hwmon'),
    (r'/drivers/iio/', 'drv-iio-sensors'),

    # LEDs
    (r'/drivers/leds/', 'drv-leds'),

    # watchdog
    (r'/drivers/watchdog/', 'drv-watchdog'),

    # RTC
    (r'/drivers/rtc/', 'drv-rtc'),

    # GPIO
    (r'/drivers/gpio/', 'drv-gpio'),
    (r'/drivers/pinctrl/', 'drv-pinctrl'),

    # PWM
    (r'/drivers/pwm/', 'drv-pwm'),

    # regulator
    (r'/drivers/regulator/', 'drv-regulator'),

    # MFD (multi-function devices)
    (r'/drivers/mfd/', 'drv-mfd'),

    # PCI
    (r'/drivers/pci/', 'drv-pci'),

    # thunderbolt/USB4
    (r'/drivers/thunderbolt/', 'drv-thunderbolt'),

    # firewire
    (r'/drivers/firewire/', 'drv-firewire'),

    # media/video4linux
    (r'/drivers/media/usb/', 'drv-media-usb'),
    (r'/drivers/media/pci/', 'drv-media-pci'),
    (r'/drivers/media/platform/', 'drv-media-platform'),
    (r'/drivers/media/v4l2-core/', 'drv-media-core'),
    (r'/drivers/media/', 'drv-media-misc'),

    # infiniband/RDMA
    (r'/drivers/infiniband/', 'drv-infiniband'),

    # crypto hardware
    (r'/drivers/crypto/', 'drv-crypto-hw'),

    # DMA
    (r'/drivers/dma/', 'drv-dma'),

    # EDAC (error detection)
    (r'/drivers/edac/', 'drv-edac'),

    # memory controllers
    (r'/drivers/memory/', 'drv-memory'),

    # IOMMU
    (r'/drivers/iommu/', 'drv-iommu'),

    # vfio
    (r'/drivers/vfio/', 'drv-vfio'),

    # vhost
    (r'/drivers/vhost/', 'drv-vhost'),

    # bus drivers
    (r'/drivers/bus/', 'drv-bus'),

    # SoC drivers
    (r'/drivers/soc/', 'drv-soc'),

    # filesystems
    (r'/fs/nfs/', 'fs-nfs'),
    (r'/fs/cifs/', 'fs-cifs'),
    (r'/fs/smb/', 'fs-smb'),
    (r'/fs/btrfs/', 'fs-btrfs'),
    (r'/fs/xfs/', 'fs-xfs'),
    (r'/fs/ext4/', 'fs-ext4'),
    (r'/fs/f2fs/', 'fs-f2fs'),
    (r'/fs/fat/', 'fs-fat'),
    (r'/fs/ntfs3/', 'fs-ntfs'),
    (r'/fs/exfat/', 'fs-exfat'),
    (r'/fs/fuse/', 'fs-fuse'),
    (r'/fs/overlayfs/', 'fs-overlay'),
    (r'/fs/squashfs/', 'fs-squashfs'),
    (r'/fs/9p/', 'fs-9p'),
    (r'/fs/isofs/', 'fs-iso'),
    (r'/fs/udf/', 'fs-udf'),
    (r'/fs/hfsplus/', 'fs-hfs'),
    (r'/fs/jfs/', 'fs-jfs'),
    (r'/fs/reiserfs/', 'fs-reiserfs'),
    (r'/fs/erofs/', 'fs-erofs'),
    (r'/fs/efivarfs/', 'fs-efi'),
    (r'/fs/nls/', 'fs-nls'),
    (r'/fs/', 'fs-misc'),

    # crypto
    (r'/crypto/', 'crypto'),
    (r'/lib/crypto/', 'crypto'),

    # lib
    (r'/lib/', 'lib'),

    # security
    (r'/security/', 'security'),

    # catch-all for remaining drivers
    (r'/drivers/', 'drv-misc'),
]

def categorize_module(path: str) -> str:
    """determine category for a module based on its path."""
    for pattern, category in CATEGORY_RULES:
        if re.search(pattern, path):
            return category
    return 'misc'

def main():
    categories = defaultdict(list)
    release = None

    for line in sys.stdin:
        path = line.strip()
        if not path:
            continue

        fields = path.split(maxsplit=3)
        if (
            len(fields) == 4
            and re.fullmatch(r'[0-7]{6}', fields[0])
            and fields[1] in {'directory', 'regular', 'symlink'}
        ):
            if fields[1] == 'directory':
                continue
            path = fields[3]

        # normalize path to start with /
        if not path.startswith('/'):
            path = '/' + path

        release_match = (
            re.match(r'^/usr/lib/modules/([^/]+)/', path)
            or re.match(r'^/usr/src/linux-([^/]+)/', path)
            or re.match(
                r'^/boot/(?:System\.map|config|vmlinuz)-(.+)$', path
            )
        )
        if release_match:
            path_release = release_match.group(1)
            if release is None:
                release = path_release
            elif release != path_release:
                raise SystemExit(
                    f"mixed kernel releases: {release} and {path_release}"
                )

        if re.match(r'^/boot/(?:System\.map|config|vmlinuz)-', path):
            categories['boot'].append(path)
            continue

        module_root_match = re.match(r'^/usr/lib/modules/([^/]+)/([^/]+)$', path)
        if module_root_match:
            if module_root_match.group(2) in {'build', 'source'}:
                categories['module-sdk'].append(path)
            else:
                categories['modules-meta'].append(path)
            continue

        if re.match(r'^/usr/src/linux-[^/]+/', path):
            categories['module-sdk'].append(path)
            continue

        if path.startswith('/usr/lib/kernel/size/'):
            categories['lib'].append(path)
            continue

        # handle compressed modules (.ko.zst)
        original_path = path
        if path.endswith('.ko.zst'):
            path = path[:-4]  # strip .zst for categorization

        # skip non-module files
        if not path.endswith('.ko'):
            continue

        category = categorize_module(path)
        categories[category].append(original_path)

    if release is None:
        raise SystemExit("no module path supplied a kernel release")

    # sort categories and files within each
    print("outputs:")

    # output each category
    for category in sorted(categories.keys()):
        files = sorted(set(categories[category]))
        print(f"  {category}:")
        print("    files:")
        for f in files:
            print(f"    - path: {f}")
        print()

    # print summary to stderr
    total = sum(
        1
        for files in categories.values()
        for path in set(files)
        if path.endswith(('.ko', '.ko.zst'))
    )
    print(f"# total: {total} modules in {len(categories)} categories", file=sys.stderr)
    for category in sorted(categories.keys()):
        entries = len(set(categories[category]))
        print(f"#   {category}: {entries} entries", file=sys.stderr)

if __name__ == '__main__':
    main()
