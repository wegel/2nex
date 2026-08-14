# Build Nvidia, Looking Glass, And v4l2loopback Support

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex must support the hardware and capture stack that the human actually uses:
Nvidia GPUs, Looking Glass, and v4l2loopback. The earlier parity plan deferred
these rows while Nex lacked a kernel-module policy. The human chose the policy
now: the kernel package must expose a module SDK, and out-of-tree module
packages must build against that exact SDK.

After this plan, Nex should build these concrete variants:

- a 580-series Nvidia deployment for machines with Pascal GPUs, such as the
  current GTX 1060 plus RTX 2070 machine
- a current Nvidia deployment for machines that need newer GPUs, such as RTX
  5090
- v4l2loopback kernel module and user tools matched to the shipped kernel
- Looking Glass client support, and host-side support where it belongs in Nex

A single booted root must activate exactly one Nvidia driver branch. Nex may
build both 580 and current driver branches in the repository, but it must not
install two active Nvidia branches into one root.

## Progress

- [x] (2026-06-29 11:21Z) Created this ExecPlan after the human clarified that
  Nvidia, Looking Glass, and v4l2loopback are required.
- [x] (2026-06-29 11:21Z) Recorded the chosen kernel-module policy: the kernel
  exports a module SDK, and out-of-tree module packages build against it.
- [x] (2026-06-29 11:35Z) Ran the Ralph worktree pre-task before
  implementation. Only local untracked notes and settings were present; no
  tracked commit candidate existed.
- [x] (2026-06-29 11:39Z) Created sub-EP
  `003a-kernel-module-sdk`.
- [x] (2026-06-29 13:04Z) Completed sub-EP `003a-kernel-module-sdk`.
  Commits: `2144ebe cli: categorize kernel module sdk outputs`,
  `54468e6 pkg/kernel: expose module sdk`.
- [x] (2026-06-29 13:15Z) Created sub-EP `003b-v4l2loopback`.
- [x] (2026-06-29 14:24Z) Completed sub-EP `003b-v4l2loopback`.
  Commit: `b19eb45 pkg/kernel: add v4l2loopback module`.
- [x] (2026-06-29 15:10Z) Created sub-EP
  `003c-nvidia-580-and-current`.
- [x] (2026-06-29 14:07Z) Completed sub-EP
  `003c-nvidia-580-and-current`. Commit:
  `22778f8 pkg: add nvidia driver branches`.
- [x] (2026-06-29 14:07Z) Created sub-EP `003d-looking-glass`.
- [x] (2026-06-29 14:07Z) Completed sub-EP `003d-looking-glass`.
  Commits: `dfdd2bf pkg: add spice protocol headers`,
  `79e152f pkg/kernel: add kvmfr module`, and
  `b22e6c2 pkg: add looking glass client`.
- [x] (2026-06-29 14:07Z) Created sub-EP
  `003e-final-driver-assembly-validation`.
- [x] (2026-06-29 16:12Z) Completed sub-EP
  `003e-final-driver-assembly-validation`.
  Commits: `50bb3fd pkg: include nvidia runtime metadata`,
  `92b24b9 asm: add nvidia desktop variants`.

## Surprises & Discoveries

- Observation: Two different Nvidia Linux driver branches cannot both be
  active for different Nvidia cards in one booted root.
  Evidence: Nvidia forum answers say Linux can use only one Nvidia GPU driver
  at a time, Nvidia documentation says kernel module flavors are mutually
  exclusive, and Nvidia userspace and kernel module versions must match.

- Observation: The current mixed GTX 1060 plus RTX 2070 machine needs the
  580-series proprietary Nvidia branch.
  Evidence: GTX 1060 is Pascal. Nvidia says 580 is the last Linux branch for
  Maxwell, Pascal, and Volta. RTX 2070 is Turing and can be served by a branch
  that still supports Turing.

- Observation: RTX 5090-class machines need a current branch, not 580.
  Evidence: Nvidia's current supported-products lists include RTX 5090, and
  Nvidia documents that Blackwell and later GPUs use the open kernel module
  flavor.

- Observation: Kernel output generation needs a dedicated `module-sdk`
  category for new SDK files.
  Evidence: `src/cli/src/outputs/mod.rs` preserves existing output paths
  during regeneration, while `src/cli/src/utils.rs` sends unknown
  `/usr/src/...` paths to `misc`.

- Observation: Both Nvidia branches now build as separate packages, but an
  assembly still must pick one branch.
  Evidence: `pkg/libs/graphics/nvidia-580.yaml` installs 580.159.04 modules
  with proprietary license metadata; `pkg/libs/graphics/nvidia-current.yaml`
  installs 595.84 open modules with `Dual MIT/GPL` license metadata. Both
  packages install the same module and userspace names.

- Observation: Looking Glass package-level parity is covered, but assemblies
  still need to include the client and `kvmfr`.
  Evidence: `pkg/apps/virt/looking-glass.yaml` builds B7 reproducibly and its
  help smoke prints `Looking Glass (B7)` under the Nex dynamic loader.
  `pkg/core/kernel/kvmfr.yaml` builds the host module and `modinfo` reports
  `6.12.58 SMP modversions`.

- Observation: Final Nvidia desktop roots now activate exactly one Nvidia
  branch while sharing the capture stack.
  Evidence: `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml` includes
  `nvidia-580`, v4l2loopback, `kvmfr`, and Looking Glass.
  `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml` includes
  `nvidia-current`, v4l2loopback, `kvmfr`, and Looking Glass. Checked-out
  roots showed Nvidia module versions `580.159.04` and `595.84` respectively,
  and each root lacked the other Nvidia package capsule.

- Observation: Nvidia runtime metadata must include the Nvidia `misc` output.
  Evidence: the first assembly smoke lacked Nvidia Vulkan ICD files until
  `pkg/libs/graphics/nvidia-580.yaml` and
  `pkg/libs/graphics/nvidia-current.yaml` added `misc` to the runtime bundle.

## Decision Log

- Decision: Use the kernel module SDK model for out-of-tree modules.
  Rationale: It keeps the kernel package clean while every external module
  proves it builds against the exact shipped kernel.
  Date/Author: 2026-06-29 / human and Carlos

- Decision: Build both Nvidia branches in Nex, but activate only one branch in
  each booted assembly.
  Rationale: The 580 branch is needed for Pascal hardware, and a current branch
  is needed for newer Blackwell hardware. Nvidia's Linux stack uses global
  module and userspace names, so one root cannot safely activate two branches.
  Date/Author: 2026-06-29 / human and Carlos

- Decision: Prefer a 580 proprietary kernel-module package for the mixed
  GTX 1060 plus RTX 2070 machine.
  Rationale: Nvidia says the open module flavor does not support pre-Turing
  GPUs, and GTX 1060 is Pascal.
  Date/Author: 2026-06-29 / Carlos

- Decision: Prefer the current open kernel-module flavor for RTX 5090-class
  assemblies.
  Rationale: Nvidia says Blackwell and later are only supported by open kernel
  modules.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

This ExecPlan is complete. Nex now builds a kernel module SDK, v4l2loopback,
Nvidia 580, Nvidia current, `kvmfr`, Looking Glass, and two desktop assembly
variants that activate one Nvidia branch at a time.

The final checked artifacts are:

- `desktop-vwl-nvidia-580`, checksum
  `448f89f4963c0587e935bfe1ebc3fe14529f68215222fb45ecc55eb4d7e04cff`
- `desktop-vwl-nvidia-current`, checksum
  `628e60ad927d94f3628735b7d7f445532b13799b4dd36cfd997db69f5660a558`

The 580 variant passed
`TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1
scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout 180
--headless` with `ASSERT-BOOT-PASS`.

The `nvidia-container-toolkit` parity row is no longer blocked on driver
policy, but Nex still needs a separate container hook package and proof. The
matrix now marks that row as `needs-manifest`.

## Context and Orientation

Read these files before work:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `RUST_CODE_STYLE.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/ostreefy-parity.md`

Current relevant manifests:

- `pkg/core/kernel/linux.yaml`
- `pkg/core/kernel/initramfs.yaml`
- `pkg/core/kernel/kmod.yaml`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-dev.yaml`

The parity matrix rows that this plan must replace are:

- `nvidia-580xx-dkms`
- `nvidia-container-toolkit`
- `looking-glass`
- `v4l2loopback-dkms`
- `v4l2loopback-utils`

## Plan of Work

1. Add or verify a `module-sdk` style kernel output or bundle from
   `pkg/core/kernel/linux.yaml`. It must contain the files needed for
   `make -C <kernel-build-tree> M=<module-source> modules` against the exact
   shipped kernel.
2. Build `v4l2loopback.ko` from source against that SDK. Package the CLI tools
   separately or as a bundle in the same manifest when upstream ships them
   together.
3. Package the Nvidia 580 branch for Pascal/Turing machines. Build or install
   the proprietary module flavor for the shipped kernel, and package matching
   userspace libraries and tools from the same release.
4. Package a current Nvidia branch for Blackwell/new machines. Build the open
   kernel module flavor against the shipped kernel, and package matching
   userspace libraries and tools from the same release.
5. Create explicit Nvidia assembly variants rather than putting both branches
   into `desktop-vwl` at once. The expected names can change if repo patterns
   suggest better names, but they must make the active branch obvious.
6. Package Looking Glass. Put the Linux client in the desktop assembly. Decide
   whether host-side pieces such as `kvmfr` belong in the Looking Glass package
   or in a separate out-of-tree module package. Prove the chosen command and
   module files exist.
7. Update `.agents/ostreefy-parity-matrix.md` and durable knowledge so these
   rows are no longer deferred policy rows.

## Concrete Steps

Use these commands as the baseline. Adjust paths only when implementation
chooses different manifest names.

```bash
./src/cli/target/debug/nex check pkg/core/kernel/linux.yaml
./src/cli/target/debug/nex build pkg/core/kernel/linux.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

./src/cli/target/debug/nex check pkg/core/kernel/v4l2loopback.yaml
./src/cli/target/debug/nex build pkg/core/kernel/v4l2loopback.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

./src/cli/target/debug/nex check pkg/graphics/nvidia/nvidia-580.yaml
./src/cli/target/debug/nex build pkg/graphics/nvidia/nvidia-580.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

./src/cli/target/debug/nex check pkg/graphics/nvidia/nvidia-current.yaml
./src/cli/target/debug/nex build pkg/graphics/nvidia/nvidia-current.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

./src/cli/target/debug/nex check pkg/apps/virt/looking-glass.yaml
./src/cli/target/debug/nex build pkg/apps/virt/looking-glass.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

Assembly checks should include both Nvidia branches:

```bash
./src/cli/target/debug/nex check asm/desktop-vwl-nvidia-580.yaml
./src/cli/target/debug/nex build asm/desktop-vwl-nvidia-580.yaml --verbose --check --update-checksum --force

./src/cli/target/debug/nex check asm/desktop-vwl-nvidia-current.yaml
./src/cli/target/debug/nex build asm/desktop-vwl-nvidia-current.yaml --verbose --check --update-checksum --force
```

Use `.agents/cleanup-workdirs.sh` before removing or recreating disposable
roots. Add new paths to that script before using them.

## Validation and Acceptance

This plan is complete only when all of these checks pass or a true hard blocker
is recorded:

- The kernel package exposes a module SDK that can build at least one external
  module.
- `v4l2loopback.ko` builds against the shipped kernel SDK. `modinfo` must show
  a vermagic string that matches the shipped kernel release.
- v4l2loopback user tools run a non-hardware smoke check, such as a version or
  help command.
- Nvidia 580 builds or installs modules for the shipped kernel and packages
  matching 580 userspace. The proof must show module version, vermagic, and
  `depmod` behavior in a checked-out root.
- Nvidia current builds open modules for the shipped kernel and packages
  matching userspace. The proof must show module version, vermagic, and
  `depmod` behavior in a checked-out root.
- The 580 and current assemblies do not both activate in the same root. Each
  branch has its own assembly or clear activation boundary.
- Looking Glass client command runs a version or help smoke. If host-side
  module support is packaged, its `.ko` also passes a vermagic check.
- `desktop-vwl-nvidia-580` and `desktop-vwl-nvidia-current` build
  reproducibly.
- A checked-out root for each assembly contains the expected kernel modules,
  modprobe files, userspace tools, Vulkan or GL vendor files, and Looking Glass
  or v4l2loopback files.
- The final boot or direct initramfs check proves at least one Nvidia-enabled
  root still enters the Nex boot path. If QEMU cannot prove real Nvidia module
  loading because no device exists, record the gap and prove all offline module
  and root filesystem checks.

## Idempotence and Recovery

The zub store is a cache. If a package build leaves stale scratch roots, add
their paths to `.agents/cleanup-workdirs.sh` and rerun the build from scratch.
Do not manually remove disposable paths with ad hoc `rm -rf`.

If an Nvidia release is superseded while this plan runs, keep the chosen 580
branch pinned for Pascal support. For the current branch, use the latest branch
that supports RTX 5090 and works with the shipped kernel, then record the
chosen version and source URL in the sub-EP.

If a kernel update changes `uname -r`, rebuild every out-of-tree module package
and every Nvidia assembly before committing.

## Artifacts and Notes

External facts used when creating this plan:

- Nvidia says Linux 580 is the last branch for Maxwell, Pascal, and Volta.
- Nvidia says open kernel modules support Turing and newer, and Blackwell and
  later are only supported by open kernel modules.
- Nvidia says open and proprietary kernel module flavors are mutually
  exclusive in one kernel and should not both be installed as active files.
- Nvidia forum answers say Linux can use only one Nvidia GPU driver at a time.
- Nvidia forum answers say kernel modules and userspace libraries must match
  exactly.

Useful source pages:

- `https://nvidia.custhelp.com/app/answers/detail/a_id/3142/~/support-timeframes-for-unix-legacy-gpu-releases`
- `https://download.nvidia.com/XFree86/Linux-x86_64/570.144/README/kernel_open.html`
- `https://docs.nvidia.com/datacenter/tesla/driver-installation-guide/latest/kernel-modules.html`
- `https://forums.developer.nvidia.com/t/two-driver-versions-under-linux/71327`
- `https://forums.developer.nvidia.com/t/driver-library-version-mismatch/75276`
