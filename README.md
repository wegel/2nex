# 2nex

## Full source-bootstrappable

I'm doing a (free) reproducible build system/distro (just a hobby, won't be big and professional like Guix or Nix) for Linux. This has been brewing for a few months, and is starting to get ready.

Jokes aside, I love the idea of an immutable, reproducible OS, but I find that I don't like how the build scripts are implemented in "strange" programming languages in Nix (the Nix Language) and Guix (Scheme, which is Turing-complete; not something you necessarily want for a build script language), and I don't like the complexity of the configuration languages they are using, although they are incredibly powerful (too much?). Note that I LOVE functional programming languages, but I don't think build scripts and configuration are where they're most needed.

I also don't understand *why* exactly Guix and Nix are searching for the Holy Full Source Bootstrap, because we already have it. Maybe *they* don't have it, but it's there. [Guix says](https://guix.gnu.org/en/blog/2023/the-full-source-bootstrap-building-from-source-all-the-way-down/):

> If you run guix pull today, you get a package graph of more than 22,000 nodes rooted in a 357-byte program; something that had never been achieved, to our knowledge, since the birth of Unix.

The 22,000 nodes is very impressive (and the >80,000 packages of Nix too), but the 357-byte root program is, to me, quite useless. They also need a ~25MB binary package to bootstrap the whole thing. Yes, the 357-byte program thing is VERY cool. But it's not *necessary*. Remember, the goal is to achieve reproducibility, and ultimately, you still have to build the "normal" GNU toolchain to be able to compile most software. They even say it in the "Full-Source Bootstrap" doc:

> hex0 (the 357-byte root program) builds hex1 and then on to M0, hex2, M1, mescc-tools and finally M2-Planet. Then, using mescc-tools, M2-Planet we build Mes. From here on starts the more traditional C-based bootstrap of the GNU System.

Why would we take this particular route to achieve reproducibility exactly? What we need, it seems to me, is a reproducible GNU toolchain, and once we have that, we *only* have to make sure that everything else that's built from there is also reproducible. Sounds simple enough to me, especially if the build system actually enforces reproducibility.

So I present 2nex. Currently, it's only a proof-of-concept sandbox to make a fully reproducible, fully source-bootstrappable, no fixed binary build system (eg, any x64 system with a gcc toolchain should work), but eventually I'd like to build on that to make a full distro for multiple architectures, and I also have some ideas for the configuration part of the OS (e.g., what's handled by the Nix language in NixOS).

## How it works

Packages are described in a yaml Manifest (see [pkg](pkg)). The Manifest lists the sources, dependencies (in the form of other packages), and build instructions. The Manifest includes the checksum of the result of the build; we thus can know that the result of the build is what we expect, and is reproducible. The [builder](src/builder) is used to build the Manifest, and the bundles are then commited to an ostree repository.

There are two types of manifests:

- **Package manifests**: Build a single piece of software (e.g., zlib, coreutils). They specify sources, build-time dependencies, and produce outputs that get committed to OSTree. The runtime dependency scanner automatically detects what each output actually needs.

- **Assembly manifests**: Assemble packages (including their transitive runtime dependencies) into a file structure. Can specify build-time only `dependencies` that are available during a configure-time build script to modify the resulting filesystem. The output could be a bootable system or just a configured rootfs.

Package manifests live under `pkg/`, while assembly manifests are in `asm/`.

## The builder

The builder is a rust program that reads the Manifest, builds and verifies it, and then commits the result to an ostree repository. It controls the build environment for reproducibility (sandboxing, timestamp clamping, etc).

## Manifest pinning

Every dependency listed in a manifest records a `manifest_ref`, which is simply the Git blob SHA of the dependency's manifest at the moment you explicitly promoted it. The builder never trusts whatever happens to be on disk: when a dependency has a `manifest_ref` it calls `git cat-file -p <sha>`, writes the blob into a cache, and resolves the dependency graph from that content. If the referenced outputs already exist in OSTree, the builder compares the cached manifest's hash to `nex.manifest.hash` on the OSTree commits and reuses them bit-for-bit. If they are missing or stale, it builds from the cached manifest content and publishes new commits before continuing.

When a dependency is missing `manifest_ref` it is considered “floating” and the builder will fall back to the working tree copy of the manifest (with a loud log message). This escape hatch is handy while iterating locally, but CI should reject such manifests because the final pin is what keeps rebuilds reproducible long after the local file has changed.

Use `nex link path/to/manifest.yaml` to update every dependency in that manifest to the current working tree versions. The subcommand hashes each dependency's manifest (`git hash-object -w …` ensures the blob is stored) and rewrites the `dependencies:` block with the new `manifest_ref` entries. This explicit promotion step lets you update libraries independently and only switch consumers over when you're ready.

Every package branch now lives under `x86_64/pkg/<namespace-path>/<slug>/<version>/{outputs,bundles}/…`, so the directory layout in `pkg/` matches the strings you see in manifests and OSTree metadata.

## Current Status

**Bootstrap toolchain**: Complete. Three-phase bootstrap from any x64 GCC system. The whole thing is 100% reproducible.

**Packages**: ~50 packages in categories (sys/libs, sys/apps, app/*, dev/*, net/*). Runtime dependency scanner auto-detects what each package needs.

**Assemblies**: Working. Minimal images (14MB) and more complex ones (podman). No bootstrap deps leak into production.

**Kernel + initramfs**: Built as reproducible packages. Kernel is EFI-enabled.

**Builder features**:
- Parallel graph-based builds with automatic dependency ordering
- Runtime dependency scanning (`--update-outputs-requires`)
- Dependency tracing (`--trace-dependency "bootstrap/phase1"`)
- Checksum verification
- Dependency minimizer script

## TODOs

**Next up**:
- Enforce the new pkg/ taxonomy everywhere (CI checks for misplaced manifests)
- Harden CI checks so floating `manifest_ref` entries are rejected

**Custom EFI boot manager**:
- Boot manager that scans OSTree commits and boots selected kernel
- Rollback support
- QEMU test harness
- OSTree-aware initramfs for root mounting

**Later**:
- Deployment tooling for real hardware
- User environments and composition
- More packages
