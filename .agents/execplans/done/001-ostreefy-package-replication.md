# Commit Current Package Work, Then Wire OSTreefy Desktop Packages

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must maintain this file according to `.agents/PLANS.md`.

## Purpose / Big Picture

Ralph will first test and commit the package work that already exists in the
current worktree. That package work touches the kernel and initramfs package
path, so Ralph must prove those package manifests still check and build before
starting new assembly edits.

After that package commit, Ralph will make the `desktop-vwl` and `desktop-dev`
assemblies include the non-browser packages that the old OSTreefy personal
image used and Nex already has as package manifests. After this change, a user
can build the desktop assembly and find VM tools, Vulkan runtime pieces, X11
compatibility tools, Wayland protocol data, libsecret support, and Node.js in
the built root filesystem.

This plan does not add new package manifests. Ralph must use existing manifests
under `pkg/` and must build the changed assemblies to prove that the new package
refs work inside a Nex system image. A Nex assembly manifest is a YAML file
under `asm/` that names the built packages that go into one root filesystem.

## Progress

- [x] (2026-06-27 20:09Z) Read `AGENTS.md`,
  `.agents/RUST_CODE_STYLE.md`, `.agents/MANIFESTS_CODE_STYLE.md`,
  `.agents/PLANS.md`, `.agents/ralph-prompt.md`, and the small instruction
  files under `.agents/`.
- [x] (2026-06-27 20:09Z) Read `OSTREEFY_REPLICATION_REPORT.md` and confirmed
  that it recommends an assembly patch before new manifests.
- [x] (2026-06-27 20:09Z) Read `CURRENT_TODO.md` and confirmed that Chromium
  still has a GTK build blocker.
- [x] (2026-06-27 20:09Z) Updated this ExecPlan after the human clarified that
  work must start by testing and committing current uncommitted package files.
- [x] (2026-06-28 00:00Z) Built the local debug CLI so package and assembly
  checks can run through `./src/cli/target/debug/nex`.
- [x] (2026-06-28 00:00Z) Added focused CLI fixes required by the current
  package and installer checks: formatter support for system `overlays`, boot
  output categorization for `/boot`, and chroot usr-merge symlink refresh.
- [x] (2026-06-28 01:54Z) Identified the current uncommitted package files and
  kept them separate from unrelated dirty files.
  assembly, CLI, script, and note changes.
- [x] (2026-06-28 01:54Z) Added an automated QEMU regression test path for the
  changed initramfs deployment and writable-state mounts.
- [x] (2026-06-28 01:54Z) Tested the current uncommitted kernel package
  manifests with strict package builds.
- [x] (2026-06-28 02:16Z) Rebuilt the initramfs package after remounting the
  `/sysroot` bind read-only; the strict package build reported reproducible
  output and updated the checksum to
  `f349440de0df84d7c8f06c233051555834c18365364bf853fa94f5a83ef127aa`.
- [x] (2026-06-28 02:16Z) Booted the rebuilt kernel and initramfs with the
  direct QEMU assertion path; the guest printed `ASSERT-BOOT-PASS`.
- [x] (2026-06-28 02:22Z) Extracted durable package-manifest notes from
  `.claude/agent_docs/working-on-package-manifests.md` into
  `.agents/knowledge/package-manifests.md`.
- [x] (2026-06-28 02:22Z) Ran the pre-commit checks for the package commit:
  shell syntax checks for `pkg/core/kernel/initramfs-init.sh` and
  `scripts/qemu-test-installer.sh`, targeted `rustfmt --check`, package and
  installer manifest checks, `git diff --check`, and four focused CLI tests.
- [x] (2026-06-28 02:24Z) Committed the current package work as
  `014bb2f pkg/kernel: fix deployment boot mounts`.
- [x] (2026-06-28 02:34Z) Fixed and committed the manifest formatter after it
  removed `system.extends`; commit
  `297fd29 cli: preserve assembly inheritance when formatting`.
- [x] (2026-06-28 02:42Z) Fixed and committed the manifest formatter after
  manifest checks rejected required package group comments; commit
  `4af1199 cli: preserve manifest group comments`.
- [x] (2026-06-28 02:34Z) Formatted `asm/nex-systemd.yaml`,
  `asm/desktop-vwl/desktop-vwl.yaml`, and `asm/desktop-dev.yaml` with the
  rebuilt CLI.
- [x] (2026-06-28 02:42Z) Checked `asm/nex-systemd.yaml`,
  `asm/desktop-vwl/desktop-vwl.yaml`, and `asm/desktop-dev.yaml`.
- [x] (2026-06-28 03:18Z) Built the existing package refs needed by
  `asm/desktop-vwl/desktop-vwl.yaml`; `libtirpc`, `json-c`, `libnl`, `qemu`,
  and `libvirt` completed their strict package checks during the assembly
  build.
- [x] (2026-06-28 03:18Z) Found and fixed a self-referential system checksum
  problem in the self-hosting source snapshot: the assembly copied `asm/` into
  the built root filesystem, so changing `system.checksum` between the first
  and second build changed the output.
- [x] (2026-06-28 03:48Z) Fixed the CLI system reproducibility check for
  `nex_structure` systems after it materialized explicit package refs in the
  first pass and expanded dependency refs in the second pass.
- [x] (2026-06-28 03:55Z) Added existing non-browser OSTreefy package refs to
  the right desktop
  assembly.
- [x] (2026-06-28 03:55Z) Built the changed assembly or assemblies and
  inspected the built root
  filesystem for the newly added packages.
- [x] (2026-06-28 03:55Z) Record commands, results, and any package blockers
  in this ExecPlan.
- [x] (2026-06-28 04:06Z) Committed the checked CLI fix as
  `d757d0b cli: fix system reproducibility check`.
- [x] (2026-06-28 04:08Z) Committed the checked assembly work as
  `a4670a4 asm: add ostreefy desktop packages`.

## Surprises & Discoveries

- Observation: The debug CLI binary did not exist when this plan was created.
  Evidence: `./src/cli/target/debug/nex --help` failed with `No such file or
  directory`, while the `./nex` wrapper existed and builds the CLI before it
  runs it.
- Observation: Chromium should not enter this first assembly patch yet.
  Evidence: `CURRENT_TODO.md` says `pkg/libs/graphics/gtk3.yaml` fails while
  Chromium work tries to build GTK3, so the browser path needs a separate
  package-fix plan before a desktop assembly should depend on it.
- Observation: `asm/desktop-dev.yaml` already includes `gn`.
  Evidence: The manifest has a `gn` package entry under the Chromium tooling
  comment.
- Observation: The current uncommitted package files are in the kernel package
  path.
  Evidence: `git status --short -- pkg asm .agents CURRENT_TODO.md chromium-manifests-todo.md OSTREEFY_REPLICATION_REPORT.md`
  showed modified `pkg/core/kernel/initramfs-init.sh`,
  `pkg/core/kernel/initramfs.yaml`, and `pkg/core/kernel/linux.yaml`.
- Observation: Ralph finds work from numbered files under
  `.agents/execplans/`.
  Evidence: `AGENTS.md` and `.agents/ralph-prompt.md` tell Ralph to select the
  lowest numbered active file.
- Observation: `.agents/knowledge/` currently has no durable note files.
  Evidence: `find .agents/knowledge -maxdepth 1 -type f -printf '%f\n' | sort`
  printed no files on 2026-06-28.
- Observation: The strict initramfs build passed after the init script change.
  Evidence: `./src/cli/target/debug/nex build pkg/core/kernel/initramfs.yaml
  --verbose --single --check --update-checksum --force --compute-deps
  --record-profile --generate-outputs` completed and updated the initramfs
  checksum to `1cfde003a3770040273c743d2b0a82ecb530bbefe722dc3fe8bd58bac284d268`.
- Observation: The strict kernel build exposed a rootfs write in the manifest,
  not a kernel compile error.
  Evidence: The build reached `modules_install`, then failed at `mkdir -p
  /usr/lib/modules` with `Permission denied`.
- Observation: `depmod -m /usr/lib/modules` is not safe for the kmod available
  in the kernel build root.
  Evidence: A retry with `/tmp/depmod -b ${OUT_DIR} -m /usr/lib/modules
  6.12.58` failed with `depmod: ERROR: Bad version passed /usr/lib/modules`.
- Observation: An earlier fixed strict kernel build retry was interrupted before
  it could prove the change.
  Evidence: The manifest now uses `/tmp/depmod -b ${OUT_DIR}/usr 6.12.58`
  without `mkdir -p /usr/lib/modules`, but the human interrupted the retry
  while it was still compiling.
- Observation: The fixed strict kernel build passed and kept curated output
  names.
  Evidence: `./src/cli/target/debug/nex build pkg/core/kernel/linux.yaml
  --verbose --single --check --update-checksum --force --compute-deps
  --record-profile --generate-outputs` completed on 2026-06-28, set checksum
  `4fc28063accfef32254806f6f4ea2daac0a8014d20b316752033d948731d4c38`, printed
  `Build is reproducible. Checksums match.`, and left outputs such as `boot`,
  `modules-meta`, `drv-eth-intel`, and `drv-gpu-amd` in the manifest.
- Observation: The kernel build can print missing checksum-tool warnings while
  it still succeeds.
  Evidence: The second strict build pass printed warnings from
  `kernel/gen_kheaders.sh` about missing `sha1sum` and `md5sum`, then completed
  and reported reproducible output.
- Observation: The full installer QEMU path currently needs host tools that are
  not present on this machine.
  Evidence: `./scripts/qemu-test-installer.sh --rebuild --autoinstall
  --headless --extra-nex-var --assert-boot` reached installer image creation
  but then needed `parted`, `mkfs.vfat`, `mcopy`, and `mmd`.
- Observation: The direct initramfs QEMU path can test the early boot mount
  contract without installer media tooling.
  Evidence: `./scripts/qemu-test-installer.sh --direct-initramfs --assert-boot
  --headless --extra-nex-var --timeout 180` created an installed disk from store
  refs, booted QEMU directly, and printed `ASSERT-BOOT-PASS`.
- Observation: Bind mounts under the selected deployment disappear after
  `switch_root` unless the deployment root is a mount point first.
  Evidence: The direct QEMU assertion could not see `/sysroot` until the init
  script ran `mount --bind "$DEPLOY" "$DEPLOY"` before binding child paths.
- Observation: A read-only source sysroot does not make the `/sysroot` bind
  read-only by itself.
  Evidence: The direct QEMU assertion failed with `ASSERT-FAIL: /sysroot is not
  read-only: rw,relatime` until the init script ran
  `mount -o remount,ro,bind "${DEPLOY}/sysroot"`.
- Observation: The old Claude package-manifest note had useful defaults, but it
  needed one caveat from current package work.
  Evidence: `.claude/agent_docs/working-on-package-manifests.md` says outputs
  should be generated and kept blank. The kernel package requires curated
  output names, so `.agents/knowledge/package-manifests.md` now says normal
  manifests should use generated outputs while curated names must be preserved.
- Observation: The manifest formatter used by the local CLI could remove
  assembly inheritance.
  Evidence: `./src/cli/target/debug/nex format asm/nex-systemd.yaml
  asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml` removed
  `system.extends` from `desktop-vwl` and `desktop-dev` before the CLI was
  rebuilt with the formatter fix.
- Observation: The manifest formatter also needed explicit comment
  preservation for grouped assembly entries.
  Evidence: After group comments were restored, `nex check` reported `needs
  formatting` for all three assembly manifests until the formatter reinserted
  comments attached to `sources`, `dependencies`, and `packages` entries.
- Observation: A system assembly that ships a copy of `asm/` can make its own
  checksum self-referential.
  Evidence: `./src/cli/target/debug/nex build
  asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum
  --force` built all package refs, stored the first system commit at
  `systems/desktop-vwl/0.0.1`, updated the manifest checksum to
  `973c9358e52ad22e6f4eb65a64f1650c76cc189a100adc9a715113587ad38c3a`, then
  rebuilt with the changed manifest in the copied `asm` source snapshot and
  failed reproducibility with second checksum
  `6af0e9da98970c4922c46e5c357dcc78d2d7ad900d9164666abace35a2ac5f21`.
- Observation: The CLI used different package ref lists for the two passes of
  `nex_structure` system reproducibility checks.
  Evidence: Before the fix, the first `desktop-vwl` pass logged
  `Materialized 108 packages`, the second pass logged `Materialized 206
  packages`, and the build failed with first checksum
  `3519a6080af1ec59f04c7ea4303035cb067f09c3830d9e3767e3f8bb89e3de15` and
  second checksum
  `c11b1c262b6ff7c47b82b209ae807bd24a2700fdfb3023b8c74beee579516519`.
  After `src/cli/src/system/mod.rs` used the original package refs for the
  second pass, both passes logged the same package count and checksums matched.
- Observation: This sandbox does not have `ostree`.
  Evidence: `which ostree` found no executable, and the CLI integration test
  `test_time_travel_history_search` failed immediately with `No such file or
  directory`.

## Check Log

- `./src/cli/target/debug/nex build pkg/core/kernel/linux.yaml --verbose
  --single --check --update-checksum --force --compute-deps --record-profile
  --generate-outputs`: passed on 2026-06-28; checksum became
  `4fc28063accfef32254806f6f4ea2daac0a8014d20b316752033d948731d4c38`.
- `./src/cli/target/debug/nex build pkg/core/kernel/initramfs.yaml --verbose
  --single --check --update-checksum --force --compute-deps --record-profile
  --generate-outputs`: passed on 2026-06-28; checksum became
  `f349440de0df84d7c8f06c233051555834c18365364bf853fa94f5a83ef127aa`.
- `./scripts/qemu-test-installer.sh --direct-initramfs --assert-boot
  --headless --extra-nex-var --timeout 180`: passed on 2026-06-28; the guest
  printed `ASSERT-BOOT-PASS`.
- `sh -n pkg/core/kernel/initramfs-init.sh`: passed.
- `sh -n scripts/qemu-test-installer.sh`: passed.
- `rustfmt --check --edition 2021 src/cli/src/build/mod.rs
  src/cli/src/manifest/format.rs src/cli/src/utils.rs
  src/cli/src/outputs/mod.rs`: passed.
- `./src/cli/target/debug/nex check pkg/core/kernel/initramfs.yaml
  pkg/core/kernel/linux.yaml`: passed.
- `./src/cli/target/debug/nex check asm/installer/installer.yaml`: passed.
- `git diff --check -- src/cli/src/build/mod.rs
  src/cli/src/manifest/format.rs src/cli/src/utils.rs
  src/cli/src/outputs/mod.rs pkg/core/kernel/initramfs-init.sh
  pkg/core/kernel/initramfs.yaml pkg/core/kernel/linux.yaml
  asm/installer/installer-overlay.yaml asm/installer/installer.yaml
  scripts/qemu-test-installer.sh`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml
  outputs::tests::generated_outputs_preserve_existing_output_names`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml
  manifest::format::tests::formats_system_overlays`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml
  build::tests::refreshes_chroot_usrmerge_symlinks`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml
  utils::tests::determine_category_handles_common_layouts`: passed.
- `rustfmt --check --edition 2021 src/cli/src/manifest/format.rs`: passed for
  the formatter inheritance fix.
- `cargo test --manifest-path src/cli/Cargo.toml
  manifest::format::tests::formats_system_extends`: passed for the formatter
  inheritance fix.
- `cargo test --manifest-path src/cli/Cargo.toml
  manifest::format::tests::formats_system_overlays`: passed after the formatter
  inheritance fix.
- `./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose
  --check --update-checksum --force`: failed on 2026-06-28 at the system
  reproducibility gate after successful package checks for the new refs; first
  checksum `973c9358e52ad22e6f4eb65a64f1650c76cc189a100adc9a715113587ad38c3a`,
  second checksum
  `6af0e9da98970c4922c46e5c357dcc78d2d7ad900d9164666abace35a2ac5f21`.
- `rustfmt --check --edition 2021 src/cli/src/system/mod.rs`: passed.
- `cargo build --manifest-path src/cli/Cargo.toml`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml`: failed because
  `build::orchestration::tests::rebuilds_when_dependency_manifest_changes`
  reported `uid 0 not mapped in namespace` and
  `test_time_travel_history_search` could not find `ostree`.
- `cargo test --manifest-path src/cli/Cargo.toml --bin nex -- --skip
  build::orchestration::tests::rebuilds_when_dependency_manifest_changes`:
  passed; 54 tests passed and 1 was filtered out.
- `cargo test --manifest-path src/cli/Cargo.toml --test blob_ref_tests --
  --skip test_time_travel_history_search`: passed; 2 tests passed and 1 was
  filtered out.
- `./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose
  --check --update-checksum --force`: passed on 2026-06-28 after the CLI
  fix; both passes produced checksum
  `ed3fbaa8397e02d42968b2d0201d52d58e2d45ac5d03506beed3acf6d7d74b09`.
- `zub --repo .nex/repo checkout --copy systems/desktop-vwl/0.0.1
  /tmp/nex-desktop-vwl-check.6iBpKu` followed by file probes: passed. The
  checked root filesystem had symlink paths for `Xwayland`, `libseat`,
  `wlroots`, `wayland-protocols`, Mesa EGL and Vulkan ICD files,
  `libvulkan`, `libsecret`, `libnotify`, `notify-send`, `qemu-system-x86_64`,
  `bios.bin`, `virsh`, `libvirt.conf`, `libvirt.so.0`, `libvirtd`, and
  `mkfs.vfat`.
- `./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check
  --update-checksum --force`: passed on 2026-06-28; both passes produced
  checksum `f2ceab5871533bfdf5a83f494c9acc7796a177786502abd7c803eef320fbe512`.
- `zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1
  /tmp/nex-desktop-dev-check.ednxJd` followed by file probes: passed. The
  checked root filesystem had `/usr/bin/node`, `/usr/bin/npm`, and
  `/usr/bin/npx` symlinks into the Node.js package capsule.
- `./src/cli/target/debug/nex check asm/nex-systemd.yaml
  asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml`: passed.
- `git diff --check -- src/cli/src/system/mod.rs asm/nex-systemd.yaml
  asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml
  manifest::format::tests::preserves_system_item_comments`: passed for the
  formatter comment-preservation fix.
- `cargo build --manifest-path src/cli/Cargo.toml`: passed after the formatter
  comment-preservation fix.
- `./src/cli/target/debug/nex check asm/nex-systemd.yaml
  asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml`: passed after the
  formatter fixes and assembly edits.

## Decision Log

- Decision: Ralph will test and commit current uncommitted package work before
  adding OSTreefy package refs to assemblies.
  Rationale: The human explicitly clarified that the work should start with the
  current uncommitted packages. A package commit gives the assembly work a
  checked base and avoids mixing existing kernel/initramfs changes with new
  desktop package wiring.
  Date/Author: 2026-06-27 / Carlos

- Decision: After the current package commit, Ralph will work on assembly
  manifests and existing package manifests, not new package manifests.
  Rationale: `OSTREEFY_REPLICATION_REPORT.md` says twelve old OSTreefy packages
  or close equivalents already have local manifests but do not land in the main
  desktop assembly. Assembly wiring proves those existing packages first.
  Date/Author: 2026-06-27 / Carlos

- Decision: Ralph will leave Chromium out of this ExecPlan.
  Rationale: `CURRENT_TODO.md` records a known GTK3 blocker in the Chromium
  stream. Adding Chromium to `desktop-vwl` before that package builds would make
  this assembly task depend on a large browser package repair.
  Date/Author: 2026-06-27 / Carlos

- Decision: Ralph will put desktop runtime packages in `asm/desktop-vwl/desktop-vwl.yaml`
  and Node.js in `asm/desktop-dev.yaml`.
  Rationale: `desktop-vwl` is the personal desktop runtime assembly. `desktop-dev`
  extends it with build tools and scripting languages, and Node.js fits that
  role better than the base desktop runtime.
  Date/Author: 2026-06-27 / Carlos

- Decision: Ralph will not add `linux-headers` in this plan.
  Rationale: The report says `linux-headers` matters when the target system
  needs kernel module builds. This plan does not add DKMS packages or other
  module build paths.
  Date/Author: 2026-06-27 / Carlos

- Decision: Ralph will use the direct initramfs QEMU mode as the package commit
  boot proof.
  Rationale: The full installer-media path needs host FAT and partition tools
  that are not available here. The direct path still boots the rebuilt kernel
  and initramfs against an installed-style disk, includes a decoy `nex-var`
  disk, and verifies the early boot mount contract inside the guest.
  Date/Author: 2026-06-28 / Carlos

## Outcomes & Retrospective

Ralph first committed the current kernel and initramfs package work as
`014bb2f pkg/kernel: fix deployment boot mounts`. The direct initramfs QEMU
assertion proved the deployment mount contract and printed `ASSERT-BOOT-PASS`.

Ralph fixed two formatter defects before editing assemblies:
`297fd29 cli: preserve assembly inheritance` and
`4af1199 cli: preserve manifest group comments`.

Ralph fixed the system reproducibility check in
`d757d0b cli: fix system reproducibility check`. The bug made the second
`nex_structure` system build materialize expanded dependency refs instead of
the explicit assembly package refs.

Ralph added the non-browser OSTreefy desktop package refs in
`a4670a4 asm: add ostreefy desktop packages`. `desktop-vwl` now includes
Xwayland, Wayland protocol data, libseat, wlroots, Mesa EGL and Vulkan ICD
files, vulkan-loader, libsecret, libnotify, QEMU, libvirt, and dosfstools.
`desktop-dev` extends `desktop-vwl` and adds the development toolchain plus
Node.js.

Both changed systems built with `--check --update-checksum --force`. The
checked-out artifacts had the expected symlink paths for the new runtime files
and Node.js commands.

The full `cargo test --manifest-path src/cli/Cargo.toml` check could not pass
in this sandbox because one test reported `uid 0 not mapped in namespace` and
one integration test needs an `ostree` executable that is not installed. The
targeted CLI unit suite and the integration file with that missing-tool test
filtered out both passed.

Not started.

## Context and Orientation

Read these files before edits:

- `AGENTS.md` for repo rules, commit checks, and Ralph rules.
- `.agents/RUST_CODE_STYLE.md` for Rust changes.
- `.agents/MANIFESTS_CODE_STYLE.md` for package and assembly manifest changes.
- `.agents/TESTING.md` for the evidence required before each commit.
- `.agents/PLANS.md` for the ExecPlan format.
- `OSTREEFY_REPLICATION_REPORT.md` for the old personal image comparison.
- `CURRENT_TODO.md` for the Chromium and GTK3 blocker.

Current uncommitted package files that Ralph must handle first:

- `pkg/core/kernel/initramfs-init.sh`
- `pkg/core/kernel/initramfs.yaml`
- `pkg/core/kernel/linux.yaml`

Ralph should edit these files:

- `asm/nex-systemd.yaml`, only if the formatter changes it.
- `asm/desktop-vwl/desktop-vwl.yaml`, to add runtime packages that belong in the
  desktop root filesystem.
- `asm/desktop-dev.yaml`, to add Node.js if the manifest does not already carry
  it.
- This ExecPlan, to record progress and check output.

Existing package manifests that this plan may use:

- `pkg/libs/security/libsecret.yaml`
- `pkg/dev/virt/libvirt.yaml`
- `pkg/dev/virt/qemu.yaml`
- `pkg/libs/graphics/mesa.yaml`
- `pkg/libs/graphics/vulkan-loader.yaml`
- `pkg/servers/xwayland.yaml`
- `pkg/libs/wayland/libseat.yaml`
- `pkg/libs/graphics/libnotify.yaml`
- `pkg/libs/wayland/wayland-protocols.yaml`
- `pkg/libs/wayland/wlroots.yaml`
- `pkg/core/fs/dosfstools.yaml`
- `pkg/dev/lang/nodejs.yaml`

Do not add these packages in this plan:

- `pkg/apps/web/chromium.yaml`, because `CURRENT_TODO.md` tracks an active GTK3
  build blocker.
- `pkg/core/kernel/linux-headers.yaml`, unless another file already requires
  kernel header files during this same assembly build.

## Plan of Work

Ralph will first make the local CLI available through `./nex`. Ralph will then
inspect the existing package diffs under `pkg/core/kernel/`, run the package
checks and builds that prove those changes work, and add an automated QEMU test
that proves the changed mount behavior. Ralph must commit the boot test with the
package files it covers. Ralph must not mix this package commit with unrelated
assembly, CLI, script, note, or ExecPlan changes.

After the current package work has a checked commit, Ralph will format and check
the three assembly manifests named by the report. After the formatter pass,
Ralph will add package refs by reading each package manifest for the package
version and choosing the smallest output or bundle that the running system
needs.

Ralph should place these runtime packages in `asm/desktop-vwl/desktop-vwl.yaml`:

- `libsecret`
- `libvirt`
- `qemu`
- `mesa`
- `vulkan-loader`
- `xwayland`
- `libseat`
- `libnotify`
- `wayland-protocols`
- `wlroots`
- `dosfstools`

Ralph should place this developer package in `asm/desktop-dev.yaml`:

- `nodejs`

Ralph must build the changed assemblies after edits. If a package ref points to
a missing store object, Ralph should build that package manifest with the strict
package command from `AGENTS.md`, then rerun the assembly build. If a package
build fails for a real package bug, Ralph should either fix that package within
this plan when the fix stays small or record the blocker and split a package
ExecPlan before continuing.

## Concrete Steps

Run these commands from the repo root.

```bash
git status --short --branch
```

Build the local CLI if `./src/cli/target/debug/nex` does not exist:

```bash
cargo build --manifest-path src/cli/Cargo.toml
```

Review current package diffs before any assembly edit:

```bash
git diff -- pkg/core/kernel/initramfs-init.sh pkg/core/kernel/initramfs.yaml pkg/core/kernel/linux.yaml
```

Check the initramfs shell before building it:

```bash
sh -n pkg/core/kernel/initramfs-init.sh
if grep -n "$(printf '\t')" pkg/core/kernel/initramfs-init.sh; then
    echo "initramfs-init.sh contains tab indentation" >&2
    exit 1
fi
```

Check the current package manifests:

```bash
./nex check pkg/core/kernel/initramfs.yaml pkg/core/kernel/linux.yaml
```

Build the current package manifests with the strict package command:

```bash
./nex build pkg/core/kernel/initramfs.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
./nex build pkg/core/kernel/linux.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

Add or extend a committed test so this command installs a target disk, boots
that target in QEMU, probes the running guest, and exits zero only after every
assertion passes:

```bash
./scripts/qemu-test-installer.sh --rebuild --autoinstall --headless --extra-nex-var --assert-boot
```

The `--assert-boot` mode does not exist yet. Add it to
`scripts/qemu-test-installer.sh`. It must capture logs, enforce a timeout, and
fail unless the installed target:

- boots the deployment selected by the `zub=` kernel argument
- chooses the `nex-var` partition adjacent to the root partition instead of the
  decoy `nex-var` partition on the extra disk
- mounts the sysroot read-only after early boot finishes
- uses the writable partition for `/var`
- exposes writable `/etc`, `/home`, and `/root` from that partition
- exposes the expected writable Nex repository, user, and manifest paths
- reaches a working systemd target and accepts a command probe from the host

Print the serial log and guest probe output when any assertion fails. A timeout
or a visible login prompt does not count as success.

Run the manifest checks once more after package builds update metadata:

```bash
./nex check pkg/core/kernel/initramfs.yaml pkg/core/kernel/linux.yaml
```

Stage and commit the current package files plus their automated test only after
those checks pass:

```bash
git add pkg/core/kernel/initramfs-init.sh pkg/core/kernel/initramfs.yaml pkg/core/kernel/linux.yaml
git add scripts/qemu-test-installer.sh
git commit -m "pkg: fix deployment root boot mounts"
```

Only after that package commit exists, start the assembly part of this plan.

Format the assembly manifests:

```bash
./nex format asm/nex-systemd.yaml asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml
```

Check the assembly manifests after the formatter changes files:

```bash
./nex check asm/nex-systemd.yaml asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml
```

Read each package manifest to get the exact package version and output or bundle
name before adding refs:

```bash
sed -n '1,220p' pkg/libs/security/libsecret.yaml
sed -n '1,220p' pkg/dev/virt/libvirt.yaml
sed -n '1,220p' pkg/dev/virt/qemu.yaml
sed -n '1,220p' pkg/libs/graphics/mesa.yaml
sed -n '1,220p' pkg/libs/graphics/vulkan-loader.yaml
sed -n '1,220p' pkg/servers/xwayland.yaml
sed -n '1,220p' pkg/libs/wayland/libseat.yaml
sed -n '1,220p' pkg/libs/graphics/libnotify.yaml
sed -n '1,220p' pkg/libs/wayland/wayland-protocols.yaml
sed -n '1,220p' pkg/libs/wayland/wlroots.yaml
sed -n '1,220p' pkg/core/fs/dosfstools.yaml
sed -n '1,220p' pkg/dev/lang/nodejs.yaml
```

After edits, run the manifest checks again:

```bash
./nex check asm/nex-systemd.yaml asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml
```

Build the desktop assembly and ask Nex to update the assembly checksum if the
root filesystem changed as expected:

```bash
./nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force --build-dir .nex/tmp/build_rootfs_desktop_vwl_system
```

Build the dev assembly if `asm/desktop-dev.yaml` changed:

```bash
./nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force --build-dir .nex/tmp/build_rootfs_desktop_dev_system
```

Inspect the built root filesystem for the new runtime packages:

```bash
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/libs/security/libsecret
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/dev/virt/libvirt
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/dev/virt/qemu
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/libs/graphics/mesa
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/libs/graphics/vulkan-loader
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/servers/xwayland
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/libs/wayland/libseat
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/libs/graphics/libnotify
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/libs/wayland/wayland-protocols
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/libs/wayland/wlroots
test -d .nex/tmp/build_rootfs_desktop_vwl_system/target/nex/pkg/core/fs/dosfstools
test -x .nex/tmp/build_rootfs_desktop_vwl_system/target/usr/bin/qemu-system-x86_64
test -x .nex/tmp/build_rootfs_desktop_vwl_system/target/usr/bin/virsh
test -x .nex/tmp/build_rootfs_desktop_vwl_system/target/usr/bin/mkfs.vfat
```

Inspect the dev root filesystem if `nodejs` enters `desktop-dev`:

```bash
test -d .nex/tmp/build_rootfs_desktop_dev_system/target/nex/pkg/dev/lang/nodejs
test -x .nex/tmp/build_rootfs_desktop_dev_system/target/usr/bin/node
test -x .nex/tmp/build_rootfs_desktop_dev_system/target/usr/bin/npm
```

Stage exact files only:

```bash
git add asm/nex-systemd.yaml asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml
```

Commit only after checks pass:

```bash
git commit -m "asm: add existing ostreefy desktop packages"
```

## Validation and Acceptance

The work is complete when these facts are true:

- Ralph committed the current package work before assembly edits, and that
  commit includes only `pkg/core/kernel/initramfs-init.sh`,
  `pkg/core/kernel/initramfs.yaml`, `pkg/core/kernel/linux.yaml`, or package
  metadata files that the package build command updated.
- `./nex check pkg/core/kernel/initramfs.yaml pkg/core/kernel/linux.yaml`
  passes before the package commit.
- The strict build commands for `pkg/core/kernel/initramfs.yaml` and
  `pkg/core/kernel/linux.yaml` pass before the package commit, unless Ralph
  records a hard package blocker in this ExecPlan.
- `sh -n pkg/core/kernel/initramfs-init.sh` passes and the script contains no
  tab indentation.
- `./scripts/qemu-test-installer.sh --rebuild --autoinstall --headless --extra-nex-var --assert-boot`
  completes the install, boots the target, and passes every guest mount and
  system-state assertion.
- `./nex check asm/nex-systemd.yaml asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml`
  passes.
- `./nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force --build-dir .nex/tmp/build_rootfs_desktop_vwl_system`
  passes.
- `./nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force --build-dir .nex/tmp/build_rootfs_desktop_dev_system`
  passes if `asm/desktop-dev.yaml` changed.
- Ralph runs installed commands or service probes for every package added to an
  assembly. Directory and executable-bit checks support those probes but do not
  replace them.
- `git diff --check` reports no whitespace errors for the files Ralph changed.
- Ralph records each command and result in `Artifacts and Notes`.

If any added package cannot build or cannot enter the assembly without a larger
package repair, Ralph must record the failing command, the concrete missing file
or failing build step, and the package manifest that needs the next ExecPlan.

## Idempotence and Recovery

The formatter command is safe to rerun on the same three assembly manifests.
The assembly build commands use fixed `--build-dir` paths, so Ralph can inspect
the root filesystem after a successful build. If a build leaves stale files in a
build directory, Ralph may remove only that specific build directory and rerun
the command.

Do not run `git add -A`. If unrelated worktree changes appear, leave them in
place and stage only the files named in this plan. If a package build updates a
package manifest checksum or output list, record that file in this plan before
staging it. The first package commit must not stage assembly manifests or this
ExecPlan.

## Artifacts and Notes

- `AGENTS.md` says active ExecPlans use numbered names under
  `.agents/execplans/`.
- `.agents/PLANS.md` says assembly plans must name the assembly manifest, build
  command, and root filesystem check.
- `./nex` is a wrapper that builds `src/cli` and then runs
  `./src/cli/target/debug/nex`.
- `git status --short -- pkg asm .agents CURRENT_TODO.md chromium-manifests-todo.md OSTREEFY_REPLICATION_REPORT.md`
  showed current package changes in `pkg/core/kernel/` plus assembly and note
  changes. Ralph should commit the package files first and leave the assembly
  and note files for later commits.
- Revision note, 2026-06-27: Renamed this file with the `001` sequence and kept
  active plans under `.agents/execplans/` because the human chose direct
  numerical ordering.
