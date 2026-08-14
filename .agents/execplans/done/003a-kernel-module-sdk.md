# Expose A Kernel Module SDK

This sub-ExecPlan belongs to
`.agents/execplans/003-nvidia-looking-glass-v4l2loopback.md`.

## Purpose / Big Picture

The kernel package must ship a build tree that external module packages can
use. Packages such as v4l2loopback, Nvidia, and Looking Glass host modules
must run `make -C <kernel-build-tree> M=<module-source> modules` against the
exact kernel that Nex ships.

## Progress

- [x] (2026-06-29 11:35Z) Ran the Ralph worktree pre-task. Only local
  untracked notes and settings were present, so no commit candidate existed.
- [x] (2026-06-29 11:38Z) Listed `.agents/knowledge/` and read kernel,
  package, system assembly, ostreefy parity, and reproducibility notes.
- [x] (2026-06-29 11:44Z) Taught output generation to place kernel SDK files
  in a `module-sdk` output. Commit:
  `2144ebe cli: categorize kernel module sdk outputs`.
- [x] (2026-06-29 12:42Z) Installed a narrowed kernel module SDK from
  `pkg/core/kernel/linux.yaml` with Linux's `install-extmod-build` helper.
- [x] (2026-06-29 12:45Z) Ran the strict kernel build with checksum update,
  output generation, dependency computation, profile recording, and
  reproducibility checking. It passed.
- [x] (2026-06-29 12:56Z) Checked out the `module-sdk` bundle and compiled a
  small external module against it.
- [x] (2026-06-29 13:01Z) Committed the checked kernel package change as
  `54468e6 pkg/kernel: expose module sdk`.

## Surprises & Discoveries

- Observation: `pkg/core/kernel/linux.yaml` keeps curated outputs such as
  `boot`, `modules-meta`, and `drv-gpu-amd`.
  Evidence: `.agents/knowledge/kernel-and-boot.md` and
  `pkg/core/kernel/linux.yaml`.

- Observation: The CLI preserves known output paths during regeneration, but
  paths not already listed use `determine_category`.
  Evidence: `src/cli/src/outputs/mod.rs` calls
  `categorize_files_with_existing_outputs`, and
  `src/cli/src/utils.rs` sends `/usr/src/...` paths to `misc`.

- Observation: `cargo test --manifest-path src/cli/Cargo.toml` passed after
  the categorizer change.
  Evidence: 69 unit tests plus 3 blob ref tests passed on 2026-06-29.

- Observation: Copying the full built kernel source into `/usr/src` made a
  huge SDK output and failed strict reproducibility.
  Evidence: the first broad-copy build generated roughly 140k `module-sdk`
  paths and produced different first and second checksums.

- Observation: Linux 6.12 provides `scripts/package/install-extmod-build` for
  external module build trees, and that narrower tree passed strict
  reproducibility in this package.
  Evidence: `./src/cli/target/debug/nex build pkg/core/kernel/linux.yaml
  --verbose --single --check --update-checksum --force --compute-deps
  --record-profile --generate-outputs` printed `Build is reproducible.
  Checksums match.`

- Observation: The generated SDK can compile a disposable external module.
  Evidence: `make -C
  .nex/tmp/kernel-sdk-smoke/sdk/usr/src/linux-6.12.58
  M=/home/wegel/work/wegelcorp/nex/.nex/tmp/kernel-sdk-smoke/module modules`
  produced `hello.ko`, and `modinfo -F vermagic` printed
  `6.12.58 SMP modversions`.

## Decision Log

- Decision: Add a `module-sdk` output category for kernel build trees.
  Rationale: External module packages should depend on one clear kernel SDK
  bundle instead of pulling runtime module bundles into their build roots.
  Date/Author: 2026-06-29 / Carlos

- Decision: Use Linux's `scripts/package/install-extmod-build` helper instead
  of copying the full built source tree.
  Rationale: The upstream helper copies the files Linux expects external
  module builds to use, removes transient build products, keeps the manifest
  output list tractable, and passed the strict two-pass build.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

The CLI now classifies kernel SDK files under `module-sdk`, and the kernel
manifest now ships a `module-sdk` output and bundle. A checked-out SDK bundle
builds a tiny out-of-tree module with a `6.12.58` vermagic string.

## Concrete Steps

1. Update `src/cli/src/utils.rs` so generated outputs classify
   `/usr/src/linux-*` and `/usr/lib/modules/<release>/build` or `source` as
   `module-sdk`.
2. Update `pkg/core/kernel/linux.yaml` so the build installs a prepared kernel
   build tree and `build` / `source` symlinks.
3. Add a `module-sdk` bundle to the kernel manifest.
4. Run:

```bash
cargo test --manifest-path src/cli/Cargo.toml
./src/cli/target/debug/nex check pkg/core/kernel/linux.yaml
./src/cli/target/debug/nex build pkg/core/kernel/linux.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

5. Check out the module SDK and compile a small external module against it.

## Validation and Acceptance

This sub-plan is complete when:

- `module-sdk` appears as a kernel output and bundle.
- A checked-out kernel `module-sdk` bundle contains `Module.symvers`, the
  kernel `.config`, build scripts, headers, and generated headers.
- A small external module compiles with the SDK and has a vermagic string that
  starts with `6.12.58`.
