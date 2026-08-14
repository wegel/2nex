# Final Driver Assembly Validation

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex now builds the package pieces for the required GPU and capture stack:
Nvidia 580, current Nvidia, v4l2loopback, Looking Glass, and `kvmfr`. This
sub-EP must make those pieces useful at the assembly layer without putting two
Nvidia branches into one booted root.

The expected result is two clear Nvidia desktop assembly variants:

- one root activates the 580 branch for the current GTX 1060 plus RTX 2070
  machine
- one root activates the current branch for newer RTX 5090-class machines

Both roots should include the common capture stack: Looking Glass client,
`kvmfr`, and v4l2loopback. The proof should build the assemblies and inspect
checked-out roots for modules, userspace commands, vendor files, and depmod
metadata behavior. At least one root must still pass the smallest available
boot-path check.

## Progress

- [x] (2026-06-29 14:07Z) Created this sub-EP after package-level work for
  Nvidia, v4l2loopback, Looking Glass, and `kvmfr` passed.
- [x] (2026-06-29 14:30Z) Ran the Ralph worktree pre-task for this sub-EP.
  Only local notes and settings were dirty or untracked; no coherent tracked
  commit candidate existed.
- [x] (2026-06-29 14:36Z) Inspected existing assembly manifests and package
  inclusion patterns.
- [x] (2026-06-29 14:41Z) Added assembly manifests for Nvidia 580 and
  Nvidia current variants under `asm/desktop-vwl/`.
- [x] (2026-06-29 14:41Z) Included common capture packages in both variant
  roots: v4l2loopback, `kvmfr`, and Looking Glass.
- [x] Build both assembly variants reproducibly.
  - [x] (2026-06-29 14:48Z) `desktop-vwl-nvidia-580` built
    reproducibly with checksum
    `ade7b8fe3ef959949707dfe6d6375f2117925c810c504fd36f28eb4c769c361c`.
  - [x] (2026-06-29 14:53Z) `desktop-vwl-nvidia-current` built
    reproducibly with checksum
    `14ee559fe290a626e5a30a1a5a1911ec1a2e8ad3da68cadd4fccb6764ec58715`.
- [x] (2026-06-29 15:26Z) Fixed Nvidia runtime bundles to include `misc` so
  the system roots include Nvidia Vulkan ICD and implicit layer JSON files.
- [x] (2026-06-29 15:35Z) Rebuilt `desktop-vwl-nvidia-580` reproducibly with
  checksum
  `448f89f4963c0587e935bfe1ebc3fe14529f68215222fb45ecc55eb4d7e04cff`.
- [x] (2026-06-29 15:43Z) Rebuilt `desktop-vwl-nvidia-current` reproducibly
  with checksum
  `628e60ad927d94f3628735b7d7f445532b13799b4dd36cfd997db69f5660a558`.
- [x] (2026-06-29 15:47Z) Formatted both new assembly manifests and reran
  `nex check`; both passed.
- [x] (2026-06-29 16:01Z) Checked out fresh roots for both assembly variants
  and proved each selected Nvidia branch, v4l2loopback, `kvmfr`, Looking
  Glass, and Nvidia vendor files exist and run the expected help or metadata
  checks.
- [x] (2026-06-29 16:05Z) Ran the direct-initramfs boot assertion against
  `TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1`; QEMU printed
  `ASSERT-BOOT-PASS`.
- [x] (2026-06-29 16:08Z) Committed the checked assembly manifests.
  Commit: `92b24b9 asm: add nvidia desktop variants`.
- [x] (2026-06-29 16:12Z) Updated the main EP and durable knowledge.

## Surprises & Discoveries

- Observation: This checkout has the style files at the repository root, not
  under `.agents/`.
  Evidence: reading `MANIFESTS_CODE_STYLE.md` at the root passed; reading
  `.agents/MANIFESTS_CODE_STYLE.md` failed with no such file.

- Observation: Child assembly manifests can inherit the parent build script
  with an empty `script:` block.
  Evidence: `src/cli/src/manifest/inheritance.rs` uses the parent script when
  the child script is empty, and both new Nvidia variant manifests pass
  `nex check`.

- Observation: Nvidia Vulkan files live in the Nvidia `misc` output, not in
  `config`.
  Evidence: before the runtime bundle fix, `desktop-vwl-nvidia-580` exposed
  `/usr/share/glvnd/egl_vendor.d/10_nvidia.json` but lacked
  `/usr/share/vulkan/icd.d/nvidia_icd.json`; adding `misc` to the Nvidia
  runtime bundles made both rebuilt roots expose the ICD and implicit layer
  JSON files.

- Observation: Host-side `test -e` gives false negatives for many public
  paths in checked-out system roots because those paths are absolute symlinks
  into `/nex/pkg`.
  Evidence: host `test -e
  .nex/tmp/desktop-vwl-nvidia-580-smoke/usr/bin/nvidia-smi` failed, while
  `unshare --user --map-root-user --root
  .nex/tmp/desktop-vwl-nvidia-580-smoke /usr/bin/test -e
  /usr/bin/nvidia-smi` passed.

- Observation: Looking Glass B7 help must run as a non-root user.
  Evidence: root execution printed `Do not run looking glass as root!`;
  `unshare --map-user=1000 --map-group=1000 --root <root>
  /usr/bin/looking-glass-client --help` printed `Looking Glass (B7)`.

## Decision Log

- Decision: Keep Nvidia 580 and Nvidia current in separate assembly variants.
  Rationale: Nvidia module and userspace names overlap, and package-level work
  already proved that one booted root must activate exactly one Nvidia branch.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

This sub-EP is complete. Nex now has two desktop assembly variants:
`desktop-vwl-nvidia-580` and `desktop-vwl-nvidia-current`. Each root activates
one Nvidia branch and also includes v4l2loopback, `kvmfr`, and Looking Glass.

The package-level runtime metadata fix was required before the assembly roots
could pass vendor-file smoke tests. The final 580 root also passed the
direct-initramfs boot assertion.

## Context and Orientation

Read these files before work:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/ostreefy-parity.md`
- `.agents/knowledge/reproducibility.md`

Relevant package manifests:

- `pkg/libs/graphics/nvidia-580.yaml`
- `pkg/libs/graphics/nvidia-current.yaml`
- `pkg/core/kernel/v4l2loopback.yaml`
- `pkg/core/kernel/kvmfr.yaml`
- `pkg/apps/virt/looking-glass.yaml`

Relevant assembly manifests to inspect:

- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-dev.yaml`
- any existing assembly variant manifests under `asm/`

## Plan of Work

1. Run the worktree pre-task. Leave local notes and settings uncommitted.
2. Inspect assembly schema and existing package inclusion patterns.
3. Choose assembly names that make the active Nvidia branch obvious and match
   repo conventions.
4. Add the 580 assembly variant. It must include the base desktop, Nvidia 580,
   v4l2loopback, `kvmfr`, and Looking Glass.
5. Add the current assembly variant with the current Nvidia branch and the same
   common capture stack.
6. Build both assemblies with the relevant `nex check` and `nex build`
   commands. Use the current built CLI.
7. Check out each assembly root and assert:
   - the selected Nvidia module files exist and report the expected version and
     vermagic
   - the unselected Nvidia branch is absent from that root
   - `v4l2loopback.ko` and `kvmfr.ko` exist and report
     `6.12.58 SMP modversions`
   - `looking-glass-client` exists
   - Nvidia userspace tools and vendor files exist
8. Run the smallest boot-path check that works for a built Nvidia-enabled root.
   If QEMU cannot prove hardware module loading, record that gap and keep the
   offline root checks concrete.

## Concrete Steps

Start with these commands and adjust after inspecting the assembly names:

```bash
git status --short --untracked-files=all
./src/cli/target/debug/nex check asm/desktop-vwl-nvidia-580.yaml
./src/cli/target/debug/nex build asm/desktop-vwl-nvidia-580.yaml --verbose --check --update-checksum --force
./src/cli/target/debug/nex check asm/desktop-vwl-nvidia-current.yaml
./src/cli/target/debug/nex build asm/desktop-vwl-nvidia-current.yaml --verbose --check --update-checksum --force
```

For root checks, prefer checked-out assembly roots and direct commands such as:

```bash
modinfo -F version <root>/usr/lib/modules/6.12.58/extra/nvidia.ko
modinfo -F vermagic <root>/usr/lib/modules/6.12.58/extra/nvidia.ko
test -x <root>/usr/bin/nvidia-smi
test -x <root>/usr/bin/looking-glass-client
test -f <root>/usr/lib/modules/6.12.58/extra/v4l2loopback.ko
test -f <root>/usr/lib/modules/6.12.58/extra/kvmfr.ko
```

## Validation and Acceptance

This sub-EP is complete when:

- both Nvidia assembly manifests pass `nex check`
- both Nvidia assemblies build reproducibly
- root checks prove each root has exactly its chosen Nvidia branch and the
  common capture stack
- at least one boot-path check still reaches the Nex boot assertion, or the EP
  records a concrete reason that the available QEMU check cannot boot the new
  root yet
- durable knowledge records any new assembly or smoke-test facts

## Idempotence and Recovery

Use `.agents/cleanup-workdirs.sh` for disposable roots. Add any new smoke or
checkout paths there before using them.

If an assembly build uses dirty source snapshots, commit coherent tracked
changes before trusting the assembly checksum.
