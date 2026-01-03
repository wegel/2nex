# Kernel Configuration Philosophy

## Goal

Boot on 98% of modern x86_64 hardware with minimal builtins, maximum module coverage, and zero performance compromise.

## Core Principles

### 1. Minimal Builtins, Maximum Modules

Only boot-critical drivers are built-in (=y). Everything else is a module (=m).

**Built-in (=y):**
- Storage controllers: NVMe, SATA/AHCI, USB storage (and a small handful of ubiquitous controllers)
- Filesystems: ext4, btrfs, xfs, vfat (EFI)
- Console: serial, framebuffer, simpledrm
- Core infrastructure: PCI, ACPI, USB host controllers, input

**Modules (=m):**
- Network drivers (ethernet, wifi, bluetooth)
- Graphics drivers (i915, amdgpu, nouveau)
- Sound (ALSA, HDA)
- Webcams, sensors, exotic hardware
- Everything else

This keeps the kernel image small while supporting virtually all hardware via modules loaded post-boot.

### 2. Allmod from Allnoconfig

We don't use defconfig or allmodconfig directly. Instead we generate a controlled
config via `allmod.py`:

1. Start from allnoconfig (everything disabled)
2. Apply pre-fragments (gates + builtins)
3. Enable all tristates as modules
4. Enable only **safe gate bools** (bools that do not select tristates) whose
   dependencies are already satisfied
5. Re-apply builtins.frag (restore explicit =y and =n)
6. Apply disable.frag (unwanted subsystems)
7. Apply overrides.frag (final tweaks)

This avoids `select`-driven cascades that would force tristates to `=y`,
keeping the builtins minimal while still unlocking large module families.

### 3. No Debug/Test Overhead

Production kernel only. No:
- Debug symbols in modules
- Kernel debugging infrastructure
- Test drivers or sanitizers
- Verbose logging

These belong in a separate debug kernel variant, not the default boot kernel.

### 4. Kconfig Gates

Bool options that unlock entire menuconfig subsections must be =y (either
explicitly in builtins/gates fragments or via safe-gate auto-enable) to allow
their children to become modules. Tristate gates are NOT in builtins - they
become =m naturally via allmod.

Example bool gates:
- `CONFIG_ETHERNET=y` - unlocks all ethernet drivers
- `CONFIG_WLAN=y` - unlocks all wifi drivers
- `CONFIG_SND_PCI=y` - unlocks PCI sound cards

We distinguish between:
- **Safe gates**: bools that do not `select` tristates. These can be auto-enabled.
- **Dangerous gates**: bools that `select` tristates (directly or transitively).
  These are not auto-enabled to avoid forcing builtins.

### 5. Dependency Awareness

Some options have tricky dependencies like `depends on X || X=n`. If X=m, this evaluates to m, limiting dependent options to =m even if we want =y.

We handle this by explicitly setting the dependency to =y or =n in builtins.frag:
- `CONFIG_DAX=y` - needed for BLK_DEV_DM=y
- `CONFIG_RPMB=y` - needed for MMC_BLOCK=y
- `CONFIG_AGP=n` - allows DRM=y without AGP overhead

## Integration with Zub Boot System

This kernel config is designed for the zub-native boot architecture:

1. **Minimal initramfs**: Only what's needed to reach the deployment
2. **Boot-critical drivers are builtins**: Avoids relying on early module loads
3. **Module loading post-switch_root**: Real init loads remaining modules
4. **Edge case support**: Bootloader can generate module cpio for exotic hardware

The 95% of hardware with common storage controllers boots directly. The 5% with exotic controllers use bootloader-assisted module loading.

## File Structure

```
pkg/core/kernel/
├── allmod.py              # config generator script
├── linux.yaml             # package manifest
└── fragments/
    ├── builtins.frag      # boot-critical =y, bool gates
    ├── gates*.frag        # additional bool gates
    ├── disable.frag       # unwanted subsystems =n
    └── overrides.frag     # final tweaks, force =y for specific features
```

## Metrics

Target configuration:
- Builtins kept as low as possible (boot-only)
- Module count tracked per build
- Kernel image size tracked per build

All boot-critical options verified as =y via sanity check in build script.
