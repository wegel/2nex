# Installer

## USB Image Builder

The full USB installer image path needs host tools `parted`, `mkfs.vfat`,
`mcopy`, `mmd`, `mke2fs`, `debugfs`, `fakeroot`, and
`qemu-system-x86_64`. On the Arch sandbox, `parted`, `dosfstools`, and
`mtools` supplied the missing tools.

Evidence: EP007 found only `mke2fs`, `debugfs`, `fakeroot`, and
`qemu-system-x86_64` in the sandbox at first. Installing `parted`,
`dosfstools`, and `mtools` made the full USB image builder usable.

`scripts/create-installer-usb` must read `nex.build.checksum` when
`nex.system.checksum` is absent, then fall back to `zub rev-parse`. Current
built system metadata can use `nex.build.checksum`.

The USB image builder must size the root filesystem from the staged content,
not from a fixed constant. `desktop-vwl` already exceeded the older 3 GiB root
image.

A full USB installer image needs a little slack after the ext4 root image.
When the ext4 image size exactly matched the partition end, Linux logged
`EXT4-fs (sda2): bad geometry` and refused to mount the installer root. EP007
fixed this by leaving 8 MiB of partition slack.

Assembly overlay entries can copy a checked-in source file with `source:`,
relative to the overlay YAML directory. `asm/installer/installer-overlay.yaml`
uses this to install `scripts/nex-install` at `/usr/bin/nex-install`.

Evidence: `src/cli/src/manifest/types.rs` defines `OverlayEntry.source`, and
`src/cli/src/system/overlays.rs` copies the source path relative to the overlay
directory.

## Booted Installer

The booted installer uses `/nex/installer/system`,
`/nex/installer/system-checksum`, `/nex/installer/BOOTX64.EFI`, and
`/nex/installer/kcmdline.txt` from the installer deployment. Do not use the old
`/run/installer` layout.

The installer should initialize the installed zub repo with
`zub init /run/sysroot/var/nex/repo`. `zub init` takes the repo path as a
positional argument. Do not use `zub --repo <path> init` for this job because
`--repo` selects an existing repo for commands that open one, while `init`
creates the current directory unless given a path.

Evidence: `zub init --help` prints `Usage: zub init [PATH]`. A smoke that ran
`/usr/bin/zub --repo "$tmpdir/test-repo" init` from the repository root wrote
a root-level `config.toml`; the corrected smoke ran packaged zub with
`zub init "$tmpdir/repo"` and created `$tmpdir/repo/config.toml`.

Future installed desktop systems should carry the standalone `zub` command.
The installer can seed remotes after copying the system, but it can only create
`/var/nex/repo/config.toml` when `/usr/bin/zub` exists in the installer
runtime.

Evidence: the hardware install had `/usr/bin/nex` but no `zub`; `/nex/repo`
contained only `remotes/`, so `nex deploy ... --repo /nex/repo` would fail
before it could pull or deploy a system ref.

The installed target stores `/nex/repo`, `/nex/staging`, and
`/nex/deployments` on the root filesystem, not under `/var/nex`. The
initramfs keeps the selected deployment and `/sysroot` views read-only, but
binds `/nex/repo` and `/nex/staging` as writable root-backed paths. This lets
zub checkouts hardlink deployment files to repo blobs.

Keep `/nex/users` and `/nex/manifests` under the var partition. Those paths
are mutable machine state and do not need to share inodes with deployment
objects.

Evidence: EP009's direct-initramfs QEMU assertion verified `/nex/repo` and
`/nex/staging` come from the root source and are writable, while `/nex/users`
and `/nex/manifests` come from the var source. The focused
`scripts/test-live-upgrade-hardlinks.sh` test proves that a same-filesystem
upgrade shares deployment files with repository blobs. The full QEMU fixture
can use a var-backed repository and report `repo-copy`; it separately proves
that rollback reuses the original deployment inode on the root filesystem.

`pkg/libs/tui/newt.yaml` provides `/usr/bin/whiptail` in `outputs/bin`, so the
installer can show a TUI without adding a large graphical stack. Keep a plain
prompt fallback for noninteractive shells and automated tests.

`pkg/core/embedded/busybox.yaml` can give the installer a poweroff path without
installing BusyBox applet symlinks over normal tools. Add
`x86_64/pkg/core/embedded/busybox/1.36.1/outputs/bin` and call
`busybox poweroff -f` directly.

The installed target root partition must use GPT partition name `nex`. The
bootloader searches for that partition name, not for filesystem label
`nex-root`.

Evidence: EP007 saw a target boot fail with `PartitionNotFound` after the
installer named the partition `nex-root`. `src/bootloader/src/disk.rs` defines
`ROOT_PARTITION_NAME` as `nex`.

## QEMU Installer Proof

`scripts/qemu-test-installer.sh` must pass `TARGET_REF` through to
`scripts/create-installer-usb`. Otherwise a variant test can silently build an
installer that still embeds plain `systems/desktop-vwl/0.0.1`.

The full installer QEMU assertion should not require `multi-user.target`.
`desktop-vwl` is meant to leave session startup fairly manual, and desktop
targets can boot toward `graphical.target`. After SSH works, wait for
`systemctl is-system-running` to settle to `running` or `degraded`.

EP007 full USB proofs passed for both plain and Nvidia 580 desktops:

```bash
TARGET_REF=systems/desktop-vwl/0.0.1 scripts/qemu-test-installer.sh --rebuild --autoinstall --headless --extra-nex-var --assert-boot --timeout 600
TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1 scripts/qemu-test-installer.sh --rebuild --autoinstall --headless --extra-nex-var --assert-boot --timeout 600
```

Both runs ended with `ASSERT-BOOT-PASS`.

Autoinstall and interactive installer images should use different default
paths in the QEMU harness. Otherwise a headless autoinstall run can reuse an
older interactive image that lacks `installer.autoinstall=<disk>` on the
kernel command line.

When formatting an already-used QEMU target disk, call `mkfs.ext4` with `-F`.
Without `-F`, old ext4 signatures can make the formatter prompt and stall an
automated installer proof.

Evidence: EP009 changed `scripts/qemu-test-installer.sh` to use
`.nex/tmp/installer-autoinstall-<target>.img` for autoinstall by default and
changed `scripts/nex-install` to call `mkfs.ext4 -F -q`. The command
`scripts/qemu-test-installer.sh --headless --rebuild --autoinstall --assert-boot`
then installed `systems/desktop-vwl/0.0.1`, booted it, reported
`systemd-state=running`, and printed `ASSERT-BOOT-PASS`.

The direct-initramfs harness must measure its checked-out target rather than
use a fixed root-image size. Round used KiB up to MiB, add 25 percent plus 512
MiB, and retain the configured minimum for small roots. EP013's desktop no
longer fit in 6,144 MiB; the measured formula selected 13,585 MiB and booted
the exact stored deployment to `ASSERT-BOOT-PASS`. Use `truncate` and sparse
`dd` copies so the raw test disk does not materialize every zero-filled block.

The full live-upgrade QEMU fixture keeps a complete local source repository in
`/var`, then pulls it into the guest's writable repository. Size `/var` from
the staged payload plus two additional source-repository sizes plus 2,048 MiB.
One additional size covers the final pulled repo; the second leaves working
space and filesystem overhead while Zub writes objects.

Evidence: a 9,711 MiB source repo exhausted a 21,470 MiB `/var` image during
pull. A 31,181 MiB image pulled 9,617,341,112 bytes across 164,463 objects and
completed source boot, upgrade, upgraded boot, rollback, and rollback boot.
Probe `usr/share/factory/etc/os-release`, the regular immutable file. Do not
probe absent `etc/os-release` or the `usr/lib/os-release` compatibility
symlink. Multi-command SSH probes should start with `set -eu` so a failed file
check cannot be hidden by a later successful `printf`.

## Provisioning standard machine files

`nex-install --provision ETC_TREE` accepts a closed set of standard machine
files: hostname, hosts, localtime, locale.conf, vconsole.conf, fstab, passwd,
group, shadow, NetworkManager `.nmconnection` files, and Systemd `.network`,
`.netdev`, and `.link` files. Validate the complete source tree before writing
the target so one rejected path cannot leave a partially provisioned machine.

Only `localtime` may be a symlink. Require its canonical target below
`/usr/share/zoneinfo` and reject traversal, repeated separators, directory
links, and special files. Install root-owned files with 0600 for shadow and
NetworkManager connection secrets and 0644 for other regular files.

Evidence: `scripts/test-nex-install-provision.sh` exercises every accepted
class, rejects arbitrary files, empty directories, unsafe links, traversal,
and a FIFO, and proves the preflight prevents partial writes. Against a copied
final Nex Systemd root, the installed Glibc `getent` resolves both `localhost`
and the provisioned hostname from the provisioned hosts file.
