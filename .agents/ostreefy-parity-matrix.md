# OSTreefy Personal System Parity Matrix

This matrix records the old OSTreefy personal system inputs and the Nex work
needed to match the same user-visible behavior. It follows programs and
behavior rather than Arch package names when Nex uses a different package name
or splits files into separate outputs.

Sources:

- `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/Containerfile`
- `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/examples/Containerfile.wegel`
- `OSTREEFY_REPLICATION_REPORT.md`
- `asm/nex-systemd.yaml`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-dev.yaml`

Status values:

- `covered`: current Nex manifests and assemblies already provide the program,
  library, service, or behavior.
- `replaced`: Nex deliberately provides the behavior with a different base
  technology or desktop choice.
- `needs-manifest`: Nex should add or finish a package manifest before parity.
- `needs-assembly`: Nex has a manifest or partial package path, but an assembly
  still must include it or a runtime proof must verify it.
- `deferred-policy`: Nex needs a policy or hardware decision before adding it.
- `deferred-request`: the human explicitly said to skip it for now.

## Summary

- Base Containerfile packages: 15
- Personal Containerfile packages: 142
- AUR loop packages: 5
- Local package archive entries: 1
- Unique matrix rows: 163

## Matrix

| Item | Source | Status | Handler | Proof required before final parity |
| --- | --- | --- | --- | --- |
| aardvark-dns | base | covered | none | `desktop-vwl` checkout contains `/usr/bin/aardvark-dns` or equivalent container DNS helper. |
| age | personal | covered | none | `desktop-vwl` includes `age`; system smoke ran `age --version`. |
| alacritty | personal | replaced | 002e | Replaced by the existing `foot` terminal in `desktop-vwl`; checked-out roots expose `/usr/bin/foot`, and `ncurses` already provides Alacritty terminfo. |
| alsa-utils | personal | covered | none | `desktop-vwl` includes `alsa-utils`; checkout smoke for `aplay --version`. |
| amd-ucode | base | covered | 004 | Nex packages `/boot/amd-ucode.cpio`, desktop assemblies include it, and direct QEMU prepends it before the base initramfs. |
| ast-grep | personal | covered | none | `desktop-dev` includes `ast-grep`; system smoke ran `ast-grep --version`. The assembled `/usr/bin/sg` remains Shadow's group command. |
| aws-cli | personal | covered | 002d | `pkg/cli/net/aws-cli.yaml` builds reproducibly; `desktop-dev` includes it and a checked-out system smoke ran `aws --version` without network access. |
| base-devel | personal | covered | none | `desktop-dev` covers the build toolchain group; checkout smoke for compiler, make, pkg-config, autoconf, automake, and libtool. |
| bat | personal | covered | none | `desktop-vwl` includes `bat`; checkout smoke `bat --version`. |
| bc | personal | covered | none | `desktop-vwl` includes `bc`; checkout smoke `bc --version`. |
| bluez | personal | covered | none | `desktop-vwl` includes BlueZ runtime files and tools. |
| bluez-utils | personal | covered | none | Nex BlueZ package should provide user tools; checkout smoke `bluetoothctl --version` or matching BlueZ tool. |
| bmon | personal | covered | none | `desktop-vwl` includes `bmon`; system smoke ran `bmon -V`. |
| bottom | personal | covered | none | `desktop-vwl` includes `bottom`; checkout smoke `btm --version`. |
| broot | personal | covered | none | `desktop-vwl` includes `broot`; checkout smoke `broot --version`. |
| btop | personal | covered | none | `desktop-vwl` includes `btop`; system smoke ran `btop --version`. |
| chezmoi | personal | covered | none | `desktop-vwl` includes `chezmoi`; system smoke ran `chezmoi --version`. |
| cloc | personal | covered | none | `desktop-dev` includes `cloc`; system smoke ran `cloc --version`. |
| cmake | personal | covered | none | `desktop-dev` includes `cmake`; checkout smoke `cmake --version`. |
| curl | personal | covered | none | `nex-systemd` includes `curl`; checkout smoke `curl --version`. |
| dmidecode | personal | covered | 002d | `pkg/cli/system/dmidecode.yaml` builds reproducibly; `desktop-dev` smoke ran `dmidecode --version` and printed `3.7`. |
| docker | base | replaced | 002f | Nex chooses Podman for container runtime parity; checked-out `desktop-vwl` smoke ran `podman --version` with `/proc` mounted and host `CONTAINERS_CONF` unset. |
| docker-buildx | base | replaced | 002f | Nex chooses Podman build tooling; checked-out `desktop-vwl` smoke ran `podman build --help` without network access. |
| dool | personal | covered | none | `desktop-vwl` includes `dool` and Python; system smoke ran `dool --version`. |
| dosfstools | base | covered | none | `desktop-vwl` includes `dosfstools`; checkout smoke `mkfs.fat -V`. |
| distrobox | personal | covered | 002f | `desktop-vwl` includes `distrobox`; system smoke ran `distrobox --version` and `distrobox-create --help`. |
| dua-cli | personal | covered | none | `desktop-vwl` includes `dua-cli`; checkout smoke `dua --version`. |
| duf | personal | covered | none | `desktop-vwl` includes `duf`; checkout smoke `duf --version`. |
| dust | personal | covered | none | `desktop-vwl` includes `dust`; checkout smoke `dust --version`. |
| efibootmgr | personal | covered | 002d | `pkg/cli/system/efibootmgr.yaml` builds reproducibly; `desktop-dev` includes it and a checked-out system smoke ran `efibootmgr --version` plus `efibootdump --help`. |
| eza | personal | covered | none | `desktop-vwl` includes `eza`; checkout smoke `eza --version`. |
| fakeroot | personal | covered | 002d | `pkg/dev/tools/fakeroot.yaml` builds reproducibly; `desktop-dev` smoke ran `fakeroot --version` and `fakeroot id -u`, printing `fakeroot version 1.38.1` and `0`. |
| fd | personal | covered | none | `desktop-vwl` includes `fd`; checkout smoke `fd --version`. |
| ffmpeg | personal | covered | none | `desktop-vwl` includes `ffmpeg`; checkout smoke `ffmpeg -version`. |
| firefox | personal | replaced | 002g | Nex uses `pkg/apps/web/chromium.yaml` as the supported browser path; `desktop-vwl` includes Chromium 143.0.7499.169, the assembly builds reproducibly, and the assembled-root smoke ran `chromium --version` plus `chromedriver --version`. |
| fish | personal | covered | none | `desktop-vwl` includes `fish`; system smoke ran `fish --version`. |
| foot | personal | covered | none | `desktop-vwl` includes `foot`; checkout smoke `foot --version`. |
| freerdp | personal | covered | 002e | `pkg/apps/misc/freerdp.yaml` builds FreeRDP 3.27.1 with the X11 client and internal MD4. `desktop-vwl` includes `bundles/full`; checked-out system smoke ran `xfreerdp /version`, verified `WITH_INTERNAL_MD4=ON`, resolved X11/OpenSSL/zlib libraries, and ran `winpr-hash`. |
| fuse-overlayfs | personal | covered | 002f | `pkg/apps/containers/fuse-overlayfs.yaml` builds 1.17 reproducibly; `desktop-vwl` includes it with `fuse3`; system smoke ran `fuse-overlayfs --version`, verified the manpage, and resolved the loader closure. |
| fuse3 | personal | covered | 002f | `desktop-vwl` includes `fuse3` full bundle for container and FUSE helpers; system smoke found `/usr/bin/fusermount3`, `/usr/bin/mount.fuse3`, `/etc/fuse.conf`, and `fuse-overlayfs` used `fusermount3`. |
| fzf | personal | covered | none | `desktop-vwl` includes `fzf`; checkout smoke `fzf --version`. |
| gdb | personal | covered | 002d | `pkg/dev/tools/gdb.yaml` builds reproducibly; `desktop-dev` includes it and a checked-out system smoke ran `gdb --version` plus `gdbserver --version`. |
| git | personal | covered | none | `desktop-vwl` includes `git`; checkout smoke `git --version`. |
| gnome-keyring | personal | covered | 002e | `pkg/apps/security/gnome-keyring.yaml` builds 50.0 with Secret Service, PKCS#11, PAM, and systemd user activation. `desktop-vwl` includes `bundles/full`; package and system smokes verified commands, service files, portal file, p11-kit module, PKCS#11 module, PAM module, schemas, locale, loader resolution, and daemon version. |
| gnome-themes-extra | personal | covered | 002e | `pkg/desktop/themes/gnome-themes-extra.yaml` builds 3.28; `desktop-vwl` includes `bundles/dev`. Package and system smokes verified Adwaita, Adwaita-dark, HighContrast, and 3,456 HighContrast PNG or SVG icon files. |
| gopls | personal | covered | 002d | `pkg/dev/tools/gopls.yaml` builds reproducibly; `desktop-dev` includes it and a checked-out system tree exposes `/usr/bin/gopls` as a Nex package symlink. |
| grim | personal | covered | none | `desktop-vwl` includes `grim`; checkout smoke `grim -h`. |
| grub | base | replaced | none | Nex boot path replaces GRUB; final boot validation proves deployable boot artifacts. |
| gvfs-smb | personal | covered | 002e | `desktop-vwl` includes `gvfs` 1.56.1 with SMB and SMB browse backends; system-root smoke verified public daemon symlinks, D-Bus services, systemd user units, mount descriptors, compiled schemas, and loader closure through `libsmbclient` and libsecret. |
| helix | personal | covered | none | `desktop-vwl` includes `helix`; checkout smoke `hx --version`. |
| htop | personal | covered | none | `desktop-vwl` includes `htop`; checkout smoke `htop --version`. |
| imagemagick | personal | covered | 002e | `pkg/apps/graphics/imagemagick.yaml` builds ImageMagick 7.1.2-26; `desktop-vwl` includes `bundles/full`, and a checked-out system root ran `magick -version`, converted a generated WebP image, and identified it. |
| inetutils | personal | covered | none | `desktop-vwl` includes `inetutils`; checkout smoke a shipped tool such as `hostname --version`. |
| jack-example-tools | personal | covered | 002e | `pkg/apps/multimedia/jack-example-tools.yaml` builds tag 4 with `alsa_in`, `alsa_out`, `jack_netsource`, `jack_rec`, Readline support, Opus support, and the core `jack_*` command set. `desktop-vwl` includes `jack-example-tools` and `jack2`; checked-out system root smokes verified public JACK command symlinks, `jackd --version`, `jack_transport` loader links, `jack_netsource` loader links, and `jack_simdtests`. |
| jq | personal | covered | none | `desktop-vwl` includes `jq`; system smoke ran `jq --version`. |
| kitty | personal | replaced | 002e | Replaced by the existing `foot` terminal in `desktop-vwl`; checked-out roots expose `/usr/bin/foot`, and `ncurses` already provides Kitty terminfo. |
| lazygit | personal | covered | none | `desktop-vwl` includes `lazygit`; checkout smoke `lazygit --version`. |
| less | personal | covered | none | `desktop-vwl` includes `less`; checkout smoke `less --version`. |
| lib32-vulkan-radeon | personal | deferred-policy | 002h | Deferred until Nex chooses a 32-bit graphics runtime and multilib output policy; this pairs with Steam support. |
| libnotify | personal | covered | none | `desktop-vwl` includes libnotify library and `notify-send`; checkout smoke `notify-send --version` or file presence. |
| libsecret | personal | covered | none | `desktop-vwl` includes `libsecret`; checkout verifies library files. |
| libvirt | personal | covered | none | `desktop-vwl` includes libvirt bins, config, libs, and daemon files; final check exercises `virsh --version`. |
| libvncserver | personal | covered | 002e | `pkg/libs/net/libvncserver.yaml` builds 0.9.15; `desktop-vwl` includes `outputs/lib`. Package and system smokes resolved `libvncserver.so.1` and `libvncclient.so.1` through zlib, JPEG, PNG, OpenSSL, glibc, and the loader. |
| libxcrypt-compat | personal | covered | 002d | `pkg/libs/system/libxcrypt-compat.yaml` builds reproducibly; `desktop-dev` smoke loaded `/usr/lib/libcrypt.so.1` with Python ctypes and called `crypt`, printing `xxj31ZMTZzkVA`. |
| linux | base | covered | none | `nex-systemd` and `desktop-vwl` include kernel bundles; boot check proves kernel path. |
| linux-firmware | base | covered | none | `desktop-vwl` includes desktop firmware bundle; checkout verifies firmware files. |
| linux-headers | base | covered | 002h | `desktop-vwl` includes `linux-headers`; checked-out root smokes passed for `/usr/include/linux/eventpoll.h`, `/usr/include/asm/unistd_64.h`, and `/usr/include/drm/drm.h`. |
| looking-glass | aur | covered | 003 | `pkg/apps/virt/looking-glass.yaml` builds the B7 client, `pkg/core/kernel/kvmfr.yaml` builds host shared-memory support, and both Nvidia desktop variants include them. Checked-out root smoke ran `looking-glass-client --help` as UID 1000 and saw `Looking Glass (B7)`. |
| loupe | personal | covered | 002e | `pkg/apps/graphics/loupe.yaml` builds Loupe 49.2 with vendored Cargo dependencies; `desktop-vwl` includes `bundles/full`, and package plus checked-out system-root smoke verified the binary, desktop file, D-Bus service, metainfo, compiled schema, locale, help page, loader closure, and `loupe --help` under `unshare --root`. |
| lldb | personal | covered | 002d | `pkg/dev/tools/lldb.yaml` builds reproducibly; `desktop-dev` includes it and a checked-out system smoke ran `lldb --version`, `lldb-server version`, and `lldb-dap --version`. |
| lsd | personal | covered | none | `desktop-vwl` includes `lsd`; checkout smoke `lsd --version`. |
| make | personal | covered | none | `desktop-dev` includes `make`; checkout smoke `make --version`. |
| mako | personal | covered | none | `desktop-vwl` includes `mako`; checkout smoke `mako --version`. |
| meld | personal | covered | 002d | `pkg/apps/misc/meld.yaml` builds reproducibly; `desktop-dev` includes it and a checked-out system smoke ran `meld --version` plus GTK/GtkSource imports from the package capsule. |
| meson | personal | covered | none | `desktop-dev` includes `meson`; checkout smoke `meson --version`. |
| mosh | personal | covered | none | `desktop-vwl` includes `mosh` and Perl; system smoke ran `mosh --version`. |
| mousepad | personal | replaced | 002e | Nex ships `gnome-text-editor`, Neovim, and Helix; `desktop-vwl` checkout exposes `gnome-text-editor`, `nvim`, and `hx`. |
| mpv | personal | covered | none | `desktop-vwl` includes `mpv`; checkout smoke `mpv --version`. |
| mkinitcpio | base | replaced | none | Nex initramfs package and boot path replace Arch mkinitcpio; final boot validation proves initramfs behavior. |
| nautilus | personal | covered | none | `desktop-vwl` includes `nautilus`; checkout smoke command presence. |
| ncdu | personal | covered | none | `desktop-vwl` includes `ncdu`; checkout smoke `ncdu --version`. |
| neovim | personal | covered | none | `desktop-vwl` includes `neovim`; checkout smoke `nvim --version`. |
| networkmanager | base | covered | none | `desktop-vwl` includes NetworkManager; final system check verifies service files and `nmcli`. |
| noto-fonts | personal | covered | none | `desktop-vwl` includes Noto fonts; checkout verifies font files. |
| noto-fonts-emoji | personal | covered | none | `desktop-vwl` includes emoji fonts; checkout verifies font files. |
| noto-fonts-extra | personal | covered | 002e | Covered by the existing `noto-fonts` universal package in `desktop-vwl`; checked-out system root verifies `GoNotoKurrent-Regular.ttf`, `GoNotoCurrentSerif.ttf`, and `GoNotoCJKCore.ttf`. |
| npm | personal | covered | none | Nex Node.js package provides npm; checkout smoke `npm --version`. |
| ntp | personal | replaced | 002h | Nex uses systemd-timesyncd; checked-out `desktop-vwl` ran `systemd-timesyncd --help`, and `80-systemd-timesync.list` names `systemd-timesyncd.service`. |
| nushell | personal | deferred-request | none | Human said to skip Nushell on 2026-06-28 after reproducibility attempts failed; they do not use it. |
| nvidia-580xx-dkms | aur | covered | 003 | `pkg/libs/graphics/nvidia-580.yaml` builds Nvidia 580.159.04 for Linux 6.12.58, and `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml` activates it with the desktop and capture stack. Checked-out root smoke verified module version, vermagic, userspace tools, and Nvidia GL/Vulkan vendor files. |
| nvidia-container-toolkit | aur | needs-manifest | 003 | Nvidia driver policy now exists, and the desktop variants activate one Nvidia branch at a time. Nex still needs a container runtime hook package and proof that the selected container runtime can see the Nvidia hook or equivalent. |
| openbsd-netcat | personal | covered | none | `desktop-vwl` includes OpenBSD netcat; system smoke ran `nc -h`. |
| openconnect-sso | aur | deferred-policy | 002h | Deferred until Nex chooses an AUR/Python desktop authentication-helper policy and confirms this helper is still used. |
| openssh | personal | covered | none | `nex-systemd` includes OpenSSH; checkout smoke `ssh -V`. |
| ostree | base | replaced | none | Nex uses zub store and deployment commands instead of OSTree. |
| pavucontrol | personal | covered | 002e | `pkg/apps/multimedia/pavucontrol.yaml` builds pavucontrol 6.2 reproducibly with GTKmm, PulseAudio, JSON-GLib, and libsndfile in the runtime closure. `desktop-vwl` includes `bundles/full`; checked-out system smoke verified the public command, desktop metadata, icons, capsule libraries, and expected headless GTK no-display failure. |
| pipewire-alsa | personal | covered | none | Nex PipeWire/WirePlumber stack should provide ALSA integration; final audio check verifies files or behavior. |
| pipewire-audio | personal | covered | none | Nex PipeWire/WirePlumber stack should provide audio service files; final audio check verifies runtime files. |
| pipewire-jack | personal | replaced | 002e | Nex PipeWire currently disables `-Djack` and `-Dpipewire-jack`, so `desktop-vwl` includes `jack2` as the classic JACK provider. The checked-out system root exposes `jackd` and JACK command tools and resolves JACK libraries from public `/usr/lib`. |
| pipewire-pulse | personal | covered | none | Nex PipeWire package should provide PulseAudio compatibility; final check verifies Pulse server path. |
| podman | base | covered | 002f | `desktop-vwl` includes Podman stack; 002f system smoke ran `podman --version` and `podman build --help` under `unshare --root` with `/proc` mounted. |
| polkit-gnome | personal | covered | 002e | `pkg/apps/security/polkit-gnome.yaml` builds 0.105 reproducibly as a GTK Polkit authentication agent. `desktop-vwl` includes `bundles/full`; package and system smokes verified the public libexec command, GTK/GDK/Polkit/Wayland/X11/xkbcommon runtime closure, and expected headless GTK no-display failure. |
| procs | personal | covered | none | `desktop-vwl` includes `procs`; checkout smoke `procs --version`. |
| python-pip | personal | covered | 002d | Existing Python dev bundle in `desktop-dev` includes pip; system smoke ran `python3 -m pip --version` and printed pip 24.0. |
| qemu-full | personal | covered | none | Nex uses `qemu` outputs; checkout smoke `qemu-system-x86_64 --version`. |
| qt5-wayland | personal | covered | 002e | Covered by `pkg/libs/graphics/qt6-wayland.yaml` for the current Nex Qt 6 stack. The strict package build passed reproducibly with checksum `885de55fc40689afcec7eebbd000501d7459c0ba01882431587c13ff02d67536`; package smoke verified `libqwayland-generic.so`, xdg shell and decoration plugins, `libQt6WaylandClient.so.6`, and loader closure under `unshare --root`. |
| qt6-5compat | personal | covered | 002e | Existing `qt6-5compat` output is included in `desktop-vwl`; system checkout resolved `libQt6Core5Compat.so.6` through the real glibc loader. |
| remmina | personal | covered | 002e | Covered by `pkg/apps/misc/remmina.yaml` with GTK3, RDP, VNC, libsecret, and exec plugin support, and included in `desktop-vwl`. Package build passed reproducibly with checksum `fd32ac43d2d1c9542acda179a1446c0e11328db8cbf22f0658117402e52d2715`; assembly build passed reproducibly with checksum `5f2eca04efd6a013e260cee05f7a1948ddb011c6e15648962620e96c4522603e`; smoke roots verified `remmina --version`, public desktop files, and plugin loader closures. |
| ripgrep | personal | covered | none | `desktop-vwl` includes `ripgrep`; checkout smoke `rg --version`. |
| rsync | personal | covered | none | `desktop-vwl` includes `rsync`; checkout smoke `rsync --version`. |
| screen | personal | covered | none | `desktop-vwl` includes `screen`; system smoke ran `screen --version`. |
| seatd | personal | covered | none | Nex uses `libseat`; verify seat helper/library files in checkout. |
| slirp4netns | personal | covered | 002f | `pkg/apps/containers/slirp4netns.yaml` builds 1.3.4 against local `libslirp` 4.9.3; `desktop-vwl` includes it; system smoke ran `slirp4netns --version`, verified the manpage, and resolved the loader closure. |
| slurp | personal | covered | none | `desktop-vwl` includes `slurp`; checkout smoke `slurp -h`. |
| sqlitebrowser | personal | covered | 002d | `pkg/apps/misc/sqlitebrowser.yaml` builds reproducibly; `desktop-dev` includes it and a checked-out system smoke ran `sqlitebrowser --version` with `QT_QPA_PLATFORM=offscreen` and the wrapper's Wayland fallback. |
| sshfs | personal | covered | 002f | `desktop-vwl` includes `sshfs`; system smoke ran `sshfs -V` and found `mount.sshfs` plus `mount.fuse.sshfs`. |
| starship | personal | covered | none | `desktop-vwl` includes `starship`; checkout smoke `starship --version`. |
| steam | personal | deferred-policy | 002h | Deferred until Nex chooses proprietary app and 32-bit runtime policies. Future proof must run Steam far enough to verify the runtime loader and 32-bit graphics stack. |
| strip-nondeterminism | personal | covered | 002d | `desktop-dev` includes `strip-nondeterminism`, `Archive::Zip`, and `Archive::Cpio`; system smoke normalized a ZIP timestamp from `1782649834` to `1704067200`. |
| sway | personal | replaced | 002e | Nex uses `vwl` compositor; `desktop-vwl` checkout exposes `/usr/bin/vwl`. |
| swaybg | personal | covered | none | `desktop-vwl` includes `swaybg`; checkout smoke command presence. |
| swayidle | personal | covered | none | `desktop-vwl` includes `swayidle`; checkout smoke command presence. |
| swaylock | personal | covered | none | `desktop-vwl` includes `swaylock`; checkout smoke command presence. |
| swaync | personal | replaced | 002e | Replaced by the existing `mako` notification daemon in `desktop-vwl`; checked-out system root exposes `/usr/bin/mako`, and `mako --help` prints notification daemon options. |
| taplo-cli | personal | covered | none | `desktop-dev` includes `taplo`; system smoke ran `taplo --version`. |
| tcpdump | personal | covered | 002f | `desktop-vwl` includes `tcpdump`; system smoke ran `tcpdump --version` and compiled a BPF filter with `tcpdump -ddd 'tcp port 22'`. |
| tig | personal | covered | none | `desktop-vwl` includes `tig`; checkout smoke `tig --version`. |
| tk | personal | covered | 002d | `pkg/dev/lang/tk.yaml` builds reproducibly; `desktop-dev` includes Tcl and Tk, and a checked-out system smoke verified `tclsh`, `wish`, and Tk package loading to the expected DISPLAY failure. |
| tmux | personal | covered | none | `desktop-vwl` includes `tmux`; checkout smoke `tmux -V`. |
| tokei | personal | covered | none | `desktop-dev` includes `tokei`; system smoke ran `tokei --version`. |
| tree | personal | covered | none | `desktop-vwl` includes `tree`; checkout smoke `tree --version`. |
| ttf-firacode-nerd | personal | covered | none | Nex ships Fira Code Nerd font under its own font manifest; checkout verifies font files. |
| ttf-hack-nerd | personal | covered | 002e | `pkg/fonts/hack-nerd.yaml` builds Hack Nerd 3.4.0; `desktop-vwl` includes it; checked-out system root verifies regular, mono, and proportional Hack Nerd font files. |
| ttf-jetbrains-mono-nerd | personal | covered | none | Nex ships JetBrains Mono Nerd font under its own font manifest; checkout verifies font files. |
| ungoogled-chromium-local | local package archive | deferred-request | none | Human said to skip the custom ungoogled Chromium package for now on 2026-06-28. |
| unzip | personal | covered | none | `desktop-vwl` includes `unzip`; checkout smoke `unzip -v`. |
| v4l2loopback-dkms | personal | covered | 003 | `pkg/core/kernel/v4l2loopback.yaml` builds v4l2loopback 0.12.7 against the Linux 6.12.58 module SDK, and both Nvidia desktop variants include the runtime bundle. Checked-out root smoke verified `v4l2loopback.ko` vermagic. |
| v4l2loopback-utils | personal | covered | 003 | `pkg/core/kernel/v4l2loopback.yaml` installs upstream `v4l2loopback-ctl`; package smoke ran its help command, and both Nvidia desktop variants include the same runtime bundle. |
| vimb | personal | replaced | 002g | Nex uses Chromium as the supported browser path. No WebKitGTK stack exists in `pkg/`, so Vimb would require packaging WebKitGTK first; the assembled-root browser smoke ran Chromium and Chromedriver version checks. |
| virt-manager | personal | covered | 002f | `desktop-vwl` includes `virt-manager`; system smoke ran `virt-manager --version`, `virt-install --version`, `virt-clone --version`, and `virt-xml --version`, found Fedora OS metadata, and imported the GTK, GTKSource, GTK-VNC, VTE, LibvirtGLib, Libosinfo, libvirt, libxml2, requests, virtinst, and virtManager Python stack from the assembled capsule. |
| vlc | personal | covered | 002g | `pkg/apps/multimedia/vlc.yaml` builds VLC 3.0.23 reproducibly without the nondeterministic `plugins.dat`; `desktop-vwl` includes it. Package and assembled-root smokes verified `vlc`, `cvlc`, `nvlc`, `rvlc`, `/usr/bin/sh`, the loader shim, 238 plugin shared objects, successful `vlc-cache-gen`, and VLC's expected root-user guard. |
| vulkan-radeon | personal | covered | none | Nex uses Mesa Vulkan ICD plus Vulkan loader; final graphics check verifies ICD JSON and loader files. |
| vulkan-tools | personal | covered | 002e | `desktop-vwl` includes `vulkan-tools`; checked-out system root ran `vulkaninfo --help` and `vkcube --help`, and loader checks resolved `vulkaninfo` plus `libvulkan.so.1`. |
| waybar | personal | replaced | 002e | Nex uses `ironbar`; `desktop-vwl` checkout exposes `/usr/bin/ironbar`. |
| wayland-protocols | personal | covered | none | `desktop-vwl` includes Wayland protocol data; checkout verifies files. |
| wget | personal | covered | none | `desktop-vwl` includes `wget`; system smoke ran `wget --version`. |
| which | base | covered | none | `desktop-vwl` includes `which`; checkout smoke `which --version`. |
| wireplumber | personal | covered | none | `desktop-vwl` includes `wireplumber`; checkout verifies service files and `wireplumber --version` if the command is shipped. |
| wl-clipboard | personal | covered | none | `desktop-vwl` includes `wl-clipboard`; checkout smoke `wl-copy --version` or command presence. |
| wl-mirror | personal | covered | none | `desktop-vwl` includes `wl-mirror`; checkout smoke `wl-mirror --version`. |
| wlr-randr | personal | covered | none | `desktop-vwl` includes `wlr-randr`; checkout smoke `wlr-randr --version`. |
| wlroots0.20 | personal | covered | none | Nex uses current `wlroots`; final compositor validation proves the dependent compositor path. |
| wlopm | aur | covered | 002h | `pkg/desktop/wayland/wlopm.yaml` builds wlopm 1.0.0 reproducibly; `desktop-vwl` includes it and the checked-out root ran `wlopm --version`. |
| wofi | personal | replaced | 002e | Nex uses `fuzzel`; `desktop-vwl` checkout exposes `/usr/bin/fuzzel`. |
| xdg-desktop-portal | personal | covered | 002e | `pkg/desktop/portals/xdg-desktop-portal.yaml` builds 1.21.2 with fuse3, GStreamer pbutils, PipeWire, DBus, systemd, json-glib, and bundled `libglnx` plus `gvdb`. `desktop-vwl` includes `bundles/full`; package and system smokes verified the public libexec helpers, D-Bus services, systemd user units, portal descriptor integration, interface XML, pkg-config metadata, locale file, version output, and real-loader checks. |
| xdg-desktop-portal-wlr | personal | covered | 002e | `pkg/desktop/wayland/xdg-desktop-portal-wlr.yaml` builds 0.8.2 with PipeWire, Wayland, inih, GBM, libdrm, and systemd. `desktop-vwl` includes `bundles/full`; package and system smokes verified the public libexec helper, D-Bus service, systemd user service, portal descriptor, help output, and Mesa `libgallium` runtime closure. |
| xfsprogs | base | covered | 002f | `desktop-vwl` includes `xfsprogs`; system smoke ran `mkfs.xfs -V`, `xfs_repair -V`, `xfs_db -V`, `xfs_info -V`, and `xfs_scrub_all -V`. |
| xorg-xauth | personal | covered | 002e | `pkg/apps/x11/xauth.yaml` builds `xauth` 1.1.5; `desktop-vwl` includes `xauth`; checked-out system root exposes `/usr/bin/xauth` and `xauth -V` prints `1.1.5`. |
| xorg-xeyes | personal | skipped | 002e | Skipped as an X.Org demo app rather than a daily-driver program. `rg --files pkg | rg 'libxaw|xeyes'` found no existing `xeyes` or Athena widget stack package; do not add that stack just for a demo row. |
| xorg-xhost | personal | covered | 002e | `pkg/apps/x11/xhost.yaml` builds `xhost` 1.0.10; `desktop-vwl` includes `xhost`; checked-out system root exposes `/usr/bin/xhost` and loader smoke resolves its X11 libraries. |
| xorg-xwayland | personal | covered | none | Nex uses `xwayland`; checkout smoke `Xwayland -version`. |
| zoxide | personal | covered | none | `desktop-vwl` includes `zoxide`; checkout smoke `zoxide --version`. |
| zsh | personal | covered | none | `desktop-vwl` includes `zsh`; checkout smoke `zsh --version`. |

## Next Work Slices

- 002c packages small daily CLI tools and adds them to the right assembly.
- 002d packages developer and debug tools that belong in `desktop-dev`.
- 002e packages desktop session helpers, GUI apps, fonts, portals, terminal
  choices, and X11 compatibility helpers.
- 002f packages container, filesystem, SSHFS, network tracing, and VM support
  gaps, then proves Podman and virtualization behavior.
- 002g handles large browsers and media apps, except the local ungoogled
  Chromium archive which is deferred by request.
- 002h records or implements policy-heavy work: proprietary apps, Nvidia,
  DKMS-like modules, multilib, microcode, and time sync.
- 002i performs final runtime validation against this matrix.
