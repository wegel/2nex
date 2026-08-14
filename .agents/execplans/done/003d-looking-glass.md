# Package Looking Glass Client And Host Support

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex must package Looking Glass so the desktop image can run the Linux client
for guest display capture. If upstream still ships the `kvmfr` host kernel
module, Nex must package that module against the Linux 6.12.58 module SDK or
record a concrete reason why this repo should not ship it.

The user should be able to build a Looking Glass package, check out its runtime
bundle, and run a non-hardware command such as `looking-glass-client --help`.
If the package includes `kvmfr`, `modinfo` must show a vermagic string that
matches Linux 6.12.58.

## Progress

- [x] (2026-06-29 14:07Z) Created this sub-EP after completing Nvidia branch
  packaging.
- [x] (2026-06-29 14:07Z) Checked upstream tags. `git ls-remote --tags
  https://github.com/gnif/LookingGlass.git 'refs/tags/*'` listed `B7` as the
  newest non-rc release tag.
- [x] (2026-06-29 14:07Z) Ran the Ralph worktree pre-task. Only untracked
  local notes and settings were present; no tracked commit candidate existed.
- [x] (2026-06-29 14:07Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`. Read relevant notes by theme and searched them for
  Looking Glass, `kvmfr`, module SDK, CMake, and Wayland.
- [x] (2026-06-29 14:07Z) Added cleanup paths for Looking Glass inspection and
  smoke roots before extracting source archives.
- [x] (2026-06-29 14:07Z) Inspected the `B7` source tree and recorded client
  feature choices, PureSpice requirements, and the host `kvmfr` module path.
- [x] (2026-06-29 14:07Z) Record exact client and host build
  inputs.
- [x] (2026-06-29 14:07Z) Added and proved
  `pkg/dev/virt/spice-protocol.yaml` because Looking Glass PureSpice requires
  `spice-protocol.pc`. The strict package build passed reproducibly with
  checksum `a0451d04ab4aec6e7e958304ce15a32d3908b8295f66197f08362e62c8b17c32`;
  `pkgconf --modversion spice-protocol` reported `0.14.5` from the checked-out
  dev bundle.
- [x] (2026-06-29 14:07Z) Add `pkg/apps/virt/looking-glass.yaml` with
  Wayland/EGL/PipeWire support and build it reproducibly.
- [x] (2026-06-29 14:07Z) Added `pkg/apps/virt/looking-glass.yaml`. The
  strict two-pass package build passed with checksum
  `65c5066d414cb87fc33596c0acd39ecb2e9e266ed87e31abfd8bb4ef57078232`.
- [x] (2026-06-29 14:07Z) Upstream ships `kvmfr`, so Nex packages it as a
  standalone kernel-module package rather than mixing it into the client
  package.
- [x] (2026-06-29 14:07Z) Added and proved
  `pkg/core/kernel/kvmfr.yaml` for Looking Glass host-side shared-memory
  support. The strict package build passed reproducibly with checksum
  `2614bb26b92913318921fea27be8e9eab7b7008fc56b3ce705b42ecbdb991b8d`.
  The runtime smoke showed module version `0.0.12`, vermagic
  `6.12.58 SMP modversions`, and license `GPL v2`.
- [x] (2026-06-29 14:07Z) Smoked the manually layered Looking Glass runtime
  root under the Nex dynamic loader as UID 65534. The command printed
  `Looking Glass (B7)` and the complete option tables without missing library
  errors. The client returns 255 after printing help, so the output is the
  proof.
- [x] (2026-06-29 14:07Z) Reran final checks before the client manifest
  commit: `nex check` passed, the strict two-pass build passed again, and the
  help smoke assertion found `Looking Glass (B7)` and `complete list of
  options accepted` in the help log.
- [x] (2026-06-29 14:07Z) Smoked the `kvmfr` module output earlier in this
  sub-EP with `modinfo`; it reported version `0.0.12`, vermagic
  `6.12.58 SMP modversions`, and license `GPL v2`.
- [ ] Update the main EP and durable knowledge when the package work is
  checked.
- [x] (2026-06-29 14:07Z) After the sandbox restart, resumed and completed the
  client smoke without using `nex install --flat`.

## Surprises & Discoveries

- Observation: Looking Glass currently uses release tags named `B1` through
  `B7`, with rc tags between some releases.
  Evidence: `git ls-remote --tags https://github.com/gnif/LookingGlass.git
  'refs/tags/*'`.

- Observation: The official B7 source archive includes the submodule contents
  that a plain GitHub tag archive or non-recursive clone does not include.
  Evidence: `tar -tf /tmp/LookingGlass-B7.tar.gz` showed files under
  `repos/LGMP`, `repos/PureSpice`, `repos/cimgui`, `repos/nanosvg`, and
  `repos/wayland-protocols`.

- Observation: Looking Glass B7 needs `spice-protocol.pc` even when the package
  focuses on the Linux client.
  Evidence: `repos/PureSpice/CMakeLists.txt` requires pkg-config modules
  `spice-protocol`, `nettle`, and `hogweed`.

- Observation: Looking Glass B7 `kvmfr` builds cleanly against the Nex Linux
  6.12.58 module SDK.
  Evidence: the strict `pkg/core/kernel/kvmfr.yaml` build passed
  reproducibly, and `modinfo` on the checked-out runtime bundle showed
  version `0.0.12` and vermagic `6.12.58 SMP modversions`.

- Observation: `nex install --dry-run --flat` is not safe as a smoke-test
  helper in this environment.
  Evidence: running it for `pkg/apps/virt/looking-glass.yaml bundles/runtime`
  printed `Checking out to /...` and checked out dependency files. After that,
  command execution failed with `/usr/bin/bash: /usr/lib/libc.so.6: version
  'GLIBC_2.42' not found (required by /usr/lib/libncursesw.so.6)`.

## Decision Log

- Decision: Start from upstream release tag `B7`.
  Rationale: It is the newest non-rc tag observed from the upstream Git tags,
  and Nex should prefer a released source over an rc or moving branch.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

Completed package-level Looking Glass work. Nex now has a reproducible
Looking Glass B7 client package, a reproducible `spice-protocol` package, and
a reproducible `kvmfr` out-of-tree module package. Assembly inclusion and boot
validation remain in the main EP.

## Context and Orientation

Read these files before work:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/reproducibility.md`

Relevant nearby manifests:

- `pkg/apps/virt/virt-manager.yaml`
- `pkg/dev/virt/qemu.yaml`
- `pkg/apps/misc/remmina.yaml`
- `pkg/apps/graphics/vulkan-tools.yaml`
- `pkg/desktop/wayland/wl-mirror.yaml`
- `pkg/core/kernel/v4l2loopback.yaml`

## Plan of Work

1. Run the worktree pre-task and leave unrelated local notes/settings
   uncommitted.
2. Add Looking Glass scratch paths to `.agents/cleanup-workdirs.sh`, then use
   that script before any extraction or smoke checkout.
3. Inspect the `B7` source tree to find the client build system, optional
   features, host tools, and `kvmfr` module path.
4. Create `pkg/apps/virt/looking-glass.yaml` for the Linux client. Prefer the
   upstream build system and disable features only when Nex lacks the required
   dependency and the feature is not needed for a useful client.
5. If `kvmfr` is a standalone host kernel module, create a package such as
   `pkg/core/kernel/kvmfr.yaml` that builds against
   `x86_64/pkg/core/kernel/linux/6.12.58/bundles/module-sdk`.
6. Generate outputs and runtime dependency metadata through the strict package
   build command.
7. Smoke the checked-out package root. The client smoke must execute the
   installed command and assert version or help output. The module smoke must
   run `modinfo -F vermagic` and check for `6.12.58 SMP modversions`.

## Concrete Steps

Use these commands as the baseline and adjust only after source inspection
shows the exact package paths:

```bash
git status --short --untracked-files=all
bash .agents/cleanup-workdirs.sh
git clone --depth 1 --branch B7 https://github.com/gnif/LookingGlass.git .nex/tmp/looking-glass-inspect

./src/cli/target/debug/nex check pkg/apps/virt/looking-glass.yaml
./src/cli/target/debug/nex build pkg/apps/virt/looking-glass.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

zub -r .nex/repo checkout x86_64/pkg/apps/virt/looking-glass/B7/bundles/runtime .nex/tmp/looking-glass-smoke -f
.nex/tmp/looking-glass-smoke/usr/bin/looking-glass-client --help
```

If the plan adds a `kvmfr` module package, also run:

```bash
./src/cli/target/debug/nex check pkg/core/kernel/kvmfr.yaml
./src/cli/target/debug/nex build pkg/core/kernel/kvmfr.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
zub -r .nex/repo checkout x86_64/pkg/core/kernel/kvmfr/B7/bundles/runtime .nex/tmp/kvmfr-smoke -f
modinfo -F vermagic .nex/tmp/kvmfr-smoke/usr/lib/modules/6.12.58/extra/kvmfr.ko
```

## Validation and Acceptance

This plan is complete when all checks pass:

- `nex check` passes for every added manifest.
- The strict two-pass package build passes for every added package.
- The checked-out Looking Glass runtime bundle runs the client help or version
  command without missing loader or library errors.
- If Nex packages `kvmfr`, `modinfo` shows vermagic
  `6.12.58 SMP modversions`.
- The main EP records whether Looking Glass host-side support lives in the
  client package, a kernel package, or a later assembly layer.

## Idempotence and Recovery

The zub store is a cache. If inspection or builds leave large scratch roots,
add those paths to `.agents/cleanup-workdirs.sh` and run that script. Do not
remove disposable workdirs with ad hoc `rm -rf`.

If a dependency is missing, add it explicitly to the manifest and record why.
The builder will compute runtime library metadata after install, but build-time
tools, headers, and pkg-config providers must be listed by hand.

## Artifacts and Notes

Primary upstream source:

- `https://github.com/gnif/LookingGlass`
