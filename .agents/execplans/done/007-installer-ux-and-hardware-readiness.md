# Make The USB Installer Pleasant And Hardware-Ready

This ExecPlan is a living document. Agents must keep `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective`
current as work proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex has a USB installer path, but a person should not need repo context to use
it on a real computer. The installer should build an image that is large
enough for the selected target system, boot reliably on hardware, explain what
it will erase, guide the user through disk choice, install the chosen desktop,
and then boot that installed system. A coding agent must be able to prove this
with QEMU before anyone tries a real USB stick.

The current installer path is close but not ready enough. The image builder
hardcodes a 3 GiB root filesystem even though `desktop-vwl` currently checks
out to about 3.9 GiB. The installer manifest currently fails `nex check` on
formatting. The full USB-image test needs host image tools that this sandbox
does not currently have. The current QEMU proof mostly covers the installed
boot path through direct initramfs mode, not the full image creation,
installer boot, disk partitioning, copied target system, and rebooted target.

After this plan, `desktop-vwl` and the Nvidia desktop variants can be installed
from a generated USB image with a clear command and a friendly in-installer
flow. The repo will have an automated full installer test that creates the
image, runs unattended install in QEMU, boots the target disk, and asserts the
mount and system state.

## Progress

- [x] (2026-07-01) Created this ExecPlan after the human confirmed that the
  installer must handle the larger desktop and asked for the installer UX to
  feel pleasant in general.
- [x] Run the Ralph worktree pre-task and record what dirty or untracked files
  were committed, ignored, removed, or left alone.
- [x] Read the required root docs and relevant knowledge notes:
  `kernel-and-boot.md`, `system-assemblies.md`, `graphical-qemu.md`,
  `cli-testing.md`, and `reproducibility.md`.
- [x] Install any missing host tools in the sandbox that the full USB image
  path needs.
- [x] Fix formatting and checks for `asm/installer/installer.yaml`.
- [x] Make `scripts/create-installer-usb` size the installer image from the
  actual selected installer and target system refs.
- [x] Make `scripts/create-installer-usb` accept explicit target, image-size,
  root-size, hardware, and VM options with clear validation and useful error
  text.
- [x] Improve the booted installer UX in `asm/installer/installer-overlay.yaml`.
- [x] Make the installer target hardware-friendly defaults for
  `desktop-vwl-nvidia-580` and `desktop-vwl-nvidia-current` easy to choose.
- [x] Make the QEMU installer harness prove the full USB image install path.
- [x] Run the full USB autoinstall test and then boot the installed target
  disk with assertions.
- [x] Record all commands, logs, artifacts, and durable lessons.
- [x] Commit the checked, buildable installer slice with scoped imperative
  subject `installer: improve usb install flow` (`c74a009`).

## Surprises & Discoveries

- Observation: `scripts/create-installer-usb` already accepts a target system
  ref and the QEMU helper already passes `systems/desktop-vwl/0.0.1`.
  Evidence: `scripts/qemu-test-installer.sh` calls
  `scripts/create-installer-usb systems/desktop-vwl/0.0.1 "$INSTALLER_IMG"`.

- Observation: the current desktop no longer fits the hardcoded installer root
  image.
  Evidence: `zub --repo .nex/repo du systems/desktop-vwl/0.0.1` reports
  `3802.8 MB`, and a checkout measured `3.9G` allocated size. The USB builder
  sets `ROOT_SIZE_MB=3072`.

- Observation: the booted installer uses the overlay copy of `nex-install`,
  not the top-level `scripts/nex-install`.
  Evidence: `asm/installer/installer-overlay.yaml` installs
  `/usr/bin/nex-install` with support for `/var`, `--yes`, `--root-size`, and
  remote seeding. The top-level `scripts/nex-install` still uses older paths
  such as `/run/installer`.

- Observation: the full USB path currently depends on host image tools.
  Evidence: `scripts/create-installer-usb` requires `parted`, `mkfs.vfat`,
  `mke2fs`, `mcopy`, and `debugfs`. In this sandbox, `mke2fs`, `debugfs`,
  `fakeroot`, and `qemu-system-x86_64` exist, while `parted`, `mkfs.vfat`,
  and `mcopy` were missing when this plan was written.

- Observation: `asm/installer/installer.yaml` currently fails the manifest
  checker before any behavior test can trust it.
  Evidence: `./src/cli/target/debug/nex check asm/installer/installer.yaml`
  printed `error: needs formatting`.

- Observation: recent boot proof does not prove the full USB install path.
  Evidence: durable knowledge records
  `scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout
  180` passing for `desktop-vwl`. That path skips `create-installer-usb`,
  installer media boot, partitioning by `nex-install`, and copying the target
  system from the installer image.

- Observation: the full USB image needed partition slack after the ext4 image.
  Evidence: QEMU logged `EXT4-fs (sda2): bad geometry` when the ext4 image
  size matched the partition end exactly. Adding 8 MiB of partition slack made
  the installer root mount.

- Observation: the bootloader finds the root filesystem by GPT partition name
  `nex`.
  Evidence: the target disk boot failed with `PartitionNotFound` after the
  installer created a partition named `nex-root`. `src/bootloader/src/disk.rs`
  defines the root partition name as `nex`.

- Observation: the boot assertion must not require `multi-user.target`.
  Evidence: `desktop-vwl` booted, accepted SSH, and mounted the expected
  filesystems, but the old probe failed because it required
  `multi-user.target`. The updated probe waits for systemd to settle to
  `running` or `degraded`.

- Observation: the Nvidia 580 assembly ref was absent from the local store and
  several kernel-driver package checksums were stale.
  Evidence: `zub --repo .nex/repo rev-parse
  systems/desktop-vwl-nvidia-580/0.0.1` returned `ref not found`. Strict
  two-pass builds updated `kvmfr`, `nvidia-580`, and `v4l2loopback` after each
  package produced matching first and second checksums.

- Observation: the installer checksum changes when package manifests change.
  Evidence: after the kernel-driver manifest updates, the installer checksum
  changed from `75d6fdeefd99732cf626303636924e26056433050cf9c599aeffb1fb1ae9f934`
  to `ca2299734945b8e9e1a2b6b4415c30ff8a4040ee2863c15117c2fd85d0be7849`
  because the installer embeds `/nex/db/pkg`.

## Decision Log

- Decision: Keep one generated USB image tied to one target system ref.
  Rationale: The current image embeds a checked-out target root, not a zub
  repository with deduplicated system refs. Carrying plain `desktop-vwl`, 580,
  and current Nvidia variants together would bloat the USB image and make the
  first hardware test harder to reason about.
  Date/Author: 2026-07-01 / Carlos

- Decision: Size the image from the real materialized content, not from a fixed
  constant.
  Rationale: The desktop grows as packages land. A fixed `ROOT_SIZE_MB=3072`
  already became wrong. The script should compute the required root filesystem
  size from the staged installer root, add a clear margin, and let the user
  override it with an option.
  Date/Author: 2026-07-01 / Carlos

- Decision: Use `whiptail` for a nicer TUI when a real terminal has it, and
  keep a plain prompt fallback for scripts and minimal shells.
  Rationale: The installer should feel guided on real hardware, while QEMU
  tests and emergency shells still need a simple command-line path that an
  agent can drive and assert.
  Date/Author: 2026-07-01 / Carlos

- Decision: Make the full USB autoinstall test the acceptance gate.
  Rationale: Direct initramfs boot is useful, but it does not catch broken USB
  image sizing, missing image tools, bad installer root layout, bad
  `nex-install` arguments, partitioning errors, or copy failures.
  Date/Author: 2026-07-01 / Carlos

- Decision: Treat real hardware instructions as a checked artifact, not only
  tribal knowledge.
  Rationale: A user should see the commands to build the correct Nvidia 580 or
  current image, write it to a USB device, boot it, and install it. The docs
  must name the destructive step and how to identify the target disk.
  Date/Author: 2026-07-01 / Carlos

## UX Contract

The image builder should:

- print the selected target ref, target checksum, installer checksum, and
  chosen kernel command line mode
- fail early when host tools are missing, with package names or tool names
  that a user can act on
- compute a root filesystem size from the staged content and show the
  calculated size plus margin
- expose `--vm`, `--hardware`, `--kcmdline`, `--root-size`, and
  `--image-size` or equivalent options
- refuse to overwrite an existing output image unless the user passes an
  explicit force option
- write concise logs and keep noisy debug output behind a verbose flag

The booted installer should:

- print a short welcome message that says which system it will install
- show disks as numbered choices with name, size, model, and whether the disk
  looks like the installer medium
- never default to erasing a disk without an explicit choice
- show the partition plan before writing: 512 MiB ESP, root size, remaining
  var size
- ask the user to type a specific confirmation string that includes the target
  disk name
- support `--yes` for QEMU autoinstall only when a target disk argument is
  present
- show progress for partitioning, formatting, copying, ESP setup, and final
  unmount
- print the final action: remove USB and reboot
- print actionable errors that name the command or path that failed

## Implementation Plan

1. Run the pre-task:
   - `git status --short --untracked-files=all`
   - classify any dirty files
   - commit only coherent existing work after matching checks pass

2. Prepare the sandbox:
   - install missing host image tools as needed
   - record exact package/tool names in `.agents/SCRATCH_KNOWLEDGE.md`
   - do not commit host state

3. Fix installer manifest hygiene:
   - run `./src/cli/target/debug/nex format asm/installer/installer.yaml`
   - run `./src/cli/target/debug/nex check asm/installer/installer.yaml`
   - inspect the formatter diff before committing

4. Improve `scripts/create-installer-usb`:
   - replace the hardcoded root filesystem size with computed sizing
   - stage the installer root before creating the final disk image
   - measure staged content with `du -sm` and add a safe margin
   - support explicit overrides and a force flag
   - read `nex.build.checksum` when `nex.system.checksum` is absent, because
     current zub metadata uses `nex.build.checksum`
   - make status output concise by default
   - keep a verbose/debug mode for filesystem inspection

5. Improve the booted installer:
   - update `asm/installer/installer-overlay.yaml`
   - make the shell UX guided and explicit
   - keep `--yes` and `--root-size` for automated tests
   - keep the installed layout compatible with the current initramfs boot path
   - update or remove the stale top-level `scripts/nex-install`, so the repo
     does not carry two conflicting installer stories

6. Improve target selection:
   - update scripts or docs to show plain `desktop-vwl`, Nvidia 580, and Nvidia
     current examples
   - make `TARGET_REF` override work consistently in
     `scripts/qemu-test-installer.sh`
   - use `systems/desktop-vwl-nvidia-580/0.0.1` as the human's likely current
     machine target when testing Nvidia-specific installer media

7. Prove the full path:
   - build the installer system
   - build the selected desktop system if missing
   - create a USB image with `scripts/create-installer-usb`
   - run QEMU autoinstall from that USB image to a blank target disk
   - boot the target disk
   - assert the booted system has readonly deployment root, writable `/var`,
     correct `/etc`, `/home`, `/root`, and `/nex/*` mounts, and systemd active
   - save serial logs under `.nex/tmp`

8. Commit in small buildable slices:
   - manifest formatting and installer check if separate
   - image builder sizing and CLI/UX
   - booted installer UX
   - QEMU harness assertion updates
   - docs or knowledge updates

## Validation and Acceptance

The plan is complete only when all of these pass:

```bash
sh -n scripts/create-installer-usb
sh -n scripts/qemu-test-installer.sh
./src/cli/target/debug/nex check asm/installer/installer.yaml
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl-nvidia-580.yaml
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl-nvidia-current.yaml
```

The installer system must build:

```bash
./src/cli/target/debug/nex build asm/installer/installer.yaml --verbose
```

The full USB installer test must pass for the default desktop:

```bash
TARGET_REF=systems/desktop-vwl/0.0.1 \
  scripts/qemu-test-installer.sh --rebuild --autoinstall --headless \
  --extra-nex-var --assert-boot --timeout 600
```

The full USB installer test must also pass for the human's likely hardware
target unless a hard blocker appears:

```bash
TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1 \
  scripts/qemu-test-installer.sh --rebuild --autoinstall --headless \
  --extra-nex-var --assert-boot --timeout 600
```

If the Nvidia 580 test cannot run in the sandbox because a required package ref
is missing from the store, build the assembly first with the normal assembly
build command and rerun the full USB test. If a host QEMU capability is
missing, install the needed sandbox package and record it.

The agent must inspect the generated USB image size and record:

- target ref
- installer ref
- target checksum
- installer checksum
- computed content size
- final image size
- QEMU serial log path
- final assertion marker

## Outcomes & Retrospective

The installer path now has a usable script backbone and a friendlier booted
installer. `scripts/create-installer-usb` builds a target-specific image,
sizes the root filesystem from staged content, prints target and installer
checksums, supports VM and hardware command-line modes, and refuses accidental
overwrite without `--force`. `scripts/nex-install` is the single installed
installer script, supports whiptail when a TTY is available, keeps a plain
prompt fallback, suggests a root size from the embedded target system, and
requires an explicit destructive confirmation.

The QEMU harness now proves the full path. It can create the USB image, run
the installer unattended, boot the installed disk, and assert the deployment
root, writable state mounts, `/nex` state mounts, SSH, and systemd state. It
also accepts `TARGET_REF`, so the same proof works for plain `desktop-vwl` and
the Nvidia 580 target.

Validated commands:

```bash
bash -n scripts/nex-install
sh -n scripts/create-installer-usb
sh -n scripts/qemu-test-installer.sh
./src/cli/target/debug/nex check asm/installer/installer.yaml asm/desktop-vwl/desktop-vwl.yaml asm/desktop-vwl/desktop-vwl-nvidia-580.yaml asm/desktop-vwl/desktop-vwl-nvidia-current.yaml pkg/core/kernel/kvmfr.yaml pkg/core/kernel/v4l2loopback.yaml pkg/libs/graphics/nvidia-580.yaml
./src/cli/target/debug/nex build asm/installer/installer.yaml --verbose
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl-nvidia-580.yaml --verbose
TARGET_REF=systems/desktop-vwl/0.0.1 scripts/qemu-test-installer.sh --rebuild --autoinstall --headless --extra-nex-var --assert-boot --timeout 600
TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1 scripts/qemu-test-installer.sh --rebuild --autoinstall --headless --extra-nex-var --assert-boot --timeout 600
```

Final image facts:

- Plain desktop target checksum:
  `9dd302e544e4e89db84a0d1e99e13a4f3aa46758a576aebc87ace33226722a06`
- Nvidia 580 target checksum:
  `e8e27a242c757a636e784f46b316d25b3375e67c2e7d3c0c0b72f93846094f12`
- Installer checksum:
  `ca2299734945b8e9e1a2b6b4415c30ff8a4040ee2863c15117c2fd85d0be7849`
- Plain desktop image:
  3996 MiB content, 5020 MiB root filesystem, 5093 MiB disk image
- Nvidia 580 image:
  5094 MiB content, 6118 MiB root filesystem, 6191 MiB disk image
- Both QEMU runs ended with `ASSERT-BOOT-PASS`.

## Context and Orientation

Read these files first:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/PLANS.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/graphical-qemu.md`
- `.agents/knowledge/reproducibility.md`

Relevant files:

- `scripts/create-installer-usb`
- `scripts/qemu-test-installer.sh`
- `scripts/nex-install`
- `asm/installer/installer.yaml`
- `asm/installer/installer-overlay.yaml`
- `src/bootloader/kcmdline.hardware.txt`
- `src/bootloader/kcmdline.vm.txt`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`
- `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml`

Useful current facts:

- `desktop-vwl` measured about `3802.8 MB` by `zub du` and about `3.9G` as a
  checked-out tree.
- `scripts/create-installer-usb` currently hardcodes `ROOT_SIZE_MB=3072`.
- The installed boot assertion path has passed through direct initramfs mode,
  but the full USB image autoinstall path needs fresh proof.
- The human's current machine has a GTX 1060 and an RTX 2070, so the Nvidia
  580 desktop variant is the likely first real-hardware target.
