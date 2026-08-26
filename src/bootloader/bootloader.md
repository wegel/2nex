# Zub-Native Boot System Design

## Overview
A fast-boot Linux distribution built entirely around zub, with deep integration from bootloader through system initialization. Eliminates traditional generated initramfs and Boot Loader Specification intermediaries in favor of direct zub-aware components.

## Architecture Components

### 1. Custom EFI Bootloader (rust-uefi)
**Responsibilities:**
- Load ext4 EFI driver to read Linux filesystems
- Parse zub deployment structure directly from `/zub/deploy/<osname>/deploy/`
- Present UI for deployment selection (current, rollback, etc.)
- Perform deployment verification/health checks
- Load kernel from selected deployment
- Generate initramfs cpio on-demand (only for edge-case hardware)
- Boot directly to versioned kernel

**Key innovation:** Understands zub natively, no BLS entries needed.

### 2. Kernel Configuration
**Built-in drivers (~2-3MB overhead):**
- Common storage controllers: NVMe, SATA (ahci), virtio-blk, SD/MMC, USB storage
- ext4 filesystem
- Optional: dm-crypt for encryption support

**Covers 95% of x86-64 hardware** - embedded, desktop, laptop, server, minipc.

**Built-in initramfs:** Constant init script (busybox + shell, ~1-2MB) compiled into kernel via CONFIG_INITRAMFS_SOURCE.

### 3. Constant Init (Built into Kernel)
**Minimal shell script responsibilities:**
```sh
# mount zub filesystem (ext4 driver is built-in)
mount /dev/storage /sysroot -t ext4

# find deployment (bootloader already chose it)
DEPLOYMENT=/sysroot/zub/deploy/myos/deploy/<checksum>.0

# set up zub overlay structure
# - /usr from deployment (read-only)
# - /var shared across deployments
# - /etc merged from deployment + modifications

# switch to real root
exec switch_root /newroot /sbin/init
```

**Key properties:**
- Auditable (plain shell script)
- Truly constant (same for all boots)
- No hardware detection needed
- All complexity deferred to zub-hosted init

### 4. Edge Case Hardware (5%)
**For exotic storage controllers not built into kernel:**

User creates `/etc/boot-modules.conf` in their zub deployment listing required modules:
```
/usr/lib/modules/6.x.x/exotic-raid.ko
/usr/lib/modules/6.x.x/old-storage.ko
```

**Bootloader behavior:**
- Reads this config from zub deployment
- Loads specified `.ko` files from deployment
- Generates small cpio archive in memory containing just those modules
- Passes as additional initramfs to kernel (overlays built-in initramfs)
- Constant init script runs `modprobe` for them before mounting

**Kernel automatically merges multiple initramfs cpios** - built-in constant init + runtime-generated modules work together seamlessly.

### 5. Post-Switch_root: Normal Init System
After switch_root, the zub deployment's real init system takes over:
- systemd, runit, s6, or whatever is in the deployment
- Loads additional modules (network, graphics, etc.) via standard mechanisms
- `/etc/modules-load.d/*.conf` for systemd
- Early boot scripts for others
- Mounts remaining filesystems, starts services
- Normal Linux userspace

**All flexibility lives here** - versioned in zub, changes with deployments.

## Boot Flow Summary

```
EFI Firmware
    ↓
Custom Bootloader (rust-uefi)
    - Loads ext4 driver
    - Reads /zub/deploy/ structure
    - Presents deployment menu
    - [Optional] Generates module cpio for edge cases
    - Loads kernel + (optional additional initramfs)
    ↓
Kernel (with built-in constant init)
    - Unpacks built-in initramfs
    - [Optional] Overlays additional module cpio
    - Executes /init
    ↓
Constant Init (busybox script)
    - Mounts /zub filesystem (ext4 built-in)
    - [Optional] Loads extra modules if present
    - Sets up zub deployment overlay
    - switch_root to deployment
    ↓
Real Init System (from zub deployment)
    - Loads remaining modules
    - Normal system startup
    - Everything versioned in zub
```

## Key Benefits

**Fast boot:**
- No initramfs generation at install/update time
- No cpio unpacking overhead (built-in is already in memory)
- Direct boot to versioned kernel
- Minimal bootloader overhead

**Clean versioning:**
- Kernel, init infrastructure, userspace all in zub
- Atomic updates and rollbacks
- Deployment selection in bootloader = complete system state

**Simplicity:**
- No BLS generation step
- No dracut/mkinitcpio complexity
- Auditable constant init (plain shell script)
- Standard Linux after switch_root

**Flexibility where needed:**
- 95% of hardware works out of box
- 5% edge cases handled cleanly via boot-time module loading
- All dynamic complexity in zub-versioned userspace
- Standard module loading mechanisms post-boot

## Tradeoffs

**Chosen:**
- Small kernel size increase (2-3MB) for built-in drivers
- Exotic hardware requires explicit module configuration
- Custom bootloader maintenance burden

**Rejected:**
- Universal hardware support (too much kernel bloat)
- Generated-per-deployment initramfs (too slow, defeats purpose)
- kexec approach (too slow)
- Standard bootloaders + BLS (unnecessary layer)

## Novel Aspects

No mainstream Linux distribution uses this architecture. The combination of:
- Zub-native bootloader
- Constant built-in init
- On-demand module loading via bootloader
- No generated artifacts

...is genuinely new. Most similar to embedded/appliance systems, but designed for general computing with zub-native semantics throughout.
