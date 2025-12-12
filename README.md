# nex

Reproducible, source-bootstrappable Linux distribution.

## Why

I'm doing a (free) reproducible build system/distro (just a hobby, won't be big and professional like Guix or Nix) for Linux. This has been brewing for a few months, and is starting to get ready.

Nix and Guix use DSLs for build scripts. nex uses YAML manifests and bash. Build a reproducible GNU toolchain, then build everything else reproducibly from there. Any x64 gcc system can bootstrap the whole thing.

## Architecture

Two types of manifests:

- **Packages** (`pkg/`): Build individual software. Sources, dependencies, build script, outputs.
- **Assemblies** (`asm/`): Compose packages into bootable systems.

Outputs go into a [zub](https://github.com/wegel/zub) store (content-addressed filesystem).

## Git as truth, store as cache

The git repository of manifests is the source of truth. The zub store is purely a build cache.

Everything can always be rebuilt deterministically from manifests alone. No store? Rebuild. Corrupted store? Rebuild. Different machine? Same result.

The vision: official manifest repos (curated packages) + public build cache repos (pre-built binaries). Trust the manifests, verify the checksums. Anyone can run their own build cache, or rebuild from source.

## Pinning

Dependencies reference manifests by git blob sha:

```yaml
dependencies:
- name: zlib
  manifest_ref: a1b2c3d4...
  commit: x86_64/pkg/libs/system/zlib/1.3.1/bundles/dev
```

The builder fetches manifest content from `git cat-file`, not from disk. Builds are reproducible regardless of working tree state. Without `manifest_ref`, dependency is "floating" (disk lookup). Useful for development, rejected by CI.

`nex link <manifest>` pins all dependencies to current working tree versions.

## Flat vs nex-enabled rootfs

Two modes for generating systems:

**Flat rootfs**: Packages extracted directly to standard FHS paths (`/usr/bin`, `/usr/lib`). Simple. Good for embedded, containers, or systems not managed by nex.

**Nex-enabled rootfs**: Packages live in `/nex/pkg/{namespace}/{name}/{version}/{hash}/`. Symlinks provide FHS compatibility. Each package is a self-contained "capsule" with its dependencies flattened. Enables:

- Atomic upgrades and rollbacks
- Multiple versions coexisting
- System and user packages side-by-side
- No dependency conflicts

Scales from tiny embedded (~5MB busybox system) to full desktop/server.

## EFI bootloader

Custom UEFI bootloader (`src/bootloader/`).

**Design:**

- Bootloader loads ext4 driver, reads zub deployment structure directly
- Kernel has built-in drivers for 95% of hardware (NVMe, SATA, virtio, ext4)
- Constant init script built into kernel (busybox + shell, ~1-2MB)
- No generated initramfs, no BLS entries, no dracut/mkinitcpio
- Kernel lives in nex tree, not ESP partition

**Boot flow:**

```
EFI firmware
  → nex-bootloader (reads ext4, parses /nex/deploy/)
  → presents deployment menu
  → loads kernel + optional module cpio
  → kernel (built-in init)
  → mounts root, sets up deployment overlay
  → switch_root to real init (systemd, etc.)
```

**Edge case hardware (~5%):**

For exotic storage controllers not built into kernel, user creates `/etc/boot-modules.conf` listing required `.ko` files. Bootloader reads this from deployment, generates small CPIO archive in memory with just those modules, passes as additional initramfs. Kernel merges it with built-in init automatically.

**Deployment structure:**

```
/nex/deploy/{stateroot}/{checksum}.{serial}/
  boot/vmlinuz-*
  usr/...
```

Serial number enables rollback. Higher serial = more recent. Bootloader picks highest by default, or user selects.

**Status:** Bootloader implemented, reads ext4, finds deployments, boots kernel. CPIO module loading not yet implemented.

## System vs user environments

Three repository contexts:

```
.nex/repo              # local build-time (project-specific)
/nex/repo              # system-wide (root required)
/nex/users/$USER/      # user-specific (no root needed)
```

System packages: shared, root-managed, in `/nex/pkg/`.
User packages: per-user, in `/nex/users/$USER/pkg/`.

Same tools work everywhere. Users can build and install packages without root. System and user packages coexist cleanly.

## Replaces containers

A nex capsule is effectively a container without the container:

- Self-contained: all dependencies bundled
- Isolated: no system library conflicts
- Reproducible: content-addressed, deterministic builds
- Lightweight: no runtime overhead, no daemon

Run the binary directly. It finds its libs in its capsule directory via `nex-ld-shim`. No docker, no podman, no namespaces needed for isolation from dependency hell.

For actual sandboxing (network, filesystem), use namespaces directly or a thin wrapper. The package structure already solves the hard part.

## Build

```sh
cargo build --manifest-path src/cli/Cargo.toml
./src/cli/target/debug/nex build pkg/libs/system/zlib.yaml
```

## Status

- Bootstrap: complete (3-phase from any x64 gcc)
- Packages: ~160 manifests (core, libs, cli, dev, net, apps)
- Assemblies: bootable systemd system (~58MB)
- Bootloader: working EFI loader with deployment scanning

## Layout

```
pkg/                    # package manifests
  bootstrap/            # bootstrap toolchain (phase0-3)
  core/                 # core system (toolchain, userland, fs, init)
  libs/                 # libraries (system, compression, security)
  cli/                  # cli tools (shells, text, archive)
  dev/                  # development (lang, libs, tools)
  net/                  # networking
  apps/                 # applications (containers, etc.)
asm/                    # assembly manifests (bootable systems)
src/cli/                # nex build tool
src/bootloader/         # UEFI bootloader
```

## Commands

```
nex build <manifest>      build a package
nex check <manifest>      validate manifest
nex format <manifest>     auto-format manifest
nex compute-deps <m>      scan runtime dependencies
nex link <manifest>       pin deps to current versions
```

## Dependencies

- [zub-store](https://crates.io/crates/zub-store) - content-addressed filesystem store

## License

MIT
