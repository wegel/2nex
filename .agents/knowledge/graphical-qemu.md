# Graphical QEMU Tests

`scripts/qemu-test-graphical.sh` boots a Nex desktop in QEMU and proves that a
graphical app renders real pixels. The first supported app is Chromium.

The default command is:

```bash
scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300
```

The script creates a disposable direct-initramfs disk under
`.nex/tmp/graphical-smoke`, checks `asm/desktop-vwl/desktop-vwl.yaml`, checks
out the target system ref, injects guest assertion services, and saves logs and
artifacts under `.nex/tmp/graphical-smoke/artifacts`.

The script uses `egl-headless` with `virtio-vga-gl` when QEMU supports those
features. It falls back to software `virtio-vga` when available, and has a
`gtk-debug` mode for manual debugging. This sandbox needed these Arch packages
before automated graphical QEMU worked:

```bash
qemu-ui-egl-headless qemu-ui-opengl qemu-ui-gtk qemu-hw-display-virtio-vga qemu-hw-display-virtio-vga-gl qemu-hw-display-virtio-gpu qemu-hw-display-virtio-gpu-gl qemu-hw-display-virtio-gpu-pci qemu-hw-display-virtio-gpu-pci-gl qemu-hw-display-qxl
```

Evidence: before those packages, `qemu-system-x86_64 -display help` listed
only `none`, and device help did not list virtio GPU or VGA devices. After
installing them, display help listed `gtk` and `egl-headless`, and device help
listed `virtio-vga`, `virtio-vga-gl`, `virtio-gpu-pci`, and
`virtio-gpu-gl-pci`.

The Chromium guest assertion starts `vwl`, waits for a Wayland socket, checks
`wlr-randr`, launches Chromium with `--ozone-platform=wayland` and a fresh
profile, polls Chromium's local DevTools endpoint for the page title
`NEX_GRAPHICAL_SMOKE_READY`, captures a screenshot with `grim`, and checks the
center pixel with ImageMagick.

The passing EP006 smoke selected `egl-headless-gl`, booted
`systems/desktop-vwl/0.0.1`, logged `/dev/dri/card0 /dev/dri/renderD128`,
reported `dimensions=1280 800`, saw
`remote-debugging-title=NEX_GRAPHICAL_SMOKE_READY`, and verified
`center-pixel=srgb(240,0,255)`.

When the direct-initramfs target contains `/boot/amd-ucode.cpio` or
`/boot/intel-ucode.cpio`, the script prepends those early microcode archives
before the base initramfs and checks their byte offsets with `cmp -n`.

Set `ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub` on this host when the
installed `zub` command lags the sibling checkout. The script routes command
discovery, metadata reads, and every checkout through this override. Its
`--self-test-zub-override` check substitutes `printf` and proves that path.

Size the root image from the checked-out target. Size the writable `/var`
image for both the manifest seed and the Git object store that
`nex-init-manifests` creates from it on first boot. EP013 counted the seed
twice, added 25 percent plus 512 MiB, and selected 11,098 MiB instead of the
old 2,048 MiB. Create large raw images with `truncate` and copy partitions
with `dd conv=notrunc,sparse`; zero-filling tens of GiB only slows the test.

Systemd marks `graphical-session.target` and
`xdg-desktop-autostart.target` with `RefuseManualStart=yes`. A synthetic smoke
session must create its own target and pull those targets in as dependencies.
GNOME phase-tagged autostart entries are reserved for GNOME's session manager;
the generic Systemd smoke can set `XDG_CURRENT_DESKTOP=Unity` when the same
entries also declare Unity.

Evidence: the final EP013 guest reported successful generated AT-SPI and Gnome
Keyring units, a live AT-SPI D-Bus address, Chromium title
`NEX_GRAPHICAL_SMOKE_READY`, dimensions `1280 800`, center pixel
`srgb(240,0,255)`, and `ASSERT-GRAPHICS-PASS`.
