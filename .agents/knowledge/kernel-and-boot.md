# Kernel And Boot

## Kernel Package Writes

Kernel package manifests should keep build writes under `${OUT_DIR}`. A strict
kernel build failed when the manifest tried to create `/usr/lib/modules` in the
build root. Installing modules under `${OUT_DIR}/usr/lib/modules` and running
`depmod -b ${OUT_DIR}/usr 6.12.58` passed the strict build.

## Curated Kernel Outputs

Do not let generic output generation erase curated kernel output names such as
`boot`, `modules-meta`, `drv-eth-intel`, and `drv-gpu-amd`. The kernel build
needs those named outputs for bundles and assemblies.

## Out-Of-Tree Module Policy

Nex should support third-party kernel modules through a kernel module SDK, not
through target-machine DKMS builds. The kernel package should expose the
headers, config, `Module.symvers`, scripts, and build tree files needed to run
`make M=<module-source> modules` against the exact shipped kernel. Each
out-of-tree module package should build against that SDK and install modules
under `/usr/lib/modules/<kernel-release>/extra` or another explicit external
module directory.

Evidence: the human chose this policy before ExecPlan 003. The first required
users are Nvidia 580, current Nvidia, v4l2loopback, and possibly Looking Glass
host-side support.

Nex may build several Nvidia driver branches in the repository, but a booted
root must activate exactly one branch. Nvidia's Linux stack uses global module
names and matching userspace libraries, so a root must not activate 580 and a
current branch at the same time.

The 580 branch is required for Pascal/Turing mixed machines such as GTX 1060
plus RTX 2070. A current branch is required for newer Blackwell-class machines
such as RTX 5090.

`pkg/libs/graphics/nvidia-580.yaml` builds Nvidia 580.159.04 proprietary
modules and matching userspace from the upstream no-compat32 runfile. It uses
the runfile's `kernel/` source directory, excludes `nvidia-peermem`, and
installs `nvidia.ko`, `nvidia-drm.ko`, `nvidia-modeset.ko`, and
`nvidia-uvm.ko` under `/usr/lib/modules/6.12.58/extra`.

Evidence: the strict two-pass package build passed with checksum
`a5040fec9c894cfe40b06df252cfd8817efd67abf1e4e16c0d586aca9ed2508a`.
The runtime bundle smoke showed module version `580.159.04`, vermagic
`6.12.58 SMP modversions`, license `NVIDIA`, and `nvidia-smi --help` reported
v580.159.04.

`pkg/libs/graphics/nvidia-current.yaml` builds Nvidia 595.84 open modules and
matching userspace from the upstream no-compat32 runfile. It uses the
runfile's `kernel-open/` source directory, excludes `nvidia-peermem`, and
installs the same four module names under `/usr/lib/modules/6.12.58/extra`.

Evidence: the strict two-pass package build passed with checksum
`9673fd029e8b84c456664558e46813a07d3e75e14fb631cf5eda06118c87e233`.
The runtime bundle smoke showed module version `595.84`, vermagic
`6.12.58 SMP modversions`, license `Dual MIT/GPL`, and `nvidia-smi --help`
reported v595.84.

`pkg/core/kernel/linux.yaml` now exposes a `module-sdk` output and bundle for
Linux 6.12.58. The bundle branch is
`x86_64/pkg/core/kernel/linux/6.12.58/bundles/module-sdk`. It contains the
external module build tree under `/usr/src/linux-6.12.58` plus
`/usr/lib/modules/6.12.58/build` and `source` symlinks to that tree.

Build external modules with the SDK by checking out the bundle and running
Linux's usual external-module command:

```bash
zub -r .nex/repo checkout x86_64/pkg/core/kernel/linux/6.12.58/bundles/module-sdk <sdk-dir> -f
make -C <sdk-dir>/usr/src/linux-6.12.58 M=<module-source-dir> modules
```

The SDK checkout must include `Module.symvers`, `include/config/auto.conf`,
`include/generated/autoconf.h`, `scripts/mod/modpost`, and
`tools/objtool/objtool`. A smoke module built this way produced a `.ko` whose
`modinfo -F vermagic` output was `6.12.58 SMP modversions`.

The SDK must also include `.config` and must not set
`CONFIG_TRIM_UNUSED_KSYMS`. Nvidia's module build reads `.config`, and the
trimmed-symbol option removes exported symbols that the proprietary branch
needs.

Evidence: before the SDK copied `.config`, Nvidia printed a missing `.config`
diagnostic. Before the kernel disabled `CONFIG_TRIM_UNUSED_KSYMS`, Nvidia 580
failed at `MODPOST` on missing exports such as `cpufreq_get`, `iterate_fd`,
and `drm_edid_override_connector_update`. After the fix, both Nvidia branches
built modules against the SDK.

`pkg/core/kernel/v4l2loopback.yaml` packages v4l2loopback 0.12.7 for Linux
6.12.58. The package builds against the kernel `module-sdk` bundle, installs
`/usr/lib/modules/6.12.58/extra/v4l2loopback.ko`, and installs the upstream
`v4l2loopback-ctl` Bash tool. The package applies a build-tree edit from
`strlcpy(` to `strscpy(` because Linux 6.12 removed `strlcpy`.

The checked v4l2loopback tags from 0.13.0 through 0.15.4 reference
`v4l2_fill_pixfmt_mp` and `v4l2_format_info`. Linux 6.12.58's SDK
`Module.symvers` does not export those symbols, so those tags fail at
`modpost` against the current SDK.

Do not ship package-local `depmod` metadata from out-of-tree module packages.
Running `depmod` inside a package that contains only an external module creates
partial `/usr/lib/modules/<release>/modules.*` files. Those files can
overwrite the kernel package's complete module metadata when an assembly layers
packages into one root. A later assembly step should regenerate depmod
metadata after the kernel and all external module packages are present.

Evidence: v4l2loopback 0.12.7 passed the strict two-pass package build with
checksum `19aaf3aaf95317c8e82bbac91ba4cc38b34877a583cd2eee43fae90e8c24ab82`.
The runtime bundle smoke checkout showed `modinfo -F vermagic` output
`6.12.58 SMP modversions`, and `v4l2loopback-ctl --help` printed the expected
commands.

`pkg/core/kernel/kvmfr.yaml` packages Looking Glass B7 host shared-memory
support for Linux 6.12.58. The package builds upstream `module/` against the
kernel `module-sdk` bundle and installs
`/usr/lib/modules/6.12.58/extra/kvmfr.ko`.

Evidence: the strict two-pass package build passed with checksum
`cedddc8669134ee32ad8ec57385f88b4d7fe323a6913a984770e33c1fe3f016f`.
The runtime bundle smoke showed `modinfo -F version` output `0.0.12`,
`modinfo -F vermagic` output `6.12.58 SMP modversions`, and
`modinfo -F license` output `GPL v2`.

Nvidia runtime bundles must include the `misc` output. The Nvidia manifests
store Vulkan ICD JSON, Vulkan implicit layer JSON, EGL external platform
files, desktop files, and application profiles there. If the runtime bundle
omits `misc`, a desktop assembly can expose Mesa Vulkan metadata while missing
Nvidia's vendor files.

Evidence: the first `desktop-vwl-nvidia-580` assembly smoke showed
`/usr/share/glvnd/egl_vendor.d/10_nvidia.json` but lacked
`/usr/share/vulkan/icd.d/nvidia_icd.json`. After the Nvidia manifests added
`misc` to the runtime bundle, rebuilt 580 and current roots exposed both
`nvidia_icd.json` and `nvidia_layers.json`.

`asm/desktop-vwl-nvidia-580.yaml` and
`asm/desktop-vwl-nvidia-current.yaml` are the desktop variants
that activate one Nvidia branch at a time. Both variants also include
v4l2loopback, `kvmfr`, and Looking Glass. The 580 root uses Nvidia 580.159.04;
the current root uses Nvidia 595.84.

Evidence: checked-out roots reported Nvidia module versions `580.159.04` and
`595.84`, v4l2loopback vermagic `6.12.58 SMP modversions`, `kvmfr` version
`0.0.12`, and Looking Glass help output `Looking Glass (B7)`.

## CPU Early Microcode

Linux expects x86 early microcode as an uncompressed `newc` cpio archive before
the normal initramfs. AMD uses `kernel/x86/microcode/AuthenticAMD.bin`. Intel
uses `kernel/x86/microcode/GenuineIntel.bin`.

`pkg/core/kernel/amd-ucode-initramfs.yaml` builds that archive from the
existing `linux-firmware` `amd-ucode` output and installs it as
`/boot/amd-ucode.cpio`. The package build needs explicit build-time
dependencies for `linux-headers` and `binutils` because the manifest compiles
`gen_init_cpio.c` with GCC.

`pkg/core/kernel/intel-ucode-initramfs.yaml` builds Intel early microcode from
Intel's `Intel-Linux-Processor-Microcode-Data-Files` release archive. Use the
normal `intel-ucode/` directory, not `intel-ucode-with-caveats/`, unless a
later policy chooses the caveat-marked files explicitly.

The Nex bootloader treats `/boot/amd-ucode.cpio` and
`/boot/intel-ucode.cpio` in the selected deployment as declarations that early
microcode should load. It prepends any declared vendor cpios before boot
module initrd data. The direct-initramfs QEMU helper follows the same rule and
uses `cmp -n` with offsets to prove each microcode cpio sits before the base
initramfs.

Evidence: ExecPlan 004 built `amd-ucode-initramfs` reproducibly with checksum
`163ca6364c7c02414a3ec62ada8152a7f6eee8b9e1ffe3d1c7459d18aa763277`.
The package smoke found `kernel/x86/microcode/AuthenticAMD.bin` inside
`/boot/amd-ucode.cpio`. The final direct QEMU command with
`TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1` logged the AMD early
microcode cpio path, passed the prefix check, booted, and printed
`ASSERT-BOOT-PASS`.

Evidence: the Intel early microcode package built reproducibly with checksum
`dac12b4b2fb450a7cd3e520fc1de9241d09b74f0205382976138d44c200e8acd`.
The package smoke found `kernel/x86/microcode/GenuineIntel.bin` inside
`/boot/intel-ucode.cpio`. A later direct QEMU command logged both AMD and Intel
early cpio paths, passed offset checks for both, booted, and printed
`ASSERT-BOOT-PASS`.

## Bootloader Checks

Host-side bootloader integration tests can skip the UEFI binary target:

```bash
cargo test --manifest-path src/bootloader/Cargo.toml --target x86_64-unknown-linux-gnu --no-default-features --test cpio_test
```

Build the UEFI binary from `src/bootloader` after installing the Rust target
`x86_64-unknown-uefi`. The local Cargo config passes `--cfg aes_force_soft` for
that target to avoid the `aes` crate x86 backend codegen failure seen with
Rust 1.93 and LLVM 21.

## Direct Initramfs QEMU Test

`./scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --headless
--extra-nex-var --timeout 180` tests the early boot mount contract without
building installer media. It avoids host tools such as `parted`, `mkfs.vfat`,
`mcopy`, and `mmd`.

The initramfs must:

- bind-mount the selected deployment root onto itself before binding child
  paths under it
- remount the deployment `/sysroot` bind read-only
- derive the writable var partition from the root device before trusting a
  decoy `nex-var` label

For final assembled desktop validation, the smaller direct command without the
extra decoy disk is enough to prove the built root enters the Nex boot path:

```bash
scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout 180
```

## Virtio GPU For Graphical QEMU

Graphical QEMU smokes that start wlroots need virtio GPU KMS available before
the compositor starts. Build `CONFIG_DRM_VIRTIO_GPU=y` and
`CONFIG_DRM_VIRTIO_GPU_KMS=y` into the kernel rather than relying on a module
load race.

Evidence: before the built-in config, the graphical smoke logged
`[drm] KMS disabled` and `vwl` found `0 GPUs`. After the built-in config, QEMU
logged `Initialized virtio_gpu`, `fbcon: virtio_gpudrmfb (fb0) is primary
device`, `vwl` found `/dev/dri/card0 (virtio_gpu)`, and the Chromium graphical
smoke passed.

When a kernel driver becomes built-in, the generated module output can
disappear from `pkg/core/kernel/linux.yaml`. Remove any bundle entry that still
names the old module output.

Evidence: after virtio GPU became built-in, `drv-gpu-virtio` disappeared from
the kernel outputs, and `nex check examples/desktop-vwl/desktop-vwl.yaml` failed
until `bundles.all-modules` stopped naming `drv-gpu-virtio`.

Set `TARGET_REF=systems/<slug>/<version>` to boot a nondefault system ref with
the same direct-initramfs path.

```bash
TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1 scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout 180 --headless
```

Evidence: during 002i, that command checked out the kernel, initramfs, and
`systems/desktop-vwl/0.0.1`, created root and var ext4 images, booted QEMU
headlessly with serial logging, reached systemd, and printed
`ASSERT-BOOT-PASS`.

Evidence: during 003e, the `TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1`
variant checked out the Nvidia 580 deployment, booted QEMU headlessly, reached
systemd, and printed `ASSERT-BOOT-PASS`.

The direct-initramfs helper resolves absolute symlinks from the checked-out
deployment through its root content directory before reading boot artifacts.
Host-side `-s` and `cat` checks can otherwise miss files such as
`/boot/amd-ucode.cpio`, which is a symlink into `/nex/pkg` in a checked-out
`nex_structure` root.

For initramfs script changes, use the direct-initramfs QEMU path as the fast
loop first:

```bash
scripts/qemu-test-installer.sh --direct-initramfs --assert-boot
```

This command boots the standalone `pkg/core/kernel/initramfs.yaml` output as
an external QEMU initrd and proves the early mount contract without rebuilding
the Linux package. After it passes, rebuild `pkg/core/kernel/linux.yaml`,
because the normal boot path embeds the packaged initramfs with
`CONFIG_INITRAMFS_SOURCE="/boot/initramfs.cpio"`. Then rebuild bootable
assemblies and run their full QEMU proofs.

Evidence: EP009 changed `pkg/core/kernel/initramfs-init.sh` to initramfs v7
and first proved the mount contract with
`scripts/qemu-test-installer.sh --direct-initramfs --assert-boot`. A normal
assembly boot still used old mount behavior until `pkg/core/kernel/linux.yaml`
was rebuilt, because the kernel package carries the built-in initramfs.

Initramfs v7 mounts the physical root filesystem read-write, bind-mounts the
selected deployment root onto itself, remounts that deployment bind read-only,
binds `/sysroot` read-only, and binds `/nex/repo` plus `/nex/staging` from the
root filesystem as writable paths. It binds `/nex/users` and `/nex/manifests`
from the var partition.

Evidence: EP009's direct-initramfs QEMU assertion printed `ASSERT-BOOT-PASS`
after verifying the selected deployment, read-only `/sysroot`, root-backed
writable `/nex/repo` and `/nex/staging`, and var-backed mutable state.

The direct initramfs image has no ESP partition, so the serial log can show
`Failed to set up automount EFI System Partition Automount` while the boot
proof still passes. Treat that failure as expected for this direct path when
the assertion service verifies readonly `/sysroot`, writable `/var` backed
paths, deployment-root boot, and systemd startup.

## Linux 6.18 module version records

When `CONFIG_MODVERSIONS=y`, Linux 6.18 also needs one record format enabled.
Nex explicitly sets `CONFIG_BASIC_MODVERSIONS=y` in the override fragment and
requires it in the kernel manifest's final config check. The generated
allmod fragment starts from `allnoconfig`; without the explicit override it
can preserve the default-y basic switch as disabled when `olddefconfig` runs.

Evidence: the bad 6.18.24 kernel config enabled `CONFIG_MODVERSIONS` but
enabled neither basic nor extended records. QEMU then rejected `virtio_net`
and `pkcs8_key_parser` with `Exec format error`, and `virtio_net.ko` had no
`__versions` section. The rebuilt `outputs/drv-net-virtio` module contains
that section, and the Systemd, graphical, installer, and live-upgrade guests
loaded the rebuilt kernel successfully.

## Kernel bundle module dependencies

A kernel bundle must include the outputs that own every loadable module named
by `modinfo -F depends`, not only the requested device driver. Linux 6.18.24
`virtio_net.ko` needs `dimlib` and `net_failover`; `net_failover.ko` needs
`failover`. The `vm` bundle already carried the network outputs for both
failover modules but omitted the `lib` output that owns `dimlib.ko`.

Adding `lib` made the `vm` bundle self-contained. Two strict Linux commands,
each with its own reproducibility pass, produced package checksum
`9745a4573612836a5e5a60d294d66300a772fd1ec332fcdba950ed9baff09a91`.
Final Systemd and live-upgrade QEMU guests then loaded virtio networking and
reached SSH.

The `vm` bundle is also missing `overlay.ko` (EP017, 2026-08-19, not yet
fixed). `pkg/core/kernel/linux.yaml:516-524` lists the `vm` bundle's members
as `boot, drv-net-misc, drv-net-virt, drv-net-virtio, drv-virtio, lib,
modules-meta, net-misc` — no `fs-overlay`. The single output that owns
`overlay.ko` (`pkg/core/kernel/linux.yaml:6141-6143`,
`/usr/lib/modules/6.18.24/kernel/fs/overlayfs/overlay.ko`) belongs to exactly
one bundle, `all-modules`, much larger than `vm`. Confirmed on a booted `vm`
guest: `kernel/fs/overlayfs/` does not exist under
`/usr/lib/modules/6.18.24/kernel` at all, `modprobe overlay` fails ("Unknown
symbol in module, or unknown parameter"), `insmod` on the literal expected
path fails ("No such file or directory"), and `/proc/filesystems` has no
`overlay` line. Any guest built from the `vm` bundle therefore cannot run
`mount -t overlay`, which blocks `nex stage` (it unconditionally overlay-mounts
`/usr/bin`, `/nex/pkg`, `/nex/env`) unconditionally. Whether `vm` should grow
an `fs-overlay`-sized addition, the way it grew `lib`, is an open question,
not decided here — see
`.agents/execplans/017-machine-operation-tests.md`.

Update, 2026-08-19: the human resolved the open question above by adding
`x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-overlay` as its own package
entry to `base/nex-systemd.yaml` (not by widening the `vm` bundle itself).
`nex-systemd` checksum `a271d6d1246076e032c99b3a8d2c060baff9004e428c6ae1f1fb9ddb258d31de`.
`nex stage` succeeds on the resulting guest.

## Two packages delivering the same kernel module file break the "direct layer" install

Found rebuilding `examples/desktop-vwl/desktop-vwl.yaml` after the
`nex-systemd` cascade above (EP017, 2026-08-19; corrected 2026-08-19 by the
human after an initial wrong diagnosis on my part — see below).
`desktop-vwl.yaml:67` overrides `linux` to
`x86_64/pkg/core/kernel/linux/6.18.24/bundles/all-modules`, which already
contains the `fs-overlay` output (`overlay.ko`). After `base/nex-systemd.yaml`
gained its own `kernel-fs-overlay` package
(`x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-overlay`), `desktop-vwl.yaml`
inherits *that* too, since it never excluded it. Two package entries both
deliver `/usr/lib/modules/6.18.24/kernel/fs/overlayfs/overlay.ko`, and the
"direct layer" kernel-module installer
(`install_kernel_modules` in `src/cli/src/system/nex.rs`) fails installing the
second one into a location the first one already populated:

    Installing kernel modules: core/kernel/linux/6.18.24 (direct layer)
    Error: Custom { kind: Other, error: "io error at .../target/usr/lib/modules/6.18.24/kernel/fs/overlayfs/overlay.ko: No such file or directory (os error 2)" }

Fix: exclude `kernel-fs-overlay` in `desktop-vwl.yaml`, since
`bundles/all-modules` already carries it. Built reproducibly at
`f07249fc4b6a0c5ba5cd620b4ecba941f484ccb79180e1e37f4be5cfa8e1972b`.

Wrong diagnosis, corrected: I first reported this as a probable `zub`
hardlink-dedup bug in `checkout_from_tree_hash`/`create_hardlink`
(`/home/wegel/work/perso/zub/src/ops/checkout.rs`,
`zub/src/fs/write.rs:152`), because a standalone `zub checkout --copy` of
`bundles/all-modules` alone succeeded, including a repeat `--force` checkout
over itself. Both of those facts were correct but tested the wrong scenario:
neither reproduces two *different* refs (`bundles/all-modules` then
`kernel-fs-overlay`) writing the same destination path in sequence, which is
what a real system build does and what actually fails. The lesson, as put to
me directly: when a build breaks immediately after a manifest change, treat
the change as the first suspect and test *that*, rather than reasoning about
ordering inside the installer from a partial repro.
