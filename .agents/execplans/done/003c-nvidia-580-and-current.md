# Build Nvidia 580 And Current Driver Packages

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex must build two Nvidia driver branches from pinned upstream runfiles:

- 580 proprietary kernel modules plus matching userspace for the current
  GTX 1060 plus RTX 2070 machine
- 595.84 open kernel modules plus matching userspace for RTX 5090-class
  machines

The repository may build both branches, but one booted root must activate only
one branch. Nvidia uses the same module names and userspace library names for
each branch, so assemblies must choose the branch explicitly.

## Progress

- [x] (2026-06-29 15:10Z) Started this sub-EP after 003a and 003b completed.
- [x] (2026-06-29 15:10Z) Ran the Ralph worktree pre-task. Only untracked local
  notes and local settings were present; no tracked commit candidate existed.
- [x] (2026-06-29 15:10Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`.
- [x] (2026-06-29 15:10Z) Read the relevant kernel, package manifest, and
  reproducibility knowledge notes.
- [x] (2026-06-29 15:42Z) Inspected the pinned Nvidia 580 and 595.84
  no-compat32 runfiles and verified their sha256 sums.
- [x] (2026-06-29 16:25Z) Fixed the Linux module SDK so Nvidia 580 can build:
  the kernel now leaves unused exports available and copies `.config` into the
  SDK. `nex check pkg/core/kernel/linux.yaml` passed, the strict kernel build
  passed reproducibly, and a manual Nvidia 580 proprietary module build
  produced `nvidia.ko`, `nvidia-drm.ko`, `nvidia-modeset.ko`, and
  `nvidia-uvm.ko`.
- [x] (2026-06-29 14:07Z) Added the Nvidia 580 manifest and proved its module
  and userspace files. The strict package build passed reproducibly with
  checksum `cc4e86a77f4e13f25d941e6bdbbcf4a9fd0fc5553791210bf2782fef63d166da`.
  The runtime smoke showed module version `580.159.04`, vermagic
  `6.12.58 SMP modversions`, license `NVIDIA`, and `nvidia-smi --help`
  reported v580.159.04.
- [x] (2026-06-29 14:07Z) Added the Nvidia current manifest and proved its
  module and userspace files. The strict package build passed reproducibly
  with checksum
  `9673fd029e8b84c456664558e46813a07d3e75e14fb631cf5eda06118c87e233`.
  The runtime smoke showed module version `595.84`, vermagic
  `6.12.58 SMP modversions`, license `Dual MIT/GPL`, and
  `nvidia-smi --help` reported v595.84.
- [x] (2026-06-29 14:07Z) Ran the package checks that prove both branches:
  `nex check` passed for both manifests; runtime smokes checked module
  version, vermagic, license, and `nvidia-smi --help` for both branches.
- [x] (2026-06-29 14:07Z) Committed the checked package manifests as
  `22778f8 pkg: add nvidia driver branches`.
- [x] (2026-06-29 14:07Z) Updated the main EP and durable knowledge after both
  packages were checked.

## Surprises & Discoveries

- Observation: Nvidia publishes `580.159.04` and `595.84` Linux x86_64
  runfile directories with `-no-compat32` variants and sha256 files.
  Evidence: `https://download.nvidia.com/XFree86/Linux-x86_64/580.159.04/`
  and `https://download.nvidia.com/XFree86/Linux-x86_64/595.84/`.

- Observation: Nvidia 595.84 README says open and proprietary kernel module
  flavors are mutually exclusive, and Blackwell and later GPUs require the
  open module flavor.
  Evidence:
  `https://download.nvidia.com/XFree86/Linux-x86_64/595.84/README/kernel_open.html`.

- Observation: The previous Linux 6.12.58 module SDK could not build Nvidia
  580 because `CONFIG_TRIM_UNUSED_KSYMS=y` removed exported symbols that
  out-of-tree modules need.
  Evidence: the manual Nvidia 580 build failed at `MODPOST` with missing
  symbols such as `cpufreq_get`, `iterate_fd`, and
  `drm_edid_override_connector_update`; the old SDK `include/config/auto.conf`
  contained `CONFIG_TRIM_UNUSED_KSYMS=y`.

- Observation: Nvidia's kernel Makefile reads the kernel SDK `.config` while
  checking compiler details, so the SDK must include that file in addition to
  generated config headers.
  Evidence: before the SDK copied `.config`, Nvidia printed a missing
  `.config` diagnostic during the external module build.

- Observation: The rebuilt SDK fixes both issues for the 580 branch.
  Evidence: `modinfo -F version .nex/tmp/nvidia-580-inspect/kernel/nvidia.ko`
  printed `580.159.04`, `modinfo -F vermagic` printed
  `6.12.58 SMP modversions`, and `modinfo -F license` printed `NVIDIA`.

- Observation: The Nvidia 580 package can extract the zstd-compressed runfile
  directly and skip the makeself wrapper.
  Evidence: the strict `pkg/libs/graphics/nvidia-580.yaml` build passed
  reproducibly after the manifest used `tail`, `zstd -d`, and
  `tar --no-same-owner` instead of executing the runfile stub.

- Observation: `nvidia-smi --help` can run from a checked-out runtime bundle
  when `LD_LIBRARY_PATH` points at that bundle's `/usr/lib`.
  Evidence: `LD_LIBRARY_PATH=.nex/tmp/nvidia-580-smoke/usr/lib
  .nex/tmp/nvidia-580-smoke/usr/bin/nvidia-smi --help` exited 0 and printed
  the 580.159.04 version line.

- Observation: The Nvidia current package can build the 595.84 open module
  flavor reproducibly against the Linux 6.12.58 module SDK.
  Evidence: the strict `pkg/libs/graphics/nvidia-current.yaml` build passed
  reproducibly with checksum
  `9673fd029e8b84c456664558e46813a07d3e75e14fb631cf5eda06118c87e233`.

- Observation: The two Nvidia package manifests keep the branches separate at
  the package level.
  Evidence: `pkg/libs/graphics/nvidia-580.yaml` builds from `kernel/` and
  smokes with license `NVIDIA`; `pkg/libs/graphics/nvidia-current.yaml` builds
  from `kernel-open/` and smokes with license `Dual MIT/GPL`. Both install the
  same kernel module names, so an assembly must choose one package branch.

## Decision Log

- Decision: Use the `-no-compat32` upstream runfiles unless package inspection
  proves that Nex needs 32-bit userspace libraries.
  Rationale: Nex currently packages 64-bit desktop userspace, and the no-compat
  runfiles are smaller and avoid extra library outputs.
  Date/Author: 2026-06-29 / Carlos

- Decision: Treat Nvidia 595.84 as the current production branch for this EP.
  Rationale: The human specifically asked to build current 595, and Nvidia's
  Linux x86_64 directory contains the 595.84 release.
  Date/Author: 2026-06-29 / Carlos

- Decision: Commit the kernel SDK export and `.config` fix before adding the
  Nvidia package manifests.
  Rationale: Nvidia packaging depends on this kernel behavior, and the kernel
  change has its own reproducible build plus an external-module smoke check.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

Completed. Nex now has separate package manifests for the 580 proprietary
branch and the 595.84 current open branch. Both packages build against the
Linux 6.12.58 module SDK, install matching userspace, and pass runtime bundle
smokes for module metadata and `nvidia-smi --help`.

## Context and Orientation

Read these files before work:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/reproducibility.md`

Relevant existing package files:

- `pkg/core/kernel/linux.yaml`
- `pkg/core/kernel/v4l2loopback.yaml`
- `pkg/core/kernel/kmod.yaml`
- `pkg/libs/graphics/vulkan-loader.yaml`
- `pkg/libs/graphics/mesa.yaml`

## Plan of Work

1. Add cleanup paths for Nvidia inspection and smoke roots to
   `.agents/cleanup-workdirs.sh`, then use that script before any large
   download or extraction.
2. Fetch the upstream sha256 files for
   `NVIDIA-Linux-x86_64-580.159.04-no-compat32.run` and
   `NVIDIA-Linux-x86_64-595.84-no-compat32.run`.
3. Inspect each runfile with `--extract-only` in disposable scratch space.
4. Determine the minimum install commands or source build commands that produce
   modules against `/usr/src/linux-6.12.58` without writing outside
   `${OUT_DIR}`.
5. Package 580 as the proprietary branch. Install its kernel modules under
   `/usr/lib/modules/6.12.58/extra` and the matching userspace under `/usr`.
6. Package 595.84 as the current open branch. Install its kernel modules under
   `/usr/lib/modules/6.12.58/extra` and the matching userspace under `/usr`.
7. Do not ship package-local `depmod` metadata from either package. Smoke
   checked-out roots by running `depmod -b <root>/usr 6.12.58` after layering
   kernel modules when needed.
8. Regenerate package outputs and dependency metadata with the strict package
   build command.

## Concrete Steps

Use these commands as the baseline:

```bash
./src/cli/target/debug/nex check pkg/libs/graphics/nvidia-580.yaml
./src/cli/target/debug/nex build pkg/libs/graphics/nvidia-580.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

./src/cli/target/debug/nex check pkg/libs/graphics/nvidia-current.yaml
./src/cli/target/debug/nex build pkg/libs/graphics/nvidia-current.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

For each checked-out package root, prove:

```bash
modinfo -F version <root>/usr/lib/modules/6.12.58/extra/nvidia.ko
modinfo -F vermagic <root>/usr/lib/modules/6.12.58/extra/nvidia.ko
modinfo -F license <root>/usr/lib/modules/6.12.58/extra/nvidia.ko
<root>/usr/bin/nvidia-smi --help
```

If direct execution fails because a checked-out package root lacks runtime
dependencies, make a smoke root that contains the package runtime bundle plus
its recorded runtime providers.

## Validation and Acceptance

This plan is complete when all checks pass:

- `nex check` passes for both Nvidia manifests.
- The strict two-pass package build passes for both Nvidia manifests.
- The 580 package produces Nvidia modules for Linux 6.12.58, and `modinfo`
  shows version `580.159.04`, the 6.12.58 vermagic, and proprietary license.
- The current package produces Nvidia modules for Linux 6.12.58, and
  `modinfo` shows version `595.84`, the 6.12.58 vermagic, and open module
  license.
- Each package installs `nvidia-smi` and enough matching userspace libraries
  for the smoke root to run `nvidia-smi --help`.
- The packages do not install both branches into the same active output or
  assembly.

## Idempotence and Recovery

The zub store is a cache. If inspection or builds leave large scratch roots,
add those paths to `.agents/cleanup-workdirs.sh` and run that script. Do not
remove disposable workdirs with ad hoc `rm -rf`.

If a runfile build fails against Linux 6.12.58, inspect the failure first. Add
small pinned patches only when they match upstream or obvious kernel API
changes. If the branch cannot build without an incompatible design change,
record a hard blocker.

## Artifacts and Notes

Primary upstream sources:

- `https://download.nvidia.com/XFree86/Linux-x86_64/580.159.04/`
- `https://download.nvidia.com/XFree86/Linux-x86_64/595.84/`
- `https://download.nvidia.com/XFree86/Linux-x86_64/595.84/README/kernel_open.html`
