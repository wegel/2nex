# Final Boot And Daily Driver Validation

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Validate the near-final OSTreefy parity system as a whole. Earlier subplans
proved package and row-level behavior. This plan proves the assembled systems
still build, expose the daily commands and service files expected by the
matrix, and either boot or record the smallest concrete remaining boot risk.

## Progress

- [x] (2026-06-29 08:18Z) Started this sub-EP after completing
  `002h-hard-or-policy-heavy-packages`.
- [x] (2026-06-29 08:18Z) Ran the new-subplan worktree pre-task. Only local
  notes and settings were visible as untracked files, and no tracked commit
  candidate was present.
- [x] (2026-06-29 08:18Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`.
- [x] (2026-06-29 05:57Z) Rechecked the parity matrix. No `needs-*` rows
  remain; all open rows are explicit request or policy deferrals.
- [x] (2026-06-29 05:57Z) Rebuilt `desktop-dev` after switching dev tools
  with support files from `outputs/bin` to full/dev bundles and after fixing
  the Meson launcher. The build is reproducible with checksum
  `a0b3c72bc6b8e4f815e897e1998dce51c2d899bdeb1cbd486a93b6fab5c18588`.
- [x] (2026-06-29 05:57Z) Checked out `systems/desktop-dev/0.0.1` and ran
  the dev-tool smoke suite. CMake, Meson, Ninja, autotools, GN, Rust, Node,
  Python, Perl, GDB, LLDB, and the listed helper tools all started.
- [x] (2026-06-29 05:57Z) Ran `scripts/qemu-test-installer.sh
  --direct-initramfs --assert-boot --timeout 180`. The guest booted
  `systems/desktop-vwl/0.0.1` through the kernel, initramfs, readonly sysroot,
  writable var partition, systemd, and printed `ASSERT-BOOT-PASS`.

## Surprises & Discoveries

- `desktop-dev` originally used `outputs/bin` for several build tools. That
  exposed command files but omitted support directories such as CMake modules
  and Meson Python modules.
- Switching Meson to `bundles/dev` alone did not fix `meson --version`.
  Python's `sys.path` came from the Python package under `/nex/pkg/...`, so it
  did not search Meson's package-local site-packages path.
- The Meson package needed a launcher wrapper that resolves its real package
  path and prepends its own `usr/lib/python3.12/site-packages` before importing
  `mesonbuild`.
- The direct initramfs boot proof is the smallest non-interactive boot check on
  this host. It does not require OVMF or an ESP, and it uses serial logs instead
  of a GUI window.
- The direct boot serial log showed an `efi.automount` failure because the
  direct image has no ESP partition. The boot proof still passed because the
  assertion service checks the deployment root, readonly `/sysroot`, writable
  `/var` backed paths, and systemd startup.

## Decision Log

- Use full/dev assembly refs for build tools that need support data, not only
  `outputs/bin`, because a version command can depend on files outside
  `/usr/bin`.
- Fix the Meson package rather than setting `PYTHONPATH` in the smoke command,
  because users need `meson` to work from the assembled system without extra
  shell setup.
- Use `scripts/qemu-test-installer.sh --direct-initramfs --assert-boot
  --timeout 180` as the boot-path acceptance check for 002i, because
  `scripts/qemu-desktop.sh` opens a GTK VM and expects local SSH keys.

## Outcomes & Retrospective

`desktop-dev` now passes its build and smoke checks, and `desktop-vwl` boots
through the direct initramfs assertion path. Remaining open parity rows are
intentional request or policy deferrals, not missing package work found during
final validation.

## Context and Orientation

The parity matrix has 163 rows. This subplan should not reopen package work
unless final validation reveals a concrete missing command, service, file, or
boot artifact. Policy rows may remain deferred when they already name the
missing human or hardware policy.

Important assemblies:

- `asm/desktop-vwl/desktop-vwl.yaml`: daily desktop runtime.
- `asm/desktop-dev.yaml`: development desktop that extends `desktop-vwl`.

Useful scripts:

- `scripts/qemu-desktop.sh`
- `scripts/qemu-test-installer.sh`
- `scripts/create-installer-usb`

## Plan of Work

1. Recheck the matrix for vague or stale rows. Every row should be `covered`,
   `replaced`, `skipped`, `deferred-request`, or `deferred-policy` with a
   concrete reason.
2. Build `desktop-vwl` and `desktop-dev` reproducibly when their manifests or
   source snapshots changed since the last proof.
3. Check out both systems through the zub store using disposable paths named in
   `.agents/cleanup-workdirs.sh`.
4. Run a compact daily-driver smoke suite inside checked-out roots. Cover CLI
   tools, Wayland helpers, browser/media, container and VM commands, Python app
   wrappers, fonts, portals, time sync, audio service files, and dev tools.
5. Run the smallest available boot or installer-path check that proves the
   built desktop root can enter the Nex boot path. If the host lacks QEMU or a
   required tool, record the exact missing command and stop only if
   `AGENTS.md` classifies it as a hard blocker.
6. Update the matrix, main EP, scratch knowledge, and durable knowledge with
   any verified final-validation facts.

## Concrete Steps

Use these commands as the baseline, adjusting only when evidence says a
smaller or more precise check proves the same result:

```bash
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
bash .agents/cleanup-workdirs.sh
zub --repo .nex/repo checkout --copy --force systems/desktop-vwl/0.0.1 .nex/tmp/desktop-vwl-002i-smoke
zub --repo .nex/repo checkout --copy --force systems/desktop-dev/0.0.1 .nex/tmp/desktop-dev-002i-smoke
```

Add `.nex/tmp/desktop-vwl-002i-smoke` and
`.nex/tmp/desktop-dev-002i-smoke` to `.agents/cleanup-workdirs.sh` before
using them.

## Validation and Acceptance

- Both target assemblies pass `nex check`.
- Both target assemblies either build reproducibly or have an already-current
  checksum whose build proof is named.
- Checked-out roots pass a representative smoke suite that can fail when
  package symlinks, runtime closures, or command wrappers are broken.
- The boot path is exercised by the smallest repo script that can run on this
  host, or the plan records the exact missing host tool.
- The main parity ExecPlan says which rows remain deferred and why.
