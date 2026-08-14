# Package v4l2loopback

This sub-ExecPlan belongs to
`.agents/execplans/003-nvidia-looking-glass-v4l2loopback.md`.

## Purpose / Big Picture

Nex needs the virtual V4L2 camera module and the small upstream control tools
that users expect from v4l2loopback. The package must build
`v4l2loopback.ko` against the exact Nex Linux 6.12.58 module SDK, then install
the module under `/usr/lib/modules/6.12.58/extra` so assemblies can layer it
with the shipped kernel.

## Progress

- [x] (2026-06-29 13:08Z) Ran the Ralph worktree pre-task. Only local
  untracked notes and settings were present; no tracked commit candidate
  existed.
- [x] (2026-06-29 13:09Z) Listed `.agents/knowledge/` and read kernel,
  package, ostreefy parity, and reproducibility notes.
- [x] (2026-06-29 13:12Z) Verified upstream tags from the primary GitHub repo;
  `v0.15.4` is the newest tag seen.
- [x] (2026-06-29 13:54Z) Created `pkg/core/kernel/v4l2loopback.yaml`.
- [x] (2026-06-29 14:16Z) Built the package with strict reproducibility
  checks. Final checksum:
  `0de68c202bab6676701eb3717d92a7debe34b87c6d1b532663155bd31b305b19`.
- [x] (2026-06-29 14:19Z) Smoke tested the checked-out runtime bundle.
  `modinfo -F vermagic` printed `6.12.58 SMP modversions`, and
  `v4l2loopback-ctl --help` printed the expected command names.
- [x] (2026-06-29 14:23Z) Committed the checked package manifest and standard
  environment fix as `b19eb45 pkg/kernel: add v4l2loopback module`.

## Surprises & Discoveries

- Observation: v4l2loopback ships both the kernel module and user tools in one
  upstream source tree.
  Evidence: upstream tag list and README source inspection during this plan.

- Observation: v0.15.4 through v0.13.0 reference V4L2 helpers that this
  Linux 6.12.58 SDK does not export.
  Evidence: the v0.15.4 build failed on unresolved `v4l2_fill_pixfmt_mp` and
  `v4l2_format_info`, and tag archive checks showed those helper names in
  v0.15.4, v0.15.3, v0.15.2, v0.15.1, v0.15.0, v0.14.0, v0.13.2,
  v0.13.1, and v0.13.0.

- Observation: v0.12.7 avoids those missing helpers but uses `strlcpy`, which
  Linux 6.12 removed.
  Evidence: v0.12.7 failed first with an implicit `strlcpy` declaration, then
  built after the package script replaced `strlcpy(` with `strscpy(`.

- Observation: Running `depmod` inside the v4l2loopback package creates
  partial module metadata.
  Evidence: the build generated a `lib` output containing
  `/usr/lib/modules/6.12.58/modules.*` files even though the output contained
  only the external module, not the full kernel module tree.

## Decision Log

- Decision: Build v4l2loopback as a static per-kernel package, not as DKMS.
  Rationale: The human chose the Nex out-of-tree module policy where packages
  build against the kernel SDK and assemblies install built module files.
  Date/Author: 2026-06-29 / human and Carlos

- Decision: Package v4l2loopback 0.12.7 for the current Linux 6.12.58 kernel.
  Rationale: It is the newest checked upstream tag that avoids V4L2 helper
  symbols missing from the SDK's `Module.symvers`.
  Date/Author: 2026-06-29 / Carlos

- Decision: Do not ship package-local `depmod` metadata from v4l2loopback.
  Rationale: A partial `modules.dep` can overwrite the kernel package's full
  metadata when an assembly layers packages into one root. A later assembly
  step should regenerate module metadata after all external modules are
  present.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

The package manifest builds v4l2loopback 0.12.7 against the Nex Linux 6.12.58
module SDK. The package script applies the Linux 6.12 `strscpy` compatibility
edit, installs `v4l2loopback.ko` and the `v4l2loopback-ctl` Bash tool, and does
not ship partial `depmod` metadata.

## Concrete Steps

1. Download and inspect v4l2loopback `v0.15.4`.
2. Add a package manifest under `pkg/core/kernel/v4l2loopback.yaml`.
3. Depend on `x86_64/pkg/core/kernel/linux/6.12.58/bundles/module-sdk`.
4. Build the module with:

```bash
make KERNEL_DIR=/usr/src/linux-6.12.58 KERNELRELEASE=6.12.58
```

5. Install `v4l2loopback.ko` under `/usr/lib/modules/6.12.58/extra`.
6. Install upstream tools such as `v4l2loopback-ctl` when the release builds
   them in this environment.
7. Run:

```bash
./src/cli/target/debug/nex check pkg/core/kernel/v4l2loopback.yaml
./src/cli/target/debug/nex build pkg/core/kernel/v4l2loopback.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

8. Check out the package and prove:

```bash
modinfo -F vermagic <checkout>/usr/lib/modules/6.12.58/extra/v4l2loopback.ko
<checkout>/usr/bin/v4l2loopback-ctl --help
```

## Validation and Acceptance

This sub-plan is complete when:

- `v4l2loopback.ko` builds against the Nex 6.12.58 SDK.
- `modinfo -F vermagic` starts with `6.12.58`.
- The user tools run a non-hardware smoke check such as `--help`.
- The manifest passes `nex check` and the strict package build.
