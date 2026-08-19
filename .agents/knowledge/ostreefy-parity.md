# OSTreefy Parity

## Source Files

The live OSTreefy source tree exists outside this checkout at:

```text
/home/wegel/work/wegelcorp/ostreefy
```

Use these files as the primary source for personal-system parity:

```text
/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/Containerfile
/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/examples/Containerfile.wegel
```

The archived fallback copy is:

```text
/home/wegel/work/wegelcorp/nex.archive-20260627T143546Z/tmp/ostreefy
```

## Matrix Count

The frozen parity matrix has 163 unique inputs when it counts base packages,
personal packages, the AUR loop, and the local package archive entry once.

Evidence: the 002b consistency script compared
`.agents/ostreefy-parity-matrix.md` with both live Containerfiles plus
`wlopm`, `nvidia-580xx-dkms`, `looking-glass`,
`nvidia-container-toolkit`, `openconnect-sso`, and
`ungoogled-chromium-local`; it printed
`expected=163 rows=163 unique_rows=163`.

## Parity Rule

Track program and behavior parity, not Arch package-name parity. Nex package
names may differ when Nex conventions split or combine outputs differently.

The local ungoogled Chromium archive is deferred by human request for now.

## Desktop Session Choices

The 002e desktop-session subplan covered or replaced its personal desktop
rows. These decisions should stand unless a later plan changes the user's
program set:

- `foot` replaces Alacritty and Kitty in `desktop-vwl`.
- `gnome-text-editor`, Neovim, and Helix replace Mousepad.
- `vwl` replaces Sway as the Wayland compositor.
- `ironbar` replaces Waybar.
- `fuzzel` replaces Wofi.
- `mako` replaces SwayNC.
- `qt6-wayland` covers the old `qt5-wayland` row for the current Nex Qt 6
  stack.
- `noto-fonts` covers the `noto-fonts-extra` row.
- `jack2` and `jack-example-tools` cover JACK command needs while Nex
  PipeWire still disables PipeWire JACK support.
- `xorg-xeyes` was skipped because it is an X.Org demo app and would require
  adding the Athena widget stack only for a demo row.

Evidence: `.agents/ostreefy-parity-matrix.md` marks those rows as covered,
replaced, or skipped under ExecPlan `002e`, and the checked-out
`desktop-vwl` smokes named in that subplan verified the relevant public
commands or files.

## Browser And Media Choices

The 002g large-apps subplan covered VLC and chose Chromium as the supported
browser path for the Firefox and Vimb rows. The custom local ungoogled
Chromium archive remains deferred by human request.

- `vlc` is covered by `pkg/apps/multimedia/vlc.yaml` and `desktop-vwl`.
- Chromium replaces Firefox in `desktop-vwl`.
- Chromium replaces Vimb because Nex does not yet package Vimb or WebKitGTK.

Evidence: `desktop-vwl` includes VLC and Chromium after 002g. Checked-out
system smokes ran `vlc`, `cvlc`, `nvlc`, and `rvlc` far enough to hit VLC's
root-user guard, ran `vlc-cache-gen` against 238 VLC plugin shared objects,
and ran `chromium --version` plus `chromedriver --version`.

## Policy-Heavy Rows

The 002h subplan covered the rows that Nex can prove today and left the rest
deferred with concrete policy decisions:

- `linux-headers` is covered in `desktop-vwl`.
- `ntp` is replaced by systemd-timesyncd from `nex-systemd`.
- `wlopm` is covered by a small package in `desktop-vwl`.
- `amd-ucode` is covered by `pkg/core/kernel/amd-ucode-initramfs.yaml` and
  the base `desktop-vwl` assembly. The Nvidia desktop variants inherit it.
- `lib32-vulkan-radeon` and `steam` wait for 32-bit runtime and proprietary
  app policies.
- `nvidia-580xx-dkms` is covered by
  `pkg/libs/graphics/nvidia-580.yaml` and
  `asm/desktop-vwl-nvidia-580.yaml`. The checked-out root proves
  the 580.159.04 module, userspace tools, and Nvidia GL/Vulkan vendor files.
  `nvidia-container-toolkit` still waits for a container-runtime hook package
  and proof.
- `looking-glass` is covered by `pkg/apps/virt/looking-glass.yaml` for the
  Linux client and `pkg/core/kernel/kvmfr.yaml` for host shared-memory support.
  The Nvidia desktop variants include both packages and root smokes ran the
  client help command.
- `openconnect-sso` waits for AUR/Python desktop authentication-helper policy
  and a confirmation that the helper is still used.
- `v4l2loopback-dkms` and `v4l2loopback-utils` are covered by
  `pkg/core/kernel/v4l2loopback.yaml`; the Nvidia desktop variants include the
  runtime bundle.

Evidence: `desktop-vwl` includes Linux headers and `wlopm` after 002h. A
checked-out `desktop-vwl` root passed `unshare --root` smokes for
`/usr/include/linux/eventpoll.h`, `/usr/include/asm/unistd_64.h`,
`/usr/include/drm/drm.h`, `systemd-timesyncd --help`, the systemd NTP unit
list, `wlopm --version`, the `wlopm` man page, and the `wlopm` bash
completion file.

AMD microcode evidence: ExecPlan 004 built `/boot/amd-ucode.cpio`
reproducibly, found `kernel/x86/microcode/AuthenticAMD.bin` inside it, found
`/boot/amd-ucode.cpio` in all three desktop roots with
`zub --repo .nex/repo ls-tree -r`, and booted
`systems/desktop-vwl-nvidia-580/0.0.1` through direct QEMU after the helper
prepended the microcode cpio before the base initramfs.

Looking Glass package evidence: the client package passed a strict two-pass
build with checksum
`65c5066d414cb87fc33596c0acd39ecb2e9e266ed87e31abfd8bb4ef57078232`, and a
manual runtime smoke printed `Looking Glass (B7)` plus the option tables under
the Nex dynamic loader. The `kvmfr` package passed a strict two-pass build
with checksum
`2614bb26b92913318921fea27be8e9eab7b7008fc56b3ce705b42ecbdb991b8d`, and
`modinfo` reported vermagic `6.12.58 SMP modversions`.

## Final Validation

The 002i final validation found no remaining vague `needs-*` rows in the
parity matrix. All open rows are explicit request or policy deferrals.

`desktop-dev` covers the development tool layer after switching build tools
from bare `outputs/bin` refs to full/dev bundles and fixing the Meson launcher.
The rebuilt assembly passed a reproducible build with checksum
`a0b3c72bc6b8e4f815e897e1998dce51c2d899bdeb1cbd486a93b6fab5c18588`, and a
checked-out root ran the final dev-tool smoke suite.

The boot path was proven with
`scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout
180`. The guest booted `systems/desktop-vwl/0.0.1`, reached systemd, verified
the readonly deployment root and writable var-backed paths, and printed
`ASSERT-BOOT-PASS`.
