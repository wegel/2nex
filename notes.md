# Notes

## Workflow issues to fix 

Todo.

## Commands Reference

```bash
# re-run compute-deps on a package
./src/builder/target/release/nex compute-deps --repo .nex/repo pkg/path/to/manifest.yaml 

# rebuild a single package (after compute-deps)
./src/builder/target/release/nex build pkg/path/to/manifest.yaml --force --single

# rebuild assembly with checksum update
./src/builder/target/release/nex build asm/bootable-systemd-nex.yaml --update-checksum

# regenerate outputs section from build
./src/builder/target/release/nex build pkg/path/to/manifest.yaml --generate-outputs --compute-deps --force --single
```

## Bootstrap Build Requirements

Bootstrap packages (phases 0-2) must all be built on the same host and in the same path because they have the absolute path of the toolchain hardcoded in them.

For example, you cannot do phase0 on one host and phase1 on another host. Phases 0-2 must be done on a single host with a consistent build path.

Phase3 packages don't have this restriction and can build in parallel on different hosts.

## Testing System Assemblies

After building a system assembly, test binaries without booting a VM using unshare + chroot:

```bash
# build the system
./src/builder/target/debug/nex build --repo .nex/repo asm/bootable-systemd-nex.yaml

# checkout to a temp directory
rm -rf tmp/asm-test
unshare --map-root-user ostree --repo=.nex/repo checkout systems/bootable-systemd-nex/0.0.1 tmp/asm-test

# test binaries in the chroot
unshare --map-root-user --root=tmp/asm-test /usr/bin/init --help
unshare --map-root-user --root=tmp/asm-test /usr/bin/mount --help
unshare --map-root-user --root=tmp/asm-test /usr/bin/bash -c 'echo hello'
```

If a binary fails with "cannot open shared object file", the dependency flattening is missing that library.

## System Assembly: dependencies vs packages

In system assembly manifests (`asm/*.yaml`), there are two distinct sections:

### `dependencies` (build-time only)

Libraries and tools needed during the system assembly build script, but NOT included in the final rootfs. These are used to compile, link, or configure things during build.

Example:
```yaml
dependencies:
- name: glibc
  commit: x86_64/pkg/libs/system/glibc/2.39/bundles/dev
- name: util_linux-lib
  commit: x86_64/pkg/core/userland/util_linux/2.40/outputs/lib
```

### `packages` (runtime, final rootfs)

Packages that end up in the final bootable system. These are what users interact with at runtime. The builder automatically resolves and flattens each package's runtime dependencies (via `needs`/`resolution` in manifests) into self-contained capsules.

Example:
```yaml
packages:
- commit: x86_64/pkg/core/init/systemd/257.5/outputs/bin
  name: systemd
- commit: x86_64/pkg/cli/shells/bash/5.2.21/outputs/bin
  name: bash
```

Key difference: `dependencies` are layered into the build environment but discarded. `packages` are installed into `/nex/pkg/` structure and survive into the final system.

