#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

case "$repo_root" in
  */nex) ;;
  *)
    printf 'refusing to clean outside a nex checkout: %s\n' "$repo_root" >&2
    exit 1
    ;;
esac

shopt -s nullglob

paths=(
  tmp/src-inspect
  tmp/src-inspect.*
  tmp/vlc-3.0.21
  tmp/vlc-3.0.23
  tmp/xdg-desktop-portal-1.21.2
  tmp/gobject-introspection-*
  tmp/lldb-desktop-dev-smoke.*
  .nex/tmp/*-smoke.*
  .nex/tmp/ast-grep-output.*
  .nex/tmp/amd-ucode-initramfs-extract
  .nex/tmp/amd-ucode-initramfs-smoke
  .nex/tmp/intel-ucode-initramfs-extract
  .nex/tmp/intel-ucode-initramfs-smoke
  .nex/tmp/chromium-refs.txt
  .nex/tmp/chromium-smoke
  .nex/tmp/desktop-dev-*-smoke.*
  .nex/tmp/desktop-dev-002i-smoke
  .nex/tmp/desktop-vwl-freerdp-smoke
  .nex/tmp/desktop-vwl-gnome-keyring-smoke
  .nex/tmp/desktop-vwl-gvfs-smoke
  .nex/tmp/desktop-vwl-jack-smoke
  .nex/tmp/desktop-vwl-002f-smoke
  .nex/tmp/desktop-vwl-002g-smoke
  .nex/tmp/desktop-vwl-002h-smoke
  .nex/tmp/desktop-vwl-002i-smoke
  .nex/tmp/desktop-vwl-smoke
  .nex/tmp/desktop-vwl-loupe-smoke
  .nex/tmp/desktop-vwl-nvidia-580-smoke
  .nex/tmp/desktop-vwl-nvidia-current-smoke
  .nex/tmp/desktop-vwl-pavucontrol-smoke
  .nex/tmp/desktop-vwl-polkit-gnome-smoke
  .nex/tmp/desktop-vwl-qt6-wayland-smoke
  .nex/tmp/desktop-vwl-xdg-desktop-portal-smoke
  .nex/tmp/desktop-vwl-xdg-desktop-portal-wlr-smoke
  .nex/tmp/dbus-glib-smoke
  .nex/tmp/direct-initramfs-root
  .nex/tmp/duktape-smoke
  .nex/tmp/ep014-*-root
  .nex/tmp/ep016-src
  .nex/tmp/freerdp-inspect
  .nex/tmp/freerdp-smoke
  .nex/tmp/freerdp-store-check
  .nex/tmp/fuse-overlayfs-smoke
  .nex/tmp/fuse3-smoke
  .nex/tmp/gcr-smoke
  .nex/tmp/geocode-glib-smoke
  .nex/tmp/gnome-keyring-smoke
  .nex/tmp/gnome-desktop-store-check
  .nex/tmp/graphical-smoke
  .nex/tmp/gvfs-smoke
  .nex/tmp/gst-plugins-base-smoke
  .nex/tmp/gstreamer-smoke
  .nex/tmp/gtksourceview4-smoke.*
  .nex/tmp/gtk-vnc-smoke
  .nex/tmp/gtk-vnc-inspect
  .nex/tmp/gtk-vnc-store-check
  .nex/tmp/inih-smoke
  .nex/tmp/initramfs-inspect
  .nex/tmp/kernel-sdk-inspect
  .nex/tmp/kernel-sdk-smoke
  .nex/tmp/kvmfr-smoke
  .nex/tmp/jack-example-tools-inspect
  .nex/tmp/jack-example-tools-smoke
  .nex/tmp/jack2-inspect
  .nex/tmp/jack2-smoke
  .nex/tmp/libsamplerate-inspect
  .nex/tmp/libsamplerate-smoke
  .nex/tmp/libpcap-smoke
  .nex/tmp/libvirt-python-smoke
  .nex/tmp/lcms2-smoke
  .nex/tmp/libgcrypt-smoke
  .nex/tmp/libdbusmenu-store-check
  .nex/tmp/libgweather-smoke
  .nex/tmp/libsodium-smoke
  .nex/tmp/libslirp-smoke
  .nex/tmp/libsoup3-smoke
  .nex/tmp/looking-glass-inspect
  .nex/tmp/looking-glass-smoke
  .nex/tmp/loupe-smoke
  .nex/tmp/meson-smoke
  .nex/tmp/meld-store-check
  .nex/tmp/nvidia-*-inspect
  .nex/tmp/nvidia-*-smoke
  .nex/tmp/libvncserver-inspect
  .nex/tmp/pavucontrol-inspect
  .nex/tmp/pavucontrol-smoke
  .nex/tmp/parse-yapp-smoke
  .nex/tmp/polkit-gnome-smoke
  .nex/tmp/polkit-smoke
  .nex/tmp/polkit-smoke-package
  .nex/tmp/pygobject-smoke.*
  .nex/tmp/qt6-wayland-smoke
  .nex/tmp/remmina-smoke
  .nex/tmp/requests-smoke
  .nex/tmp/desktop-vwl-remmina-smoke
  .nex/tmp/distrobox-smoke
  .nex/tmp/samba-smoke
  .nex/tmp/spice-protocol-smoke
  .nex/tmp/sshfs-smoke
  .nex/tmp/slirp4netns-smoke
  .nex/tmp/tcpdump-smoke
  .nex/tmp/userspace-rcu-smoke
  .nex/tmp/build_rootfs_v4l2loopback_core_kernel
  .nex/tmp/build_rootfs_toolchain_bootstrap_phase0
  .nex/tmp/virt-manager-smoke
  .nex/tmp/v4l2loopback-inspect
  .nex/tmp/v4l2loopback-smoke
  .nex/tmp/vte-smoke
  .nex/tmp/vlc-repro-first
  .nex/tmp/vlc-repro-diff
  .nex/tmp/vlc-smoke
  .nex/tmp/vulkan-tools-smoke.*
  .nex/tmp/wlopm-smoke
  .nex/tmp/xfsprogs-smoke
  .nex/tmp/xdg-desktop-portal-smoke
  .nex/tmp/xdg-desktop-portal-wlr-smoke
  /tmp/ImageMagick-7.1.2-26.tar.gz
  /tmp/dbus-glib-0.112.tar.gz
  /tmp/duktape-2.7.0.tar.xz
  /tmp/freerdp-3.27.1.tar.gz
  /tmp/fuse3-smoke
  /tmp/fuse3-smoke.c
  /tmp/gtk-vnc-1.3.1.tar.xz
  /tmp/inih-smoke
  /tmp/inih-smoke.c
  /tmp/jack-example-tools-4.tar.gz
  /tmp/jack2-1.9.22.tar.gz
  /tmp/libsamplerate-0.2.2.tar.xz
  /tmp/libsamplerate-smoke
  /tmp/libsamplerate-smoke.c
  /tmp/libvncserver-0.9.15.tar.gz
  /tmp/LookingGlass-*.tar.gz
  /tmp/nex-vulkan-tools-raw
  /tmp/nex-vulkan-tools-inspect
  /tmp/nex-vulkan-tools-smoke.*
  /tmp/NVIDIA-Linux-x86_64-*-no-compat32.run
  /tmp/polkit-124.tar.gz
  /tmp/polkit-gnome-0.105.tar.gz
  /tmp/Parse-Yapp-1.21.tar.gz
  /tmp/spice-protocol-*.tar.*
  /tmp/vte-0.76.4.tar.xz
  /tmp/vte-0.82.0.tar.xz
  /tmp/vlc-3.0.21.tar.xz
  /tmp/vlc-3.0.23.tar.xz
  /tmp/virt-manager-5.1.0.tar.xz
  /tmp/v4l2loopback-*.tar.gz
  /tmp/xdg-desktop-portal-wlr-0.8.2.tar.gz
  /tmp/xdg-desktop-portal-1.21.2.tar.xz
)

if ((${#paths[@]} == 0)); then
  printf 'no disposable work directories matched\n'
  exit 0
fi

printf 'removing %d disposable work path(s)\n' "${#paths[@]}"
rm -rf -- "${paths[@]}"
