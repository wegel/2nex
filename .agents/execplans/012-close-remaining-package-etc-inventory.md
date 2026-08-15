# Explain every remaining package-owned `/etc` path

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this work, every package manifest that currently declares a file below
`/etc` will have an explicit, tested reason for its layout. Programs that own
vendor defaults will find those defaults below `/usr`, temporary machine
overrides below `/run`, and lasting administrator choices below `/etc`.
Packages will stop creating product accounts or importing Buildroot, Yocto,
Edgebox, or another product's policy. Files whose external specification
requires `/etc` will remain there only when the evidence supports that choice.

The plan covers all 31 package manifests left by the 2026-08-14 output scan.
This is one large audit and implementation batch, but not one large commit.
Each package or tightly coupled package group must pass its own behavior test
and strict two-build check before its commit lands. A reader can inspect the
final inventory in this plan and rerun the package, assembly, and boot checks
named below.

## Progress

- [x] (2026-08-15 00:27Z) Audited the clean worktree, read the repository
  rules, listed `.agents/knowledge/`, and read the package, assembly,
  reproducibility, builder, and agent workflow notes that touch this plan.
- [x] (2026-08-15 00:47Z) Removed Netavark's Buildroot-specific firewall
  choice, corrected its package version to match both pinned 1.14.1 sources,
  strictly rebuilt it and all five direct or inherited assemblies, and tested
  the installed executable in flat and Nex-structured roots.
- [x] (2026-08-15 01:00Z) Retained the OpenCL specification's fixed Nvidia
  ICD path, exposed it through both driver runtime bundles, strictly rebuilt
  both packages and images, and called the OpenCL loader in each image.
- [ ] Freeze the exact 31-manifest inventory and record every installed
  `/etc` path, reader, override, reload path, upstream vendor-path feature,
  and governing external specification.
- [ ] Resolve the vendor-data, XDG, example, compatibility-link, and database
  group, with one checked commit per package or inseparable package pair.
- [ ] Give D-Bus, Slang, and Libvirt correct vendor-file lookup without
  weakening their administrator paths or explicit overrides.
- [ ] Resolve OpenSSL, CA certificates, Fontconfig, Linux-PAM secondary files,
  and the account databases with focused security and trust tests.
- [ ] Audit both bootstrap manifests and keep only paths required to build the
  next phase in their private bootstrap roots.
- [ ] Audit every affected assembly overlay, rebuild every affected root
  twice, run focused root tests, and boot the affected Nex system in QEMU.
- [ ] Rerun exhaustive package and assembly scans, record every justified
  remaining `/etc` path, promote durable knowledge, and complete the human
  review gate.

## Surprises & Discoveries

- Observation: A literal search finds `/etc` in build-time tests and scratch
  build-root setup after a package stops declaring `/etc` output.
  Evidence: `pkg/cli/net/openssh.yaml` still creates temporary passwd, group,
  and SSH files to test its reader, but its generated outputs contain no
  package-owned SSH default below `/etc`. Count output entries, not every
  build-script string, for the final package inventory.

- Observation: The RTK `find` wrapper omits GNU `find` features needed for the
  knowledge and ExecPlan listings.
  Evidence: `rtk find ... -printf` rejected `-printf` and compound predicates;
  `rtk proxy find ... -printf` returned the exact filenames.

- Observation: Netavark's only `/etc` output came entirely from the Nex build
  script and even carried another distribution's name.
  Evidence: `pkg/apps/containers/netavark.yaml` wrote
  `50-buildroot-nftables.conf` itself. Both pinned source inputs are version
  1.14.1, while the old package metadata and assembly refs said 1.14.0.

- Observation: A sparse root can run Netavark directly but cannot initialize
  Podman's engine far enough for `podman info`.
  Evidence: both flat and Nex-structured assembly roots printed `netavark
  1.14.1`; the flat root's `podman info` stopped with `Error: no such file or
  directory` even with a private VFS root, runroot, and mounted `/proc`.

- Observation: Both Nvidia packages generated their OpenCL ICD correctly but
  omitted the `conf` output from both public bundles.
  Evidence: the strict package roots contained `nvidia.icd` and
  `libOpenCL.so.1`, while the old `full` and `runtime` bundle lists contained
  `config` but not `conf`. The two Nvidia assembly roots lacked the ICD until
  this audit added `conf` to both bundles.

## Decision Log

- Decision: Cover all 31 remaining manifests in this ExecPlan.
  Rationale: The human asked for at least 20 items per plan and approved the
  full remaining package inventory. One plan also lets the final scan prove
  that no package escaped classification.
  Date/Author: 2026-08-15 / Codex

- Decision: Land small, independently useful commits inside this large plan.
  Rationale: This branch will merge without squashing. Every commit must build,
  pass the changed behavior test, and remain a useful `git bisect` point.
  Date/Author: 2026-08-15 / Codex

- Decision: Require an evidenced result instead of forcing the final count to
  zero.
  Rationale: Some external loaders and specifications may require a stable
  `/etc` path. Moving such a file without checking its consumer would break a
  normal package. The final acceptable state has zero unexplained entries,
  even if a small justified set remains.
  Date/Author: 2026-08-15 / Codex

- Decision: Keep product choices in assemblies and generic source changes in
  packages.
  Rationale: A Nex package must remain useful in flat roots, Nex-structured
  roots, and third-party assemblies. A package patch may implement normal
  Linux `/etc`, `/run`, and `/usr` lookup, but it may not name Nex or encode a
  jukebox, desktop, installer, Yocto, or Buildroot choice.
  Date/Author: 2026-08-15 / Codex

- Decision: Package Netavark without a default firewall driver.
  Rationale: Netavark upstream does not install the removed fragment. The Nex
  manifest created the whole `50-buildroot-nftables.conf` file and therefore
  imported another distribution's product choice. Podman and each assembly
  can choose a firewall driver when they need one.
  Date/Author: 2026-08-15 / Codex

- Decision: Retain Nvidia's `/etc/OpenCL/vendors/nvidia.icd` and expose it in
  both public bundles.
  Rationale: Khronos defines `/etc/OpenCL/vendors` as the Linux ICD directory
  in `https://registry.khronos.org/OpenCL/specs/unified/refpages/man/html/cl_khr_icd.html`.
  Moving this file would make the package incompatible with conforming ICD
  loaders. A public runtime bundle must contain it so an installed loader can
  discover the packaged vendor library.
  Date/Author: 2026-08-15 / Codex

## Outcomes & Retrospective

Not started.

## Context and Orientation

UAPI.6 gives programs three places for machine-wide configuration. A program
uses lasting administrator files from `/etc`, temporary machine files from
`/run`, and package defaults from `/usr`. For one main file, the first existing
file in that order wins. An existing empty file masks lower files. For a
drop-in directory, the program sorts filenames, lets a higher-priority tree
shadow the same basename in a lower tree, and treats an empty file or a link
to `/dev/null` as a mask when its format allows that behavior.

`PHILOSOPHY.md` defines that model and keeps persistent host `/etc` outside a
read-only Nex deployment. `.agents/MANIFESTS_CODE_STYLE.md` requires normal,
reusable upstream packages and reviewable patches. `.agents/TESTING.md`
requires built behavior tests rather than path-existence checks alone.
`.agents/kb.md` and `.agents/knowledge/package-manifests.md` record the first
UAPI package waves. `.agents/knowledge/system-assemblies.md` explains how to
inspect split package outputs and Nex-structured roots. The ignored
`tmp/UAPI_TODO.md` mirrors the live checklist for this shared worktree, but
this tracked plan contains the authoritative scope.

The 31 manifest rows are:

| # | Manifest | Initial file class to verify |
|---:|---|---|
| 1 | `pkg/apps/containers/netavark.yaml` | Buildroot-named containers policy |
| 2 | `pkg/apps/graphics/imagemagick.yaml` | XML policy and type data |
| 3 | `pkg/apps/misc/ca-certificates.yaml` | generated trust links and bundle |
| 4 | `pkg/apps/security/gnome-keyring.yaml` | XDG autostart files |
| 5 | `pkg/apps/terminal/foot.yaml` | system XDG configuration |
| 6 | `pkg/bootstrap/phase0/toolchain.yaml` | bootstrap RPC database |
| 7 | `pkg/bootstrap/phase1/glibc.yaml` | bootstrap NSS, RPC, and loader cache |
| 8 | `pkg/cli/shells/bash-completion.yaml` | compatibility links and profile file |
| 9 | `pkg/core/ipc/dbus.yaml` | system and session main files |
| 10 | `pkg/core/userland/2nex-utilities.yaml` | passwd and group host state |
| 11 | `pkg/desktop/wayland/fuzzel.yaml` | system XDG configuration |
| 12 | `pkg/desktop/wayland/swaync.yaml` | XDG configuration, schema, and style |
| 13 | `pkg/desktop/wayland/waybar.yaml` | XDG configuration and style |
| 14 | `pkg/dev/libs/openssl3.yaml` | OpenSSL configuration and helper scripts |
| 15 | `pkg/dev/virt/libvirt.yaml` | daemon, client, network, filter, QEMU, and lock files |
| 16 | `pkg/libs/audio/pipewire.yaml` | PAM limits fragment |
| 17 | `pkg/libs/crypto/p11-kit.yaml` | example configuration |
| 18 | `pkg/libs/graphics/at-spi2-core.yaml` | XDG autostart file |
| 19 | `pkg/libs/graphics/fontconfig.yaml` | vendor aliases and local font policy |
| 20 | `pkg/libs/graphics/gtk3.yaml` | input-method data |
| 21 | `pkg/libs/graphics/nvidia-580.yaml` | OpenCL loader ICD |
| 22 | `pkg/libs/graphics/nvidia-current.yaml` | OpenCL loader ICD |
| 23 | `pkg/libs/net/libnl.yaml` | class and packet-location databases |
| 24 | `pkg/libs/net/libtirpc.yaml` | netconfig and reserved-port data |
| 25 | `pkg/libs/security/linux-pam.yaml` | secondary PAM module files |
| 26 | `pkg/libs/system/attr.yaml` | xattr policy |
| 27 | `pkg/libs/system/fuse3.yaml` | example or local policy |
| 28 | `pkg/libs/text/vte.yaml` | shell profile fragments |
| 29 | `pkg/libs/tui/slang.yaml` | slsh main file |
| 30 | `pkg/net/firewall/iptables.yaml` | ethertypes database |
| 31 | `pkg/net/firewall/nftables.yaml` | OS fingerprint database |

For each row, add an audit result to `Artifacts and Notes`. The result must
name all declared `/etc` outputs and choose exactly one primary outcome:

1. Move a vendor default below `/usr` through an existing upstream option.
2. Add a generic source patch that reads `/etc`, then `/run`, then `/usr`.
3. Remove true host state or product policy from the package and create the
   needed initial state in the relevant assembly.
4. Move a sample to `/usr/share/doc/<package>` or another normal example path.
5. Retain an externally fixed `/etc` path and cite the loader, specification,
   or upstream contract that requires it.

When one manifest contains several file classes, record and test one outcome
for each class. Empty directories for administrator configuration do not count
as vendor files, but the audit must say why the package creates them.

## Plan of Work

### Milestone 1: freeze the readers and standards map

Generate the 31-manifest list from declared output paths and compare it with
the table above. Inspect each manifest's build script and output groups.
Inspect the pinned upstream source for every program that reads one of those
files. Record command-line arguments, environment variables, reload paths,
include rules, vendor-directory support, and file-format ordering. Check the
authoritative upstream documentation or external specification for loader
paths such as OpenCL ICDs and trust databases.

Do not edit a parser until the notes name all readers of its packaged files.
Do not change `--sysconfdir=/etc` mechanically: that option often tells a
program where the administrator writes local files.

### Milestone 2: resolve vendor data, XDG files, examples, links, and databases

Start with packages whose consumers already search a standard vendor data or
XDG directory. Move package defaults through upstream build options when they
exist. Move genuine examples below `/usr/share/doc`. Remove the
Buildroot-named Netavark fragment unless an upstream-neutral package default
has a documented purpose. Check both Nvidia manifests together against the
OpenCL loader contract, but preserve separate reproducible package commits if
their artifacts differ.

Each commit must include a focused test that invokes the installed consumer or
parses the installed data through the real library. A link-only test is not
enough when the loader can exercise the file.

### Milestone 3: convert the remaining main-file readers

Give D-Bus, Slang, and Libvirt a normal vendor path. Preserve explicit file
arguments, environment overrides, user files, administrator fragment
directories, reload behavior, and parser ordering. Prefer a released upstream
feature. When the pinned release lacks it, carry a generic patch beside the
manifest with the header and zero-fuzz rules from
`.agents/MANIFESTS_CODE_STYLE.md`.

Test every tier, an empty higher-tier mask, same-basename drop-in shadowing
where supported, explicit overrides, and reload behavior where supported.
Start a built daemon and query its real interface when a service supplies the
only meaningful proof.

### Milestone 4: resolve trust, authentication, font policy, and accounts

Treat OpenSSL, CA certificates, Fontconfig, and Linux-PAM as security-sensitive
packages. Preserve `OPENSSL_CONF`, OpenSSL module lookup, `/etc/ssl` as an
administrator interface, certificate update behavior, Fontconfig's include
graph, PAM module semantics, and every supported local override.

Remove `passwd` and `group` creation from `2nex-utilities`. Put each product's
initial accounts in its assembly or a reusable assembly overlay, and prove
that flat and Nex-structured roots still resolve required users and groups.
Package manifests must not choose product users, passwords, shells, or IDs.

### Milestone 5: audit bootstrap-only paths

Inspect `pkg/bootstrap/phase0/toolchain.yaml` and
`pkg/bootstrap/phase1/glibc.yaml` as bootstrap programs, not normal runtime
packages. Record which next-phase command consumes each `/etc` file and whether
the file leaves the private bootstrap root. Reuse the final Glibc source
behavior where practical. Retain a bootstrap-local path only when a concrete
next-phase tool requires it and no built package exposes it to assemblies.

### Milestone 6: rebuild assemblies and close the scans

Audit these overlays and every child that inherits them:

- `asm/nex-systemd-overlay.yaml`
- `asm/edgebox-rootfs-overlay.yaml`
- `asm/desktop-vwl/desktop-vwl-overlay.yaml`
- `asm/installer/installer-overlay.yaml`

Classify their `/etc` files as initial host state or misplaced vendor policy.
Keep assembly choices explicit. Rebuild every assembly whose selected package
ref or initial host state changed. Test relevant commands and services inside
checked-out roots. Run the Edgebox smoke, desktop and installer checks, and a
direct QEMU boot for `nex-systemd` when their roots changed.

Finally, regenerate both package-output and assembly-overlay inventories.
Every remaining path must have an evidence entry in this plan. The final scan
must contain zero unexplained package-owned `/etc` files and zero product
choices in reusable package manifests.

## Concrete Steps

Run commands from `/var/home/wegel/work/wegelcorp/nex`. Every shell command in
this worktree starts with `rtk`; use `rtk proxy` when RTK shadows a required
GNU command feature.

Start or resume with:

    rtk git status --short --untracked-files=all
    rtk proxy find .agents/knowledge -maxdepth 1 -type f -printf '%f\n' | sort
    rtk semeja search 'relevant package or reader concept' .

For every changed package manifest, run:

    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./src/cli/target/debug/nex check <manifest>
    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

The build command performs two builds because `--check` compares their
checksums. Run the focused behavior test from the package build root or a
checked-out package capsule. Record the exact command and its asserted output
under `Artifacts and Notes` before committing.

For a changed assembly, run:

    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build <assembly> --single --check --update-checksum --verbose

Check out a Nex-structured root with:

    rtk /home/wegel/work/perso/zub/target/debug/zub --repo .nex/repo checkout --copy systems/<slug>/<version> <temporary-directory>

Enter it with `unshare --user --map-root-user --mount --pid --fork chroot` when
absolute `/nex/pkg` links or root identity matter. Use the repository's
existing Edgebox, desktop, installer, and QEMU scripts that the audit finds;
record their exact paths before the first assembly commit.

Before every commit:

    rtk git diff --check
    rtk git status --short --untracked-files=all
    rtk git diff --cached --check
    rtk git diff --cached

Stage exact files only. Push each clean, checked commit to the current upstream
branch because the human authorized branch pushes during this work.

## Validation and Acceptance

This ExecPlan is complete only when all of these statements hold:

- `Artifacts and Notes` contains a final result for all 31 manifest rows and
  names every declared `/etc` path in each row.
- Every changed manifest passes `nex check`, the strict two-build command, and
  a test that exercises its installed consumer or service.
- Every parser change proves `/usr`, `/run`, and `/etc` priority, masks,
  explicit overrides, and live reload where those features apply.
- Every retained `/etc` file cites an authoritative loader, specification, or
  upstream contract. The final inventory contains no unexplained entry.
- No changed package manifest or package patch contains Nex, Edgebox, Soniq,
  Yocto, Buildroot, or another product's policy.
- Every affected assembly builds twice with one checksum and passes a root,
  service, installer, or boot test that can fail when the changed behavior is
  broken.
- The final Edgebox smoke passes if Edgebox changes. The desktop and installer
  runtime checks pass if their roots change. A changed `nex-systemd` root boots
  in QEMU and prints `ASSERT-BOOT-PASS`.
- `.agents/kb.md`, `.agents/SCRATCH_KNOWLEDGE.md`, the relevant files under
  `.agents/knowledge/`, and `tmp/UAPI_TODO.md` agree with the final evidence.
- Every commit on this branch remains a clean, buildable mainline commit.

### Completion Check

Not started. Before human review, compare every acceptance item above with the
finished commits and record exact commands, checksums, assertions, skipped
checks, and remaining risks here.

## Idempotence and Recovery

The strict package and assembly commands may be rerun. They refresh checksums,
profiles, outputs, and dependency maps, so inspect all generated manifest
changes before staging. Always pass `--single` to avoid refreshing an unrelated
dependency graph.

If a build stops, inspect its retained `.nex/tmp/build_rootfs_*` root and log,
fix the package, then rerun the same strict command. Do not commit a stale
checksum or one successful half of a two-build check. If a broad assembly build
reveals a missing or stale package ref, rebuild and test that package in its
own commit before resuming the assembly. Preserve unrelated human edits and
leave ignored local artifacts unstaged.

## Artifacts and Notes

### Initial inventory

The prior completed parser wave reduced the declared package-output inventory
from 35 to 31. The 31 rows in `Context and Orientation` are the frozen starting
set. `tmp/UAPI_TODO.md` has the longer historical 41-row list and records which
ten manifests earlier plans already cleared.

### Per-manifest results

Add one numbered result for each inventory row here. Each result must name the
paths, readers, chosen outcome, focused behavior test, strict build checksum,
affected assemblies, and commit.

1. `pkg/apps/containers/netavark.yaml`

   The old `conf` output declared
   `/etc/containers/containers.conf.d/50-buildroot-nftables.conf`. Podman reads
   that fragment as administrator or distribution policy; Netavark itself
   does not install or read it. Outcome 3 applies: the package no longer
   creates the Buildroot-named choice, and an assembly can add a firewall
   driver when its product policy needs one. Both pinned upstream inputs and
   the installed executable identify version 1.14.1, so the manifest and its
   two assembly refs now use 1.14.1.

   The strict package command built twice with checksum
   `c648f6c25191dabf9ec46602309ef5ea112e2ecce44c3570241383915374c56c`.
   The package build script also asserts that its new binary reports 1.14.1.
   `unshare --user --map-root-user --mount --pid --fork chroot ...
   /usr/bin/netavark --version` printed `netavark 1.14.1` in both the
   `flat-podman` and `desktop-dev` roots. Exact-file scans found no
   `50-buildroot-nftables.conf` in either root. A YAML assertion found no
   declared `/etc` output or `buildroot` text in the package manifest.

   The affected assemblies reproduced with these checksums: `flat-podman`
   `f8feefae4d82452da5dec9f6fe69e5d65c008a7e6a34273184838f0fe9bd31c9`,
   `desktop-vwl`
   `43515783ba6831b2f1bdbca64b32f85f13595e53b126c89a9d9506060c00cbc4`,
   `desktop-vwl-nvidia-580`
   `fc237af4027a16ad8cb4333803ac36eeac5f8635f80b6f60493c55c027fb4c76`,
   `desktop-vwl-nvidia-current`
   `63328e5de39d6649842c7b486398b03b5f5d94ace843c2edbbff1efb4ff62f4d`,
   and `desktop-dev`
   `3f45970d91af64016e788e93897e9ded96aa05d650764f7ded9e83bc664653ab`.
   Commit: `pkg: package netavark without distribution policy`.

21. `pkg/libs/graphics/nvidia-580.yaml`

   The `conf` output declares `/etc/OpenCL/vendors/nvidia.icd`, whose content
   names `libnvidia-opencl.so.1`. Outcome 5 applies. The Khronos ICD extension
   reference fixes this directory on Linux, so the manifest retains the path.
   The package already generated the file but neither public bundle selected
   `conf`; `full` and `runtime` now include it.

   The strict package command built twice with checksum
   `58b3a59dc14fcc68b8d755b4a153d7b5e489548e2b803f4a7f1cc85897d2885a`.
   A C smoke program linked the packaged `libOpenCL.so.1`, called
   `clGetPlatformIDs`, and returned the expected `-1001` without a GPU. In the
   rebuilt image, the same call loaded the Nvidia ICD far enough to attempt
   the Nvidia kernel module before returning `-1001`. The image placed the ICD
   at `/usr/share/factory/etc/OpenCL/vendors/nvidia.icd` and reproduced with
   checksum
   `e833ab46831dfa9cc16b17c784f147c88b23567b4b98b9a03c7bf9f5b085ef88`.
   Commit: `pkg: include nvidia OpenCL ICDs at runtime`.

22. `pkg/libs/graphics/nvidia-current.yaml`

   This branch declares the same `/etc/OpenCL/vendors/nvidia.icd` path and
   names the same `libnvidia-opencl.so.1` soname. Outcome 5 and the Khronos
   evidence above apply. Its `full` and `runtime` bundles now include `conf`.

   The strict package command built twice with checksum
   `1bf7490765156c555325cfd1efa3ff26466e3f20af023677085a88f057b97b06`.
   The same installed-library test called `clGetPlatformIDs`; the assembly
   call attempted to load the Nvidia module and returned the expected `-1001`
   on this non-Nvidia test host. The image placed the ICD in its factory tree
   and reproduced with checksum
   `43c20f709c15622e278ae0f1bf486be49f1ae578d377d6586d47498b91fc0a57`.
   Commit: `pkg: include nvidia OpenCL ICDs at runtime`.

## Interfaces and Dependencies

This plan may change the 31 package manifests listed above, local patch files
beside those manifests, and assembly manifests or overlays that must own host
state. It does not add a Nex-only package configuration API. Programs keep
their normal command-line and environment interfaces.

The package build interface is `./nex build <manifest> --verbose --single
--check --update-checksum --force --compute-deps --record-profile
--generate-outputs`. The assembly build interface is `./nex build <assembly>
--single --check --update-checksum --verbose`. Both use
`/home/wegel/work/perso/zub/target/debug/zub` through `ZUB_BIN`.

Any new local source patch must be a tracked `file:` source with a lowercase
SHA-256, a complete patch header, explicit application order, and
`patch --batch --fuzz=0`. Online patches may remain URLs only when their host
should retain them for at least as long as the pinned target archive.
