# Remove package-owned `/etc` compatibility files

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this work, packages maintained by the Nex distribution will not install
vendor defaults or integration fragments below `/etc` or `/usr/etc`. Programs
built by Nex will read immutable defaults below `/usr`, temporary settings below
`/run`, and lasting administrator settings below `/etc`. The desktop, Edgebox,
and other affected systems will continue to load autostart entries, shell hooks,
Libvirt integration, OpenCL drivers, and package configuration from the new
paths.

An assembly that adds an unpatched outside program may still create the exact
compatibility path that program needs. No current assembly will retain a
compatibility path merely because upstream software historically installed one.
`CONFIGURATION.md` defines this contract and explains how `/etc` behaves across
upgrades and rollbacks.

The known starting inventory contains 28 paths across 13 existing manifests:
15 paths below `/etc` and 13 paths below `/usr/etc`. This plan resolves all of
them in one implementation batch, with small checked commits for independent
package groups.

## Progress

- [x] (2026-08-15 12:55Z) Wrote and committed `CONFIGURATION.md` before
  implementation, linked it from `README.md`, and aligned `PHILOSOPHY.md` with
  the package and assembly rules in commit `6cc913d`.
- [x] (2026-08-15 13:09Z) Ran the Ralph worktree pre-task against a clean
  tracked tree, listed all ten durable knowledge files, and read every
  note that touches package configuration, assembly factory state, package
  testing, and reproducibility.
- [x] (2026-08-15 13:16Z) Froze 28 paths across 13 manifests, added a
  self-tested repository checker for `/etc` and `/usr/etc` output declarations,
  and documented the rule for manifest authors. The checker correctly reports
  all 28 known violations until later milestones remove them.
- [x] (2026-08-15 13:24Z) Moved Bash Completion, VTE, Elfutils, and Rust shell
  files into `/usr/lib/profile.d` or `/usr/share`, removed their nine package
  paths below `/etc` and `/usr/etc`, and ran their executable package smokes.
  The remaining inventory has 19 paths across nine manifests.
- [x] (2026-08-15 13:57Z) Published
  `XDG_CONFIG_DIRS=/etc/xdg:/run/xdg:/usr/share/xdg` from the desktop's
  `/usr/lib/environment.d`, built the Desktop VWL assembly twice at checksum
  `f3e6a13c0fc94b1f9dc4422bc85657e817610c39d0023bfd5311f00481ea2dad`,
  and ran the installed Systemd environment and autostart generators in its
  finished root. The reader found all three package entries, selected
  administrator then transient then vendor files with the same basename, and
  honored an administrator `Hidden=true` mask.
- [x] (2026-08-15 13:39Z) Added the assembly-owned `/usr/lib/profile` reader,
  layered fragment basenames from `/usr/lib/profile.d`, `/run/profile.d`, and
  `/etc/profile.d`, and proved transient and administrator replacement plus
  `/dev/null` masking in a finished Edgebox root. Two strict commands
  reproduced Flat Systemd checksum
  `1212c60ff0f42d8f5910cae5644c08f8d23bbd7af9d2f1c2247932b253028ba8`
  and Edgebox checksum
  `bef9bc7ebe717608fc48b5c261e802c973c8b6836f4be35c2f5a4f634bc78841`.
  The finished-root test also loaded the installed Bash Completion hook and
  preserved caller shell settings while ignoring a neighboring `.csh` hook.
- [x] (2026-08-15 13:50Z) Moved the Gnome Keyring and AT-SPI2 autostart entries
  from `/etc/xdg/autostart` to `/usr/share/xdg/autostart`. Their strict package
  builds asserted the installed `Type=` and `Exec=` fields and reproduced
  checksums `066579bd01075af3d1214709aca0d8d10055d9865faa279a91691a4e91fde9cd`
  and `c4cfc4e2f63e73d1ba383cc68f48e114051ceb13795472c11a3bc3b54acb3499`.
  The remaining inventory has 16 paths across seven manifests. The desktop
  assembly work in the preceding checkpoint closes the XDG milestone.
- [ ] Move Libvirt's Logrotate and OpenSSH fragments into vendor trees, add the
  missing Nex-built Logrotate reader, and exercise both integrations.
- [ ] Replace Nvidia's binary generic OpenCL loader with a source-built Khronos
  loader that reads all three configuration tiers, then move both Nvidia ICD
  files below `/usr`.
- [ ] Replace the remaining `/usr/etc` outputs from Tig, Wget, OSTree, CUPS, and
  Rust with vendor data, patched readers, or explicit integration templates.
- [ ] Rebuild every affected assembly twice and exercise shell, desktop,
  Libvirt, OpenCL, printing, network download, Edgebox, installer, and boot
  behavior in finished roots.
- [ ] Run the final package and factory-tree scans, update knowledge and the UAPI
  checklist, complete the human review gate, and stop for approval.

## Surprises & Discoveries

- Observation: The previous package scan counted only paths rooted directly at
  `/etc`.
  Evidence: a literal output scan after EP012 found another 13 declared paths
  below `/usr/etc` in Rust, Elfutils, Tig, Wget, OSTree, and CUPS. Nex has chosen
  `/usr/lib` or `/usr/share` for vendor files, so `/usr/etc` does not fit the
  distribution contract even though the UAPI specification permits an
  implementation to choose it.

- Observation: The Nvidia archives contain both the proprietary ICD and a
  generic OpenCL loader.
  Evidence: `pkg/libs/graphics/nvidia-install-runtime.sh` currently installs
  `libnvidia-opencl.so` and Nvidia's `libOpenCL.so.1.0.0`, while both Nvidia
  manifests publish that loader and `/etc/OpenCL/vendors/nvidia.icd`. A
  source-built generic loader can replace only `libOpenCL`; the proprietary ICD
  remains the provider.

- Observation: Systemd already installs
  `/usr/lib/systemd/user-generators/systemd-xdg-autostart-generator`, and the
  desktop packages already use the standard `XDG_CONFIG_DIRS` interface.
  Evidence: `pkg/core/init/systemd.yaml` publishes the generator, while the
  Gnome Keyring and AT-SPI2 manifests own the three retained autostart files.
  An assembly can add the vendor XDG directory to `XDG_CONFIG_DIRS` without a
  Nex-only application patch.

- Observation: Nex does not currently package Logrotate or GRUB.
  Evidence: no manifest under `pkg/` declares either slug. Libvirt nevertheless
  publishes four Logrotate fragments, and OSTree publishes a GRUB integration
  script. This plan adds a source-built Logrotate reader because the desktop can
  exercise those fragments. It keeps the GRUB script as an immutable OSTree
  integration template because current Nex systems use the Nex boot path.

- Observation: The tracked worktree was clean at the EP013 pre-task.
  Evidence: `rtk git status --short --untracked-files=all` printed no paths at
  commit `0873019`, so there were no human changes, local-only files, or checked
  commit candidates to classify.

- Observation: Rust 1.91.1's bundled LLVM needs Zlib, but the old manifest did
  not select Zlib or record `libz.so.1` in its generated dependency lists.
  Evidence: the new `rustc --version` package smoke first failed while loading
  `libz.so.1`; adding the normal Zlib development bundle made both `cargo
  --version` and `rustc --version` pass, and dependency generation recorded
  Zlib for all three affected LLVM consumers.

- Observation: Nex has no C shell package that can execute the installed VTE
  and Elfutils `.csh` hooks.
  Evidence: the package manifest scan found no `csh` or `tcsh` slug. The package
  smokes require each C shell hook to be nonempty and execute the sibling Bash
  hook. The final report must retain this narrow skipped semantic check.

- Observation: A noninteractive Bash login shell discards an inherited `PS1`
  and supplies `TERM=dumb` when the caller omits `TERM`.
  Evidence: the first finished-root profile test saw `# |dumb` after the vendor
  profile ran, but could not pass a custom `PS1` through a second Bash process.
  The final test sources `/usr/lib/profile` with explicit shell variables to
  prove it preserves them and separately proves a login shell preserves an
  explicit `TERM=xterm-256color`.

- Observation: AT-SPI2's package smoke needed an explicit Grep build tool.
  Evidence: its first strict build relocated the desktop file successfully but
  stopped at `grep: command not found`; adding the normal Grep development
  bundle let the exact `Type=` and `Exec=` assertions run in both builds.

## Decision Log

- Decision: Treat `/usr/etc` package output as part of this cleanup.
  Rationale: It stores vendor data under `/usr`, but it preserves a misleading
  second `etc` hierarchy and evades the path rule instead of following it.
  Date/Author: 2026-08-15 / Carlos

- Decision: Reject package output declarations below both `/etc` and `/usr/etc`
  with a repository check, not a manifest-schema restriction.
  Rationale: The Nex distribution can enforce its package policy without
  forbidding an independent manifest repository from making a different
  deliberate choice.
  Date/Author: 2026-08-15 / Carlos

- Decision: Patch or configure every reader that Nex ships before adding a
  compatibility path.
  Rationale: Nex aims to ship a complete distribution. Packages can place
  defaults below `/usr` when every installed reader follows the UAPI directory
  order.
  Date/Author: 2026-08-15 / Carlos

- Decision: Let assemblies own compatibility adapters for unpatched outside
  programs.
  Rationale: An assembly knows whether it contains the outside consumer. A
  reusable package should not create compatibility state for consumers that may
  not exist.
  Date/Author: 2026-08-15 / Carlos

- Decision: Use `XDG_CONFIG_DIRS` to expose `/usr/share/xdg` to desktop sessions
  and keep `/etc/xdg` plus `/run/xdg` ahead of it.
  Rationale: XDG already defines the environment interface. One assembly-owned
  setting serves the Systemd autostart generator and other compliant readers
  without patching each desktop program.
  Date/Author: 2026-08-15 / Carlos

- Decision: Put vendor shell hooks below `/usr/lib/profile.d` and ordinary Bash
  completions below `/usr/share/bash-completion`.
  Rationale: Systemd already installs its own shell hook under
  `/usr/lib/profile.d`, and Bash Completion already uses `/usr/share` for its
  main script and command completions. The assembly's vendor profile can source
  the three profile fragment tiers.
  Date/Author: 2026-08-15 / Carlos

- Decision: Let Bash choose its terminal default and keep the vendor profile
  silent about `TERM`.
  Rationale: Bash already supplies `dumb` when no terminal exists, while a real
  terminal or caller provides the correct value. A vendor profile cannot infer
  a better terminal type and must not replace a caller's choice.
  Date/Author: 2026-08-15 / Carlos

- Decision: Add a source-built Logrotate package and teach it the three-tier
  main-file and fragment lookup.
  Rationale: Moving Libvirt fragments without a reader would preserve files but
  would not prove that the integration works. A complete Nex desktop should not
  depend on an outside Logrotate binary.
  Date/Author: 2026-08-15 / Carlos

- Decision: Build the Khronos OpenCL ICD Loader from source and stop publishing
  Nvidia's copy of the generic loader.
  Rationale: Nex cannot patch Nvidia's binary loader. The Khronos loader has
  source and tests, while Nvidia's proprietary `libnvidia-opencl.so` remains a
  normal ICD behind it.
  Date/Author: 2026-08-15 / Carlos

- Decision: Do not add compatibility adapters to current assemblies unless a
  finished-root behavior test proves that a current unpatched consumer needs
  one.
  Rationale: The current package graph should use Nex-built readers. Future
  product assemblies can add narrow adapters when they add outside programs.
  Date/Author: 2026-08-15 / Carlos

- Decision: Keep the commits suitable for a normal, unsquashed merge and push
  every checked commit to the current upstream branch.
  Rationale: The human explicitly requires the branch history to land as-is and
  has authorized pushes from this repository.
  Date/Author: 2026-08-15 / Carlos

## Outcomes & Retrospective

Not started. At completion, record the final package-path count, each package
and assembly checksum, the reader behavior proved in built roots, every adapter
that remains, and every skipped hardware or graphical check.

## Context and Orientation

`CONFIGURATION.md` is the normative design for this plan. `PHILOSOPHY.md`
states the same package rule at a higher level. `.agents/MANIFESTS_CODE_STYLE.md`
requires reusable, product-neutral manifests and visible generic patches.
`.agents/TESTING.md` requires strict package builds plus tests that execute the
changed readers. EP012 is archived at
`.agents/execplans/done/012-close-remaining-package-etc-inventory.md` and
contains the original reader audit and finished assembly checks.

The 15 direct `/etc` paths are:

- `pkg/apps/security/gnome-keyring.yaml`: two files below
  `/etc/xdg/autostart`.
- `pkg/libs/graphics/at-spi2-core.yaml`: one file below
  `/etc/xdg/autostart`.
- `pkg/cli/shells/bash-completion.yaml`: `/etc/bash_completion`, one
  compatibility fragment below `/etc/bash_completion.d`, and one hook below
  `/etc/profile.d`.
- `pkg/libs/text/vte.yaml`: two hooks below `/etc/profile.d`.
- `pkg/dev/virt/libvirt.yaml`: four fragments below `/etc/logrotate.d` and one
  fragment below `/etc/ssh/ssh_config.d`.
- `pkg/libs/graphics/nvidia-580.yaml` and
  `pkg/libs/graphics/nvidia-current.yaml`: one
  `/etc/OpenCL/vendors/nvidia.icd` file each, installed by
  `pkg/libs/graphics/nvidia-install-runtime.sh`.

The 13 `/usr/etc` paths are:

- `pkg/dev/util/elfutils.yaml`: two profile hooks.
- `pkg/dev/lang/rust-bin.yaml`: one Cargo completion and one target schema.
- `pkg/dev/vcs/tig.yaml`: one system Tig configuration file.
- `pkg/cli/net/wget.yaml`: one system Wget configuration file.
- `pkg/core/fs/ostree.yaml`: one GRUB integration script.
- `pkg/libs/printing/cups.yaml`: three CUPS configuration files and their three
  `.default` copies.

The likely supporting changes include:

- an assembly-owned XDG environment setting with
  `/etc/xdg:/run/xdg:/usr/share/xdg` in that order;
- a vendor `/usr/lib/profile` that sources the merged profile fragments;
- the existing OpenSSH vendor fragment reader from
  `pkg/cli/net/openssh-uapi-config.patch`;
- a new Logrotate manifest and generic UAPI patch;
- a new OpenCL Headers manifest if the current graph cannot supply the headers;
- a new Khronos OpenCL ICD Loader manifest and generic UAPI patch;
- package-local reader patches for Tig, Wget, and CUPS; and
- a shell check under `scripts/` that rejects package output declarations rooted
  below `/etc` or `/usr/etc`.

The affected assembly graph includes `asm/flat-systemd.yaml`,
`asm/nex-minimal.yaml`, `asm/nex-systemd.yaml`, `asm/installer.yaml`,
`asm/flat-podman.yaml`, `asm/edgebox-rootfs.yaml`,
`asm/desktop-vwl/desktop-vwl.yaml`, both Nvidia desktop children, and
`asm/desktop-dev.yaml`. The agent must use evidence from actual dependency refs
to narrow or expand this list before each assembly commit.

## Plan of Work

### Milestone 1: freeze and enforce the distribution inventory

Run the worktree pre-task from `AGENTS.md`. List `.agents/knowledge/`, read the
package and assembly notes, and copy candidate findings into
`.agents/SCRATCH_KNOWLEDGE.md`. Reproduce both path scans and record exact
manifest, output, and bundle ownership in this plan.

Add a small repository script that scans package YAML output declarations and
fails on `/etc`, `/etc/...`, `/usr/etc`, or `/usr/etc/...`. The script must print
the offending manifest and line. It must ignore source patches, test roots, and
reader strings because those may validly mention administrator paths. Add a
self-test with allowed and rejected sample manifests so the check can fail for
the intended reason. Add the same package rule to
`.agents/MANIFESTS_CODE_STYLE.md` so future manifest authors see it before the
script rejects their work.

### Milestone 2: move shell integration to vendor paths

Configure Bash Completion to keep its main script and command completions below
`/usr/share/bash-completion`. Move its login hook to `/usr/lib/profile.d` and
remove package-created compatibility files. Move both VTE and both Elfutils
hooks to `/usr/lib/profile.d`. Move Cargo's completion to
`/usr/share/bash-completion/completions/cargo` and its target schema to an
appropriate `/usr/share` data or documentation path.

Teach the assembly vendor profile to combine `/usr/lib/profile.d`,
`/run/profile.d`, and `/etc/profile.d` by basename, with `/etc` highest and
`/dev/null` masks. Preserve Bash's existing whole-file lookup for `/etc/profile`,
`/run/profile`, and `/usr/lib/profile`; an administrator who replaces the main
profile owns its contents. Prove a login shell loads Bash Completion and the VTE
hook, sees a transient fragment, honors an administrator replacement, and
honors a mask. Prove the Cargo completion registers for `cargo`. Parse or source
the C shell hooks with a Nex-built C shell if the package graph contains one;
otherwise record that narrow skipped check and validate the installed script
without adding a host dependency.

### Milestone 3: move XDG autostart data to the vendor tree

Move the three Gnome Keyring and AT-SPI2 desktop files to
`/usr/share/xdg/autostart`. Add an assembly-owned environment setting that gives
compliant readers `/etc/xdg`, `/run/xdg`, and `/usr/share/xdg` in priority order.
Use the existing Systemd user generator as the concrete reader. Its focused
test must show vendor entries, a transient same-name replacement, an
administrator replacement, and an administrator mask. Run a graphical QEMU
smoke after the desktop assembly selects the new package refs and prove that
the expected user units or daemons appear in the booted system.

### Milestone 4: give Libvirt integrations Nex-built readers

Move Libvirt's OpenSSH fragment to `/usr/lib/ssh/ssh_config.d`. Use the existing
OpenSSH UAPI reader to run `ssh -G` against a Libvirt URI or host pattern and
assert that the vendor proxy command appears, then assert that a transient or
administrator same-name fragment replaces it.

Add a source-built Logrotate package. Patch it to select a main file from
`/etc`, `/run`, and `/usr`, and to merge fragment directories at
`/etc/logrotate.d`, `/run/logrotate.d`, and `/usr/lib/logrotate.d` by basename.
Move Libvirt's four fragments to `/usr/lib/logrotate.d`. Run Logrotate against
temporary Libvirt logs and assert that vendor rules rotate them, that a
transient fragment replaces a vendor fragment, and that an administrator mask
disables one. Add Logrotate to the desktop only if the finished system uses the
fragments there; do not add its service policy to a reusable package.

### Milestone 5: remove the remaining `/usr/etc` package outputs

Patch Tig and Wget to select their main system file from `/etc`, `/run`, and a
normal vendor path below `/usr`. Preserve explicit command-line and environment
overrides. Exercise the installed readers with distinct values in every tier,
an empty mask, and the package default.

Patch CUPS so its daemon reads vendor defaults below `/usr`, accepts temporary
and administrator replacements, and keeps writable runtime and spool state out
of the deployment. Run `cupsd` configuration validation against every tier and
start it in an isolated built root far enough to query its real interface. Do
not confuse the `.default` copies with live administrator files.

Move OSTree's GRUB script to an immutable OSTree integration-template path.
Current Nex assemblies use the Nex bootloader, so they must not create a GRUB
compatibility link. Run the script with mocked GRUB helpers and assert the
generated entry or concrete side effect. A future GRUB assembly can add an
adapter when it selects a Nex-built or outside GRUB reader.

### Milestone 6: replace the unpatchable OpenCL loader

Add pinned source manifests for the Khronos OpenCL headers and ICD loader as
needed. Patch the loader so it merges `.icd` files from an administrator
directory, a temporary directory, and a vendor directory below `/usr`, with
same-name priority and masks. Preserve `OCL_ICD_FILENAMES` and
`OCL_ICD_VENDORS` as explicit overrides according to upstream behavior.

Change the shared Nvidia install script to place `nvidia.icd` below the vendor
directory and to omit Nvidia's generic `libOpenCL` loader while retaining
`libnvidia-opencl.so`. Update both Nvidia manifests and the desktop package
selection so the Khronos loader supplies `libOpenCL`. Compile a stub ICD in the
package smoke and prove the loader discovers it through each tier, respects a
mask, and reports a platform. In both finished Nvidia roots, prove that the
public `libOpenCL` comes from the source-built loader and that its vendor file
selects the matching proprietary ICD.

### Milestone 7: rebuild systems and close the audit

Relink only manifests whose refs changed. Build each affected assembly twice,
in parent-before-child order and sequentially for the large desktop systems.
Check out finished roots with the current Zub binary at
`/home/wegel/work/perso/zub/target/debug/zub`; do not rely on the stale host
`zub` command.

Run the Edgebox smoke, installer test, Systemd boot test, and graphical desktop
test where their package graphs changed. Exercise the changed commands and
readers in checked-out roots. Scan package output declarations again and scan
the finished factory tree so no moved package file survives through an old
bundle or assembly copy.

`scripts/qemu-test-graphical.sh` currently invokes `zub` by command name. Make
it honor the same `ZUB_BIN` override as the other QEMU scripts before relying on
it, add a shell-level check for that override, and use the current Zub binary in
the final graphical run.

Update `tmp/UAPI_TODO.md` and `.agents/SCRATCH_KNOWLEDGE.md` throughout the
work. Before the human gate, promote verified facts into the relevant
`.agents/knowledge/` files and record exact checksums and results below.

## Concrete Steps

Run all commands from the repository root. Every top-level shell command must
start with `rtk`; use `rtk proxy` when the RTK wrapper hides a required GNU
option.

Start with:

    rtk git status --short --untracked-files=all
    rtk proxy find .agents/knowledge -maxdepth 1 -type f -print
    rtk rg -n '^\s*- path: /etc(?:/|$)|^\s*- path: /usr/etc(?:/|$)' pkg --glob '*.yaml'

Build the CLI if the binary is missing or stale:

    rtk cargo build --manifest-path src/cli/Cargo.toml

For each changed existing or new package manifest, run `nex check`, then run
the strict command twice. Replace `<manifest>` with the exact path:

    rtk ./src/cli/target/debug/nex check <manifest>
    rtk ./src/cli/target/debug/nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

After the second build, run the focused installed-reader test named in the
matching milestone. Record the two output checksums and behavior assertions in
this plan before committing.

For every changed assembly, run `nex check`, relink its changed refs, and build
it twice with the same strict flags. Expected assembly manifests include:

    asm/flat-systemd.yaml
    asm/nex-minimal.yaml
    asm/nex-systemd.yaml
    asm/installer.yaml
    asm/flat-podman.yaml
    asm/edgebox-rootfs.yaml
    asm/desktop-vwl/desktop-vwl.yaml
    asm/desktop-vwl/desktop-vwl-nvidia-580.yaml
    asm/desktop-vwl/desktop-vwl-nvidia-current.yaml
    asm/desktop-dev.yaml

Use the current Zub binary for boot scripts:

    rtk proxy /home/wegel/work/perso/zub/target/debug/zub --repo .nex/repo checkout systems/edgebox-rootfs/0.0.1 .nex/tmp/ep013-edgebox-root
    rtk bash scripts/test-edgebox-rootfs.sh .nex/tmp/ep013-edgebox-root
    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub scripts/qemu-test-systemd.sh systems/nex-systemd/0.0.1 --timeout 120
    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub TARGET_REF=systems/desktop-vwl/0.0.1 scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --headless --timeout 180
    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300

Inspect every staged commit before committing:

    rtk git diff --cached --check
    rtk git diff --cached

Stage exact paths only. Use scoped imperative subjects. Push each clean commit
to the current branch after its integrated checks pass.

## Validation and Acceptance

The plan is complete only when all of the following statements are true:

- The repository path check passes and an exhaustive scan reports zero package
  output declarations rooted below `/etc` or `/usr/etc`.
- `.agents/MANIFESTS_CODE_STYLE.md` tells package authors to use `/usr/lib` or
  `/usr/share` and to put outside-program adapters in assemblies.
- Each of the 13 starting manifests passes `nex check`, builds reproducibly
  twice, and passes a focused test that executes its reader or integration.
- Every new Logrotate or OpenCL package builds reproducibly twice and its
  layered lookup test covers vendor, temporary, administrator, same-name
  replacement, and masking behavior.
- A login shell in a finished system loads vendor shell hooks while temporary
  and administrator fragments retain higher priority.
- The Systemd XDG autostart generator discovers all three moved desktop files,
  and the graphical smoke proves the finished desktop still starts the
  relevant accessibility and keyring components.
- OpenSSH reads Libvirt's vendor fragment, and the Nex-built Logrotate reader
  applies Libvirt's vendor rules to real temporary log files.
- Tig, Wget, and CUPS read their vendor defaults and honor `/run` and `/etc`.
- Both Nvidia desktop roots use the source-built Khronos `libOpenCL`, discover
  the matching vendor ICD below `/usr`, and contain no package-created
  `/etc/OpenCL` tree.
- Every affected assembly passes `nex check`, reproduces across two strict
  builds, and passes its named finished-root or QEMU test.
- The final desktop factory-tree scan explains every remaining `/etc` leaf and
  finds none copied from the 28 removed package paths.
- `git diff --check` passes, all checked commits are pushed, and the tracked
  worktree is clean.

### Completion Check

Not complete. Before requesting human approval, compare every acceptance item
with the final diff and commit list. Record the exact scan output, package and
assembly checksums, focused test assertions, boot markers, graphical result,
remaining factory paths, and every skipped hardware check here.

## Idempotence and Recovery

Strict package and assembly builds are safe to repeat. The Zub store is a cache;
Git remains the source of truth. Use fresh task-specific checkout directories
for focused root tests and remove only directories created by this plan after
their paths have been checked explicitly.

Large desktop assemblies must build sequentially. If a build fills `/tmp`,
identify only this plan's generated roots before deleting them, then resume with
the parent assembly. Do not remove unrelated user or agent work.

If a package checksum changes after a source or patch edit, rerun the strict
build twice and update its direct consumers before building assemblies. If an
assembly still exposes an old `/etc` path, inspect its pinned package ref and
selected output before changing overlay policy.

The path-check script should remain useful after this plan. If a future package
truly needs a fixed `/etc` interface for an outside binary, keep the package
data below `/usr` and put the adapter in the assembly. Do not weaken the scan
with a broad exception.

## Artifacts and Notes

- Design contract: `CONFIGURATION.md` at commit `6cc913d`.
- Prior complete audit:
  `.agents/execplans/done/012-close-remaining-package-etc-inventory.md`.
- Working checklist: `tmp/UAPI_TODO.md`, which is ignored and must not be
  committed.
- Candidate knowledge: `.agents/SCRATCH_KNOWLEDGE.md`, which is ignored until
  verified facts move into `.agents/knowledge/`.
- Initial inventory command:
  `rtk bash scripts/check-package-config-paths.sh`. It reports exactly 28 paths
  in 13 manifests and exits with failure while the known work remains.
- Checker proof: `rtk bash scripts/check-package-config-paths.sh --self-test`
  prints `PASS: package configuration path checker self-test`; `rtk bash -n
  scripts/check-package-config-paths.sh` also passes. This host does not have
  `shellcheck`, so the first commit uses Bash syntax checking and the executable
  self-test.
- The UAPI Group specification permits implementations to choose the precise
  vendor path below `/usr`; Nex deliberately standardizes its own manifests on
  `/usr/lib` and `/usr/share` rather than `/usr/etc`.
- The Khronos OpenCL ICD Loader source supports explicit
  `OCL_ICD_FILENAMES` and `OCL_ICD_VENDORS` overrides. The implementation must
  preserve those interfaces while adding the default layered directories.
- Shell package checksums after relocation: Bash Completion
  `b6771b9688f614b674fb9fb32d50d4254a7457043512df57f01760cab183695d`,
  VTE `cb779385060af4815061ca5517c6614504d1c7b50d6c2ac75f8ba7d8c0448c40`,
  Elfutils `8cc94c6a0b2ebf1a5ce0f444daec9f4f6a6ad764e9d9a10cfc721a8a08cda872`,
  and Rust
  `69ca52e02ece272a597586ca2a99e1b1e200b5056b528fd6a7e7e7b2ddc54302`.
  Each strict build command performed two matching builds. The embedded smokes
  loaded Bash Completion's default reader, activated VTE's interactive Bash
  hook, preserved Elfutils' explicit `DEBUGINFOD_URLS`, registered Cargo's
  completion, and ran both Cargo and Rustc. The same strict command was then
  repeated against each generated final manifest before commit.

## Interfaces and Dependencies

This plan changes package paths and reader behavior, but it does not add a Nex
configuration DSL or a general compatibility-manifest schema.

The public configuration contract remains:

- vendor files below `/usr/lib` or `/usr/share`;
- temporary overrides and generated views below `/run`;
- administrator files below `/etc`;
- service-owned persistent state below `/var/lib`; and
- explicit compatibility adapters in the assembly that adds an unpatched
  outside consumer.

The new repository check applies only to package output declarations under
`pkg/`. Assembly overlays may still create deliberate machine policy and
compatibility paths below `/etc`.

The new Logrotate package must expose the normal `logrotate` command and
configuration reader. The new OpenCL loader must provide the standard
`libOpenCL.so` ABI while Nvidia packages provide only their proprietary ICD and
vendor registration file. Record exact package versions, source URLs, hashes,
manifest refs, output refs, and assembly refs here when the implementation pins
them.
