# Add AMD Early Microcode To The Nex Boot Path

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex should load AMD CPU microcode before the kernel starts normal boot work.
The current desktop roots contain AMD firmware files through
`pkg/core/kernel/linux-firmware.yaml`, but no checked boot path passes those
files as an early microcode archive.

After this plan, a user can boot a Nex desktop on an AMD CPU and see the
kernel report that it consumed AMD microcode early. The implementation must
keep boot state reproducible: Git names the manifest and assembly refs, zub
caches the built artifacts, and the bootloader consumes artifacts selected by
the deployment.

## Progress

- [x] (2026-06-29 16:35Z) Created this ExecPlan after the human asked how Nex
  should implement microcode while respecting the project philosophy.
- [x] (2026-06-29 16:44Z) Ran the Ralph worktree pre-task. Only local
  untracked notes/settings and ignored `.agents` files were present; no
  coherent tracked commit candidate existed.
- [x] (2026-06-29 16:47Z) Verified the Linux early microcode archive path and
  file layout against upstream kernel documentation.
- [x] (2026-06-29 17:01Z) Added
  `pkg/core/kernel/amd-ucode-initramfs.yaml`.
- [x] (2026-06-29 17:07Z) Built `amd-ucode-initramfs` reproducibly with
  checksum
  `163ca6364c7c02414a3ec62ada8152a7f6eee8b9e1ffe3d1c7459d18aa763277`.
- [x] (2026-06-29 17:09Z) Checked out the package output and proved
  `/boot/amd-ucode.cpio` contains
  `kernel/x86/microcode/AuthenticAMD.bin`; extracted payload size was
  304866 bytes.
- [x] (2026-06-29 18:24Z) Taught the Nex bootloader and direct-initramfs
  QEMU path to pass the AMD
  microcode archive before other initramfs data.
- [x] (2026-06-29 18:27Z) Included the AMD early microcode artifact in the
  base `desktop-vwl` assembly. The Nvidia variants inherit it through
  `extends`.
- [x] (2026-06-29 18:35Z) Proved the artifact layout, bootloader ordering,
  direct QEMU boot path, and desktop root contents. Real hardware was not
  available during this plan, so the plan records QEMU ordering proof instead
  of a hardware microcode log.
- [x] (2026-06-29 17:12Z) Committed the package artifact change.
  Commit: `8ac0438 pkg/kernel: add amd microcode initramfs`.
- [x] (2026-06-29 18:36Z) Ran the bootloader checks:
  `rustfmt --check --config skip_children=true src/bootloader/src/main.rs`,
  `cargo test --manifest-path src/bootloader/Cargo.toml --target
  x86_64-unknown-linux-gnu --no-default-features --test cpio_test`, and
  `cargo build` from `src/bootloader`.
- [x] (2026-06-29 18:40Z) Ran `nex check` and reproducible assembly builds
  for `desktop-vwl`, `desktop-vwl-nvidia-580`, and
  `desktop-vwl-nvidia-current`.
- [x] (2026-06-29 18:45Z) Checked `zub --repo .nex/repo ls-tree -r` output
  for all three desktop roots and found `/boot/amd-ucode.cpio` in each root.
- [x] (2026-06-29 18:51Z) Ran
  `TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1
  scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout
  180 --headless`. The script logged the AMD early microcode cpio path,
  passed its `cmp -n` prefix check, booted QEMU, and printed
  `ASSERT-BOOT-PASS`.
- [x] (2026-06-29 19:02Z) Committed the bootloader, direct-initramfs, and
  assembly behavior.
  Commit: `521d8fa asm: include amd early microcode`.
- [x] (2026-06-29 19:08Z) Promoted durable knowledge into
  `.agents/knowledge/kernel-and-boot.md`,
  `.agents/knowledge/system-assemblies.md`,
  `.agents/knowledge/ostreefy-parity.md`, and
  `.agents/knowledge/reproducibility.md`.

## Surprises & Discoveries

- Observation: The linux-firmware package already has an `amd-ucode` output.
  Evidence: `pkg/core/kernel/linux-firmware.yaml` lists
  `/usr/lib/firmware/amd-ucode/microcode_amd*.bin` paths under output
  `amd-ucode`.

- Observation: The bootloader already has a path for optional extra initrd
  data.
  Evidence: `src/bootloader/src/main.rs` calls `load_boot_modules`, then
  `initrd::install_initrd_protocol(initramfs)`. `src/bootloader/src/initrd.rs`
  installs an EFI LoadFile2 protocol for Linux to fetch initrd bytes.

- Observation: The current extra-initrd path builds a cpio archive only for
  boot modules.
  Evidence: `src/bootloader/src/cpio.rs` has `build_module_initramfs`, which
  writes files under `lib/modules/`.

- Observation: Upstream Linux documents the AMD early microcode cpio path and
  ordering requirement.
  Evidence: the kernel.org Linux Microcode Loader documentation says the
  combined initrd image starts with an uncompressed cpio archive and uses
  `kernel/x86/microcode/AuthenticAMD.bin` for AMD.

- Observation: The AMD microcode package needs the full C compile toolchain as
  explicit build-time inputs.
  Evidence: the first strict build failed on missing `<linux/errno.h>` until
  the manifest added `linux-headers`; the second strict build failed on
  missing `as` until the manifest added `binutils`.

- Observation: Direct QEMU checkouts can expose deployment files as absolute
  symlinks into `/nex/pkg`.
  Evidence: the first direct QEMU run booted but skipped AMD microcode because
  `/boot/amd-ucode.cpio` was a symlink. The script now resolves absolute
  symlink targets through the checked-out root content directory before it
  concatenates initramfs data.

- Observation: `zub` finds nested root files with `ls-tree -r`; this repo does
  not provide a `zub ls` command.
  Evidence: `zub --repo .nex/repo ls-tree -r
  systems/desktop-vwl/0.0.1` found both the `/boot/amd-ucode.cpio` symlink and
  the package-owned regular file.

- Observation: Broad bootloader formatting checks still see unrelated
  pre-existing drift.
  Evidence: `cargo fmt --manifest-path src/bootloader/Cargo.toml -- --check`
  and `rustfmt --check src/bootloader/src/main.rs` reported formatting in
  existing bootloader modules outside this plan. The checked command for this
  change was the file-scoped `rustfmt --check --config skip_children=true
  src/bootloader/src/main.rs`.

## Decision Log

- Decision: Treat AMD microcode as a boot artifact selected by assemblies, not
  as mutable installer state.
  Rationale: Nex says Git defines the system, zub stores replaceable build
  outputs, and deployments should select their complete boot state.
  Date/Author: 2026-06-29 / human and Carlos

- Decision: Prefer a reproducible package output that contains an early
  microcode cpio archive.
  Rationale: A package manifest can name every input and command, while an
  install-time generator would depend on mutable host state.
  Date/Author: 2026-06-29 / Carlos

- Decision: Do not use a mkinitcpio-style generated initramfs for this work.
  Rationale: Nex prefers a constant built-in early userspace plus explicit
  extra initrd artifacts selected by the deployment.
  Date/Author: 2026-06-29 / Carlos

- Decision: Treat `/boot/amd-ucode.cpio` in a deployment as the AMD early
  microcode declaration instead of adding a separate config file.
  Rationale: The assembly already declares the boot artifact by adding the
  package that exposes that path. A fixed Linux early-microcode path keeps the
  bootloader simpler, while `boot-modules.conf` remains available for dynamic
  module cpios.
  Date/Author: 2026-06-29 / Carlos

- Decision: Put the AMD early microcode package in the base desktop assembly.
  Rationale: Both Nvidia desktop variants extend the base desktop assembly, so
  one package entry gives all desktop roots the same CPU microcode boot
  artifact without duplicating manifest entries.
  Date/Author: 2026-06-29 / Carlos

- Decision: Keep the bootloader host-side tests separate from the UEFI binary
  target.
  Rationale: The cpio integration test runs on the Linux host target and does
  not need to build the no-std UEFI binary. The `bootloader-bin` Cargo feature
  lets that test run with `--no-default-features`.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

The package commit added a reproducible AMD early microcode cpio at
`/boot/amd-ucode.cpio`. The follow-up commit made the bootloader prepend that
cpio before boot module initrd data, made the direct QEMU helper build the
same combined initramfs, and included the package in the base desktop assembly
so Nvidia variants inherit it.

The plan did not capture a real Ryzen hardware boot log because the hardware
boot path was not available during this run. The checked proof instead shows
the exact cpio payload, root contents, byte ordering before the normal
initramfs, and a successful direct QEMU boot.

## Context and Orientation

Read these files before implementation:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `RUST_CODE_STYLE.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/ostreefy-parity.md`

Relevant source files:

- `pkg/core/kernel/linux-firmware.yaml`
- `pkg/core/kernel/initramfs.yaml`
- `pkg/core/kernel/gen_init_cpio.c`
- `pkg/core/kernel/gen_initramfs.sh`
- `src/bootloader/src/main.rs`
- `src/bootloader/src/cpio.rs`
- `src/bootloader/src/initrd.rs`
- `scripts/qemu-test-installer.sh`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`
- `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml`

The Linux early AMD microcode archive must contain the concatenated AMD
microcode payload at this path inside the cpio archive:

```text
kernel/x86/microcode/AuthenticAMD.bin
```

Verify that path against the upstream Linux kernel documentation before
coding. Use the Linux source documentation as the authority if this plan and
the docs disagree.

`pkg/core/kernel/linux-firmware.yaml` already installs these AMD files from
the pinned linux-firmware source:

- `/usr/lib/firmware/amd-ucode/microcode_amd.bin`
- `/usr/lib/firmware/amd-ucode/microcode_amd_fam15h.bin`
- `/usr/lib/firmware/amd-ucode/microcode_amd_fam16h.bin`
- `/usr/lib/firmware/amd-ucode/microcode_amd_fam17h.bin`
- `/usr/lib/firmware/amd-ucode/microcode_amd_fam19h.bin`
- `/usr/lib/firmware/amd-ucode/microcode_amd_fam1ah.bin`

Do not include `.asc` signature files in the early cpio payload.

## Plan of Work

1. Run the worktree pre-task from `AGENTS.md`. Leave local notes and settings
   uncommitted.
2. Verify the Linux-required AMD early microcode cpio path and payload rules
   against upstream Linux docs.
3. Add a package manifest for the early boot artifact. Prefer
   `pkg/core/kernel/amd-ucode-initramfs.yaml` unless the repo already has a
   better naming pattern. The package should depend on the existing
   `linux-firmware` `amd-ucode` output and should build
   `/boot/amd-ucode.cpio`.
4. Build the cpio deterministically:
   - concatenate the AMD `microcode_amd*.bin` files in a stable order
   - place the result at `kernel/x86/microcode/AuthenticAMD.bin`
   - write the cpio with uid 0, gid 0, mode 0644, and fixed timestamps
   - avoid gzip unless Linux docs or the existing boot path require it
5. Add tests for the artifact layout. The test should extract or list the
   cpio and prove that `kernel/x86/microcode/AuthenticAMD.bin` appears before
   any later initramfs content when buffers are concatenated.
6. Update the bootloader so it loads `/boot/amd-ucode.cpio` from the selected
   deployment when the assembly includes that artifact. Keep
   `boot-modules.conf` behavior working.
7. Make the bootloader pass AMD microcode before module or other initrd bytes.
   Linux must see the microcode cpio first.
8. Update `scripts/qemu-test-installer.sh` direct-initramfs mode so it can use
   the same artifact order. The QEMU direct path currently passes
   `-initrd "$DIRECT_INITRAMFS"`; it must pass a combined file that starts
   with AMD microcode when the target root declares it.
9. Include the AMD early microcode package in these assemblies:
   - `asm/desktop-vwl/desktop-vwl.yaml`
   - `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`
   - `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml`
10. Update `.agents/ostreefy-parity-matrix.md` so `amd-ucode` no longer stays
    deferred once the proof passes.
11. Promote durable knowledge about the microcode artifact and boot checks
    before moving this ExecPlan to `done`.

## Concrete Steps

Start with these commands from the repository root:

```bash
git status --short --untracked-files=all
rg -n "microcode|AuthenticAMD|early" pkg/core/kernel src/bootloader scripts asm .agents/knowledge
```

Verify the existing firmware package:

```bash
./src/cli/target/debug/nex check pkg/core/kernel/linux-firmware.yaml
./src/cli/target/debug/nex build pkg/core/kernel/linux-firmware.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

After adding the new package, run:

```bash
./src/cli/target/debug/nex format pkg/core/kernel/amd-ucode-initramfs.yaml
./src/cli/target/debug/nex check pkg/core/kernel/amd-ucode-initramfs.yaml
./src/cli/target/debug/nex build pkg/core/kernel/amd-ucode-initramfs.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

Inspect the built artifact. Adjust the ref after the first build records the
checksum:

```bash
bash .agents/cleanup-workdirs.sh
zub --repo .nex/repo checkout --copy x86_64/pkg/core/kernel/amd-ucode-initramfs/0.0.1/bundles/runtime .nex/tmp/amd-ucode-initramfs-smoke
ls -l .nex/tmp/amd-ucode-initramfs-smoke/boot/amd-ucode.cpio
```

Use a cpio listing command that exists in the current environment or in a Nex
smoke root. The listing must include exactly this path:

```text
kernel/x86/microcode/AuthenticAMD.bin
```

For bootloader changes, run:

```bash
cargo test --manifest-path src/bootloader/Cargo.toml
cargo build --manifest-path src/bootloader/Cargo.toml
```

For assembly changes, run:

```bash
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl-nvidia-580.yaml
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl-nvidia-580.yaml --verbose --check --update-checksum --force
```

Run the direct boot assertion against the AMD desktop candidate:

```bash
TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1 scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout 180 --headless
```

If the host can boot real hardware during this plan, capture a boot log from
the Ryzen machine and record the kernel line that proves early AMD microcode
loaded. If real hardware is not available, record that QEMU cannot prove AMD
microcode loading on a host-independent virtual CPU and keep the artifact
ordering proof explicit.

## Validation and Acceptance

This ExecPlan is complete when all of these checks pass:

- The AMD early microcode package manifest passes `nex check`.
- The AMD early microcode package builds reproducibly with `--check`.
- The built package contains `/boot/amd-ucode.cpio`.
- A cpio listing or extraction proves the archive contains
  `kernel/x86/microcode/AuthenticAMD.bin`.
- A test or script proves the bootloader and direct-initramfs QEMU path place
  AMD microcode bytes before any ordinary initramfs bytes.
- The changed bootloader Rust code passes its tests and build.
- The changed desktop assembly passes `nex check` and a reproducible
  assembly build.
- A checked-out desktop root contains the declared boot artifact at
  `/boot/amd-ucode.cpio`.
- The direct-initramfs QEMU check still prints `ASSERT-BOOT-PASS`.
- The plan records a real hardware boot-log proof when hardware is available,
  or records why QEMU can only prove ordering and boot-path integration.

The final commit set must leave each commit buildable. Do not commit a package
manifest before its reproducible build and artifact smoke pass. Do not commit
bootloader behavior before the Rust tests and direct-initramfs path pass.

## Idempotence and Recovery

The zub store is a cache. If a package or assembly checksum looks stale,
rebuild from the manifest with `--force --check --update-checksum`.

Use `.agents/cleanup-workdirs.sh` for disposable roots and extracted cpio
trees. Add these paths before using them:

- `.nex/tmp/amd-ucode-initramfs-smoke`
- `.nex/tmp/amd-ucode-initramfs-extract`

Do not remove `.nex/tmp` paths by hand with ad hoc `rm -rf`.

If the bootloader cannot prove multiple initrd ordering through EFI LoadFile2,
stop and record a hard blocker. The human must choose whether Nex should
concatenate initrd data in the bootloader, pass a generated combined cpio from
the deployment, or change the boot protocol.

If upstream Linux docs contradict the `AuthenticAMD.bin` path in this plan,
follow the Linux docs and update this plan before implementing.

## Artifacts and Notes

The earlier parity matrix row said:

```text
amd-ucode | base | deferred-policy | 002h | Deferred until Nex chooses a CPU microcode boot policy.
```

This plan chooses the policy: Nex packages AMD early microcode as a
reproducible boot artifact, assemblies declare that artifact, and the
bootloader passes it before other initramfs data.

Useful local facts from initial inspection:

- `pkg/core/kernel/linux-firmware.yaml` version `20251125` already owns the
  AMD microcode input files.
- `asm/desktop-vwl/desktop-vwl.yaml` currently includes
  `linux-firmware` `bundles/desktop`, which does not include the `amd-ucode`
  output.
- `src/bootloader/src/main.rs` currently loads only boot modules as optional
  initrd data.
- `scripts/qemu-test-installer.sh` direct-initramfs mode currently passes one
  initramfs path to QEMU.
