# Reproducibility

## Assembly Source Snapshots

Assemblies that package repo source snapshots can pick up dirty tracked files
from `pkg/`, `asm/`, `src/cli/`, and `scripts/`. Commit coherent tracked work
before using an assembly checksum as proof, or the checksum can describe local
source content that will not exist after the next commit.

Evidence: a local `desktop-vwl` build while unrelated `src/cli` and `scripts`
changes were dirty produced a checksum that was not safe to commit for the jq
assembly change.

When a CLI change affects assembly materialization, build the CLI first and use
that exact binary for the assembly proof. Commit the CLI fix separately from
the assembly checksum when the assembly manifest only records the result.

Evidence: during 002f, the capsule flattener copied only one Python leaf file
for `virt-manager` dependencies, which made the assembled root fail on missing
Python package siblings. After `cargo test` and `cargo build` passed for the
CLI flattener fix, `desktop-vwl` reproduced with checksum
`53afebab57780a66a79be94b9756fad395a286963c046ce2a038cf38b3f81f2d`; the CLI
fix landed as `cli: flatten python package runtime dirs`, and the assembly
checksum plus `virt-manager` ref landed separately as `asm: include virt
manager`.

## Audit Rebuilds

Treat manifests as read-only during an audit rebuild. An audit asks whether the
reviewed manifests already in Git produce the checksums they record. Do not pass
any option that can rewrite manifests, including `--update-checksum`,
`--record-profile`, `--generate-outputs`, or other `--update-*` flags.

Do not pass `--check` for an audit rebuild. The existing checksum in the
manifest is the check: a package or assembly either builds the recorded output
or fails. If a command needs a checksum-refresh mode to proceed, the run is no
longer auditing the committed manifest.

For a full cold audit, delete the disposable zub store, rebuild the target
assembly from the committed manifests without update flags, and let the first
checksum mismatch stop the run. Fix one real mismatch at a time, then restart
the audit from a clean store when the question is whether the full committed
graph still builds.

Evidence: On 2026-07-01, an attempted desktop cold rebuild used
`--check --update-checksum --record-profile`. That rewrote 451 YAML files and
made it impossible to claim that the whole assembly matched the previously
committed checksums. The human clarified that audit rebuilds must never use
`--update-*` flags and should not use `--check`, because the manifest checksum
already is the audit check.

When deliberately changing scripts, package inputs, or assembly contents, use
checksum-refresh builds for the changed manifests and keep those builds
separate from audit claims. A successful `--check --update-checksum` run proves
the new output reproduced within that command; it does not prove an old Git
manifest still matched its previous checksum.

Evidence: EP009 intentionally changed `pkg/core/kernel/initramfs-init.sh`,
`scripts/nex-install`, the Nex CLI, and zub. The refresh builds reproduced and
recorded these new checksums: `pkg/core/kernel/initramfs.yaml`
`c216a714681fc7ddc0231b73dae59ca5a513837e733e38f1bc6ac9c1bcaed7e1`,
`pkg/core/kernel/linux.yaml`
`dc2995d71ac81a53479483d95653d4661424a62f7d02a9f1bdd90547932e22b9`,
`examples/desktop-vwl/desktop-vwl.yaml`
`7b2c4dc656274d3d1501e00ae3193a12b45a4df31cd9a8d52a6bbcb7f6de83d2`,
`asm/desktop-vwl-nvidia-580.yaml`
`3b710c76449a913b982b34e85ca186aa1f9e602b0a5f7d3dd915e233107c9194`, and
`installer/installer.yaml`
`7aaa274c8963d618ed1e7739628043d4e85247e0d7384c5baec51c0b190cc246`.

When a manifest checksum mismatch blocks an ordinary package repair, isolate
the package with the strict two-pass command before changing the checksum. If
the first and second outputs match, the old manifest checksum was stale rather
than proof of same-command nondeterminism.

Evidence: EP007 repaired stale checksums for `pkg/core/kernel/kvmfr.yaml`,
`pkg/libs/graphics/nvidia-580.yaml`, and
`pkg/core/kernel/v4l2loopback.yaml`. Each strict two-pass build produced
matching first and second outputs before the manifest checksum changed.

## Rust Package Link Entropy

For Rust packages that build native C or C++ archives through Cargo build
scripts, inherited `-flto=auto` can make the final Rust link fail to resolve
symbols from those archives. Strip that flag from `CFLAGS`, `CXXFLAGS`, and
`LDFLAGS` in the package script before invoking Cargo or the build system that
invokes Cargo.

Evidence: `ast-grep` failed with unresolved `tree_sitter_*` and `ts_*`
symbols until the manifest stripped `-flto=auto`. Fish 4.7.1 showed the same
pattern with vendored PCRE2 UTF-32 symbols.

## Rust Reproducibility Controls

When a Rust package produces different ELF bytes in a two-pass strict build,
try the common controls first, then compare the two binaries before assuming
the package can be fixed quickly:

```bash
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS=1
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_LTO=false
export SOURCE_DATE_EPOCH=1704067200
```

These controls do not guarantee reproducibility for every package.

Evidence: Nushell 0.111.0 still failed the strict two-pass build after
disabling release LTO and setting those environment variables. The two output
ELF files had identical section headers and same-size sections, while
`readelf -r -W` showed many relocation addends differing by one byte. The
human chose to skip Nushell for the current parity work.

## Generated Runtime Caches

Generated runtime caches can make package output checksums differ even when
all installed program files match. Compare raw build outputs by path, not by
hash order, and remove or regenerate caches in a deterministic way.

Evidence: VLC 3.0.23 produced identical file paths across two strict build
passes, but `/usr/lib/vlc/plugins/plugins.dat` had different bytes. Deleting
that generated cache after install made the strict build pass with checksum
`f0721b8de57797bcf6d7a04cbba5797d9fc034a24bc7f9744d4ccbd059c4a3c2`; smokes
proved `/usr/lib/vlc/vlc-cache-gen` could regenerate the cache from the
installed plugin shared objects.

## Output Tree Checksums

Package output checksums cover tree shape, not just regular file bytes. They
include directories, regular file modes and bytes, symlink modes, and symlink
targets. Unsupported special files such as FIFOs are rejected with
`InvalidData` so a build cannot change the output tree without changing or
failing the checksum.

Evidence: EP005 added symlink and FIFO checksum tests, and the strict
`nex-ld-shim` two-pass build still printed `Build is reproducible. Checksums
match.` after the checksum code covered the full tree shape.

## Kernel Module SDKs

For Linux 6.12, prefer the upstream `scripts/package/install-extmod-build`
helper when a kernel package needs an external module SDK. A full copy of the
built kernel source tree made the generated output list huge and failed strict
reproducibility, while `install-extmod-build` produced a smaller SDK tree that
passed the two-pass strict build.

Evidence: the broad source copy under `/usr/src/linux-6.12.58` generated
roughly 140k `module-sdk` paths and produced different first and second
checksums. After `pkg/core/kernel/linux.yaml` used `install-extmod-build`, this
command completed with `Build is reproducible. Checksums match.`:

```bash
./src/cli/target/debug/nex build pkg/core/kernel/linux.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

## Standard Build Environment

`env/standard.yaml` runs its preamble in a user namespace. Package and
assembly build roots must contain independently writable copies of Zub blobs.
Do not switch their materializer paths back to hardlinks: a write through a
hardlink changes the content-addressed store object itself. Nex reserves
hardlinks for finalized deployments that it mounts read-only.

Zub commit `0db2a06aa0b741eb537ef41e5864226586aced38` makes copies
the library and command defaults, adds an explicit `--hardlink` opt-in, and
repairs an existing blob when a later write finds mismatching bytes or stored
metadata at its hash path. Nex pins that revision and tests that changing a
normal checkout leaves a second checkout unchanged.

The standard preamble must not try to replace an existing
`{build_dir}/etc/ld.so.cache`. If the cache already exists, leave it in place
and continue. If the cache is absent, create `{build_dir}/etc` and run
`ldconfig -r {build_dir}`.

Evidence: before writable-copy materialization, v4l2loopback builds failed
when the preamble tried to create or replace `/etc/ld.so.cache` from a Glibc
hardlink. The guarded preamble allowed the strict v4l2loopback build to pass;
the later writable-copy fix removed the underlying store-sharing hazard.

## Nvidia Runfiles

Nvidia Linux runfiles are makeself archives with a zstd-compressed payload.
For reproducible Nex package builds, extract the payload directly instead of
running the wrapper. Use the `skip=` header value, then pipe the payload through
`tail`, `zstd -d`, and `tar --no-same-owner`.

Evidence: the runfile wrapper calls `cksum`, but the standard build root did
not include `/usr/bin/cksum`. The wrapper also uses plain `tar xvf -`, which
can fail when tar applies archive metadata inside the build namespace. Direct
extraction let both Nvidia 580.159.04 and Nvidia 595.84 pass strict two-pass
builds.

Do not pass `KERNELRELEASE` to Nvidia's top-level module Makefile. Pass
`SYSSRC=/usr/src/linux-6.12.58` and let Nvidia's Makefile call Kbuild.

Evidence: when the manifest passed `KERNELRELEASE=6.12.58`, Nvidia included
`/Kbuild` and failed with `Makefile:18: /Kbuild: No such file or directory`.
Removing `KERNELRELEASE` let both branches build modules reproducibly.

## Runtime Smoke Roots

Do not use `nex install --dry-run --flat` as a package smoke helper. It can
still materialize into `/`, which can replace host runtime libraries inside the
sandbox. Use a disposable root and explicit `zub checkout` calls until the CLI
proves dry-run does not write.

Evidence: during the Looking Glass smoke, `nex install --dry-run --flat
pkg/apps/virt/looking-glass.yaml bundles/runtime` printed `Checking out to
/...` and checked out runtime closure files. The sandbox then needed a restart
because Bash tried to use the Nex glibc 2.39 file while host ncurses required
`GLIBC_2.42`.

When smoking a manually layered Nex binary root, do not prefix the shell
command with `LD_LIBRARY_PATH=<root>/usr/lib`; that can make the command
runner's shell load the temp root's glibc. Invoke the temp root's dynamic
loader directly:

```bash
<root>/usr/lib/ld-linux-x86-64.so.2 --library-path <root>/usr/lib <root>/usr/bin/program --help
```

Evidence: Looking Glass B7 help smoke worked through
`.nex/tmp/looking-glass-smoke/usr/lib/ld-linux-x86-64.so.2 --library-path
.nex/tmp/looking-glass-smoke/usr/lib
.nex/tmp/looking-glass-smoke/usr/bin/looking-glass-client --help`.

Some programs refuse to run as root before printing help. Use `setpriv` with a
writable temp home when a non-hardware smoke only needs to prove the command
starts and prints help.

Evidence: Looking Glass B7 printed `Do not run looking glass as root!` as
root, then printed `Looking Glass (B7)` and its option tables as UID 65534
when `HOME`, `XDG_CONFIG_HOME`, and `XDG_DATA_HOME` pointed at a writable temp
tree.

## Boot Artifact Order

When Linux needs several initramfs-like boot artifacts, a root file existence
check does not prove the boot behavior. The check must prove the byte order
that the kernel will see.

For x86 early microcode, the direct QEMU helper concatenates
`/boot/amd-ucode.cpio` and `/boot/intel-ucode.cpio` before the base initramfs.
It uses `cmp -n` with offsets to prove each cpio sits at the expected byte
range before booting QEMU.

Evidence: ExecPlan 004's direct QEMU command for
`systems/desktop-vwl-nvidia-580/0.0.1` logged the microcode cpio path, passed
the prefix comparison, booted QEMU, and printed `ASSERT-BOOT-PASS`.

Evidence: after adding Intel microcode, the direct QEMU command for
`systems/desktop-vwl-nvidia-580/0.0.1` logged both AMD and Intel microcode cpio
paths, passed both offset comparisons, booted QEMU, and printed
`ASSERT-BOOT-PASS`. The combined initramfs size matched the sum of the AMD
cpio, Intel cpio, and base initramfs.
