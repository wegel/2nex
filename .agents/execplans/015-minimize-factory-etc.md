# Minimize factory `/etc` and prove upgrades

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this work, a fresh Nex-structured system will copy only genuine machine
state and required compatibility adapters into writable `/etc`. Packages and
assemblies will keep immutable distribution defaults below `/usr`, so a new
deployment will supply new defaults without rewriting the machine owner's
files.

A two-version boot test will prove the complete rule. Version 1 will seed a
fresh machine. The test will make administrator changes, boot version 2 with
changed vendor policy, and then roll back. New vendor defaults must follow the
selected immutable deployment. Administrator and machine choices must survive
both directions. No factory link may point at an exact package checksum that
disappears during an upgrade.

`CONFIGURATION.md` already states the public contract: `/usr` belongs to the
selected deployment, `/run` belongs to one boot, and writable `/etc` belongs to
the machine. This plan makes the current assemblies follow that contract and
adds a runtime test that can catch future regressions.

EP014 must finish first. EP014 removes personal accounts, credentials, site
network data, and test helpers from Desktop VWL and its Systemd base. This plan
then classifies what remains instead of preserving test policy as if it were
required machine state.

## Progress

- [x] (2026-08-16 02:31Z) Confirmed that EP014 is archived, ran the Ralph
  worktree pre-task, listed `.agents/knowledge/`, and read the configuration,
  assembly, installer, QEMU, package-testing, and reproducibility notes. The
  worktree was clean, so it had no pre-task commit candidates or local-only
  files to classify.
- [ ] Build or check out fresh roots for every Nex-structured assembly and
  record every factory `/etc` leaf by owner, type, source, and intended
  lifetime. Audit flat assemblies separately.
- [x] (2026-08-16 02:52Z) Added a self-tested finished-root checker and a
  machine-readable policy table. The checker emits one TSV row per accepted
  leaf and rejects unclassified files, wrong file types, wrong adapter
  targets, dangling immutable targets, and checksum-pinned `/nex/pkg` links.
- [ ] Move each immutable default to a standard vendor path below `/usr`, patch
  its reader when needed, and leave only true machine choices, Systemd
  enablement or masks, and unavoidable stable adapters in factory `/etc`.
- [ ] Remove stale release metadata and exact `/nex/pkg/.../<checksum>` targets
  from factory `/etc`; put custom unit definitions below
  `/usr/lib/systemd/system`.
- [ ] Define and test how a provisioner supplies hostname, local host entries,
  timezone, locale, console layout, filesystems, accounts, and network data
  without baking arbitrary choices into a reusable assembly.
- [ ] Add a two-version first-boot, upgrade, rollback, and fresh-install test
  that exercises real readers and persistent `/etc` state.
- [ ] Rebuild the full affected assembly graph twice, run finished-root and
  QEMU checks, publish the final factory inventory, promote durable knowledge,
  and stop for human review.

## Surprises & Discoveries

- Observation: `rtk test -f <path>` invokes the Bash test builtin without the
  file argument on this host and exits 2 after printing Bash help.
  Evidence: the prescribed EP014 dependency check failed that way, while
  `rtk stat .agents/execplans/done/014-make-desktop-vwl-generic.md` found the
  archived regular file. Use `rtk stat` for this repository file check.

- Observation: The current Desktop VWL deployment itself contains no active
  `/etc` files or links, but its factory tree still contains 88 leaves.
  Evidence: checkout of Desktop VWL checksum
  `50ed467e695adcb994ce924ad3ed5123d0feeb92ced1b0db828c7673165d7809`
  found 58 regular files, 30 links, and 31 directories below
  `/usr/share/factory/etc`. The stored deployment's `/etc` had only empty
  directories.

- Observation: EP014 reduced the Desktop VWL baseline from 88 factory leaves
  to 77, but it did not change the configuration ownership problem.
  Evidence: fresh checkouts under
  `.nex/tmp/ep015-baseline.ko361R` contain 27 leaves in Nex Systemd, 77 in
  Desktop VWL and Desktop Dev, 79 in each Nvidia child, five in the installer,
  and two in Nex Minimal. Desktop VWL has 51 regular files, 26 links, and
  14,568 apparent leaf bytes.

- Observation: Systemd's package output supplies six factory files, and the
  assembly's public tree turns them into package-checksum links.
  Evidence: `pkg/core/init/systemd.yaml` declares `issue`, `locale.conf`,
  `nsswitch.conf`, `pam.d/other`, `pam.d/system-auth`, and `vconsole.conf`
  below `/usr/share/factory/etc`. The new checker rejected the corresponding
  links in every Systemd-based fresh root.

- Observation: Systemd 257.5 already installs its service-specific PAM files
  below `/usr/lib/pam.d`, while its six factory files are samples or host
  policy rather than required runtime files.
  Evidence: the finished package contains `systemd-user` and `systemd-run0`
  below `/usr/lib/pam.d`. Removing `/usr/share/factory/etc` left both installed
  service files intact, and four strict builds reproduced checksum
  `0b01357e5cb84af8c29e4da21b1075c0a576159cf4dbf0846984b0ffe233d165`.

- Observation: the current first-boot path already preserves machine-owned
  files during every deployment switch.
  Evidence: `pkg/core/kernel/initramfs-init.sh` calls
  `/bin/nex-populate-etc` before it binds `/var/etc` over the selected
  deployment. `pkg/core/kernel/initramfs-populate-etc.sh` recursively copies a
  source only when the matching target is absent and never replaces a file,
  link, or directory that the host already owns.

- Observation: the existing live-upgrade harness uses the real bootloader and
  Nex CLI for three boots, but it does not test vendor configuration or a
  fresh version 2 machine.
  Evidence: `scripts/qemu-test-live-upgrade.sh` boots the old ref, runs
  `nex upgrade`, boots the new deployment, runs `nex rollback`, and boots the
  rollback. Its assertions cover deployment names and hardlinks, not reader
  priority, factory seeds, or a separate fresh-install disk.

- Observation: the four flat roots have a different `/etc` boundary.
  Evidence: fresh checkouts contain two leaves in Flat Minimal, six in Flat
  Systemd, 12 in Flat Podman, and 470 in Edgebox. Edgebox's 470 leaves include
  445 generated certificate-cache entries, 14 service links, and 11 other
  paths. Raw sorted inventories live under
  `.nex/tmp/ep015-baseline.ko361R/logs/*-etc.tsv`.

- Observation: Readline 8.2's example install list names a source file that
  the pinned release archive does not contain.
  Evidence: the first strict build failed when `make install` tried to copy
  `examples/rltest2.c`. The archive contains `rltest.c` but not `rltest2.c`.
  Removing that stale list entry let all later installs finish.

- Observation: a package smoke must not use the build host's fixed
  configuration paths as its fixture.
  Evidence: this host has a real `/etc/inputrc`, so the first Readline smoke
  stopped before it could create the tier files. The final smoke links a small
  executable to the newly installed library and runs it in a minimal chroot
  containing only its own `/etc`, `/run`, `/usr`, and home files.

- Observation: Most of the 88 leaves fall into a few concrete groups rather
  than representing 88 independent machine choices.
  Evidence: the initial inventory counted 26 Libvirt objects, 27 Systemd unit
  files or links or masks, five PAM files, three container files, three
  tmpfiles entries, two NetworkManager files, and 22 single paths.

- Observation: Some current factory files pin one deployment's package store
  path into persistent machine state.
  Evidence: links for `issue`, `locale.conf`, `nsswitch.conf`, and
  `vconsole.conf` contain exact `/nex/pkg/.../<checksum>` targets. A later
  deployment can remove those targets while `/etc` retains the old links.

- Observation: `/etc/os-release` currently becomes a lasting regular file.
  Evidence: the assembly seeds it from the factory tree. Because first boot
  copies only missing paths, the file can continue to identify an older release
  after the machine selects a new deployment.

- Observation: the factory tree mixes unit definitions with machine choices.
  Evidence: custom `.service`, `.socket`, and `.timer` files live below
  factory `/etc/systemd/system` beside enablement links and deliberate masks.
  Systemd can load immutable unit definitions from `/usr/lib/systemd/system`;
  writable `/etc` should record only administrator overrides, enablement, and
  masks.

- Observation: the generic desktop has no settled initial values for several
  machine-owned files.
  Evidence: the baseline factory tree lacks `/etc/hostname`, `/etc/hosts`, and
  a selected timezone, while `nsswitch.conf` uses `hosts: files dns`. The plan
  must test local hostname behavior instead of assuming that an absent hosts
  file works.

- Observation: Edgebox uses a flat assembly and therefore has no factory-copy
  boundary.
  Evidence: its final `/etc` has 470 leaves, including 445 certificate-cache
  leaves and 25 other paths. This plan must audit those paths against the same
  ownership rule, but it must not pretend that a flat assembly performs the
  Nex-structured first-boot protocol.

- Observation: IWD 2.22 already parses a colon-separated configuration
  directory list, but its service unit replaces the compiled path with one
  Systemd-created `/etc/iwd` directory.
  Evidence: `src/main.c` splits `CONFIGURATION_DIRECTORY` and loads the first
  `main.conf`; `src/iwd.service.in` supplied `ConfigurationDirectory=iwd`.
  The patched package removed that unit setting and four strict builds loaded
  vendor, runtime, administrator, and empty administrator fixtures from the
  intended tier.

- Observation: Nex turns assembly overlay paths below `/etc` into factory
  paths only after the assembly build script exits.
  Evidence: the first Nex Systemd build had the intended files below
  `/target/etc` while its script ran and no `/target/usr/share/factory/etc`.
  The finished checkout moved those 14 leaves into the factory tree.

- Observation: the final Nex Systemd factory tree fell from 27 leaves to 14.
  Evidence: `scripts/check-factory-etc-policy.sh` accepted the finished
  `systems/nex-systemd/0.0.1` checkout at checksum
  `1372f413a125aafb12e2f1c394ce7d7301ce1ae73d8c8f328348e29d220070a8`.
  It contains six regular machine files, three stable adapters, and five unit
  enablement links.

- Observation: Podman 5.4.1 combines three readers with different default
  behavior.
  Evidence: its vendored `containers/common` reader merged
  `/usr/share/containers/containers.conf` before `/etc`; its vendored
  `containers/image` readers used only `/etc/containers/registries.conf` and
  `/etc/containers/policy.json`. Focused Go tests now exercise all three real
  parsers with administrator, runtime, vendor, mask, and registry drop-in
  fixtures.

- Observation: moving Desktop VWL's immutable files cut its factory tree from
  77 leaves and 14,568 bytes to 51 leaves and 12,418 bytes.
  Evidence: the finished root at checksum
  `fc9df3e68a32a7016735b6d6d61c0af891b56061d10a1ea97d006308e0edfeb8`
  passed the policy checker with 33 machine-state leaves, 12 unit choices,
  three masks, and three adapters. Twenty-six machine-state leaves belong to
  Libvirt's initial mutable objects.

## Decision Log

- Decision: Keep the accepted factory path list in
  `scripts/factory-etc-policy.tsv` and make the checker join each built leaf
  to that table.
  Rationale: the table gives every accepted path a concrete owner, consumer,
  source, class, target, and reason. A finished-root scan also catches native
  package factory outputs and build-script additions that a YAML-only scan
  would miss.
  Date/Author: 2026-08-16 / Ralph

- Decision: Remove Systemd's six upstream factory samples from its reusable
  package instead of copying or linking them into every machine.
  Rationale: Glibc already supplies `/usr/lib/nsswitch.conf`; Systemd has safe
  compiled locale and console fallbacks; a login banner is optional; and each
  assembly must choose its own PAM authentication policy. The package keeps
  `systemd-user` and `systemd-run0` in `/usr/lib/pam.d`, where the patched PAM
  reader finds them without `/etc` links.
  Date/Author: 2026-08-16 / Ralph

- Decision: Patch Readline's small complete-file selector directly and do not
  add libeconf.
  Rationale: Readline does not parse mergeable key/value drop-ins. It selects
  one explicit or user file, then one complete system file. Three direct read
  attempts preserve that model and make empty-file masking unambiguous without
  another runtime library.
  Date/Author: 2026-08-16 / Ralph

- Decision: Use IWD's existing colon-separated directory reader with the
  compiled order `/etc/iwd:/run/iwd:/usr/lib/iwd`.
  Rationale: IWD selects one complete `main.conf`, so its current reader
  already provides correct precedence and empty-file masking. Removing
  `ConfigurationDirectory=iwd` lets the service use the compiled list and
  leaves `StateDirectory=iwd` responsible only for mutable Wi-Fi state.
  Date/Author: 2026-08-16 / Ralph

- Decision: Patch Podman's vendored readers directly and do not add libeconf.
  Rationale: Podman's Go libraries already parse TOML registry and engine
  files and strict JSON signature policy. Adding path order around those
  parsers preserves their format checks, user paths, explicit overrides, and
  registry drop-in merge behavior without a second parser library.
  Date/Author: 2026-08-16 / Ralph

- Decision: Treat factory files as one-time seeds, never as a fourth reader
  tier.
  Rationale: after first boot the machine owner owns the copied path. Programs
  must read vendor data from `/usr`, temporary data from `/run`, and lasting
  overrides from `/etc`, as `CONFIGURATION.md` specifies.
  Date: 2026-08-15.

- Decision: Keep only three classes of paths in factory `/etc`: initial machine
  state, Systemd enablement or deliberate masks, and the smallest stable
  adapters needed by a fixed-path consumer.
  Rationale: all other immutable files can follow the selected deployment from
  below `/usr` and should not become stale machine-owned copies.
  Date: 2026-08-15.

- Decision: Patch Nex-built readers or use their upstream vendor-directory
  interface before adding a compatibility path.
  Rationale: Nex builds the complete distribution and can make its programs
  follow the UAPI Group directory order. An assembly may still add an explicit
  adapter for an externally built consumer that cannot be changed.
  Date: 2026-08-15.

- Decision: Consider `libeconf` when it correctly implements the target file
  format's merge rules, but do not make it a universal configuration reader.
  Rationale: a shared library can reduce repeated parser code, but some formats
  select one complete file, have their own include language, require security
  checks, or already have a simpler upstream vendor-path option. The simplest
  correct and secure reader wins for each program.
  Date: 2026-08-15.

- Decision: Remove factory `/etc/os-release` and rely on immutable
  `/usr/lib/os-release` for Nex-built readers.
  Rationale: operating-system identity describes the selected deployment, not
  persistent machine state. Readers that follow the standard already fall back
  to `/usr/lib/os-release`; any remaining fixed-path consumer needs a focused
  stable adapter or patch.
  Date: 2026-08-15.

- Decision: Move unit definitions below `/usr/lib/systemd/system` and leave
  enablement links and deliberate masks in writable `/etc`.
  Rationale: the selected deployment owns unit code, while the machine owner
  decides whether a service starts and may mask it.
  Date: 2026-08-15.

- Decision: Prove upgrades with two distinct system versions and one persistent
  machine state instead of relying only on tree inspection.
  Rationale: only a boot across versions can show that first-boot copying,
  reader priority, deployment switching, and rollback cooperate correctly.
  Date: 2026-08-15.

- Decision: Keep an old factory seed on an already initialized machine if the
  machine still has that path, but omit a removed seed from a fresh version 2
  install.
  Rationale: first boot transferred ownership to the machine. Nex cannot know
  whether the owner edited the file, so an upgrade must not delete it. A fresh
  install has no such inherited state.
  Date: 2026-08-15.

## Outcomes & Retrospective

Not started. At completion, report the starting and final factory inventories,
name every remaining leaf and its owner, list each patched or reconfigured
reader, describe the two-version test, record exact checksums and QEMU results,
and state every adapter, skipped runtime check, or unresolved upstream limit.

## Context and Orientation

Read `CONFIGURATION.md` and the configuration section of `PHILOSOPHY.md` first.
They define the public behavior that this plan must implement. Read
`.agents/MANIFESTS_CODE_STYLE.md` before assembly or package manifest edits and
`.agents/TESTING.md` before test changes. Read `RUST_CODE_STYLE.md`
before changing the CLI or any Rust helper.

Nex-structured manifests declare `nex_structure: true`. At the start of this
plan, search `asm/**/*.yaml` rather than relying on this list. The EP013 baseline
included `asm/nex-minimal.yaml`, `asm/nex-systemd.yaml`,
`asm/desktop-vwl/desktop-vwl.yaml`, both Desktop VWL Nvidia children,
`asm/desktop-dev.yaml`, and `asm/installer/installer.yaml`.

Assembly overlays declare paths that Nex places into the deployment or its
factory tree. The known `/etc` declarations live mainly in
`asm/nex-systemd-overlay.yaml`,
`asm/desktop-vwl/desktop-vwl-overlay.yaml`, and
`asm/installer/installer-overlay.yaml`. Do not edit only the final generated
tree; Git-tracked manifests and scripts remain the source of truth.

At boot, Nex mounts one immutable deployment as the system's `/usr` and retains
the machine's writable `/etc` and `/var`. Early userspace copies a factory path
only if the corresponding `/etc` path is absent. Locate the exact boot code and
its tests before changing the factory contract. Use a real first boot to prove
copy behavior.

Several programs may require reader work. The baseline inventory points at
Readline, Linux PAM, OpenSSH, NetworkManager, IWD, Systemd, containers/image or
Podman, Udev, Sysctl, Tmpfiles, and Libvirt. Inspect each upstream interface
before choosing a patch. Prefer a build option or existing vendor directory.
When a patch remains necessary, follow the repository's patch metadata rules
and make the patch generic enough for another Linux distribution.

Nex now includes `libeconf` in its manifests. It offers one possible
implementation for readers whose file semantics match it. Do not add a
dependency merely because the library exists. Compare its precedence, merge,
masking, error, ownership, and parser behavior against the program's required
contract.

Use `/home/wegel/work/perso/zub/target/debug/zub` for current store operations.
The user has authorized changes to the sibling Zub repository when a proven
Zub defect blocks the required upgrade or checkout test. Keep any Zub change in
its own tested commit and record it here.

## Plan of Work

Start by checking that EP014 has moved to `.agents/execplans/done/`. Run the
worktree pre-task, list durable knowledge files, and build or check out fresh
versions of each Nex-structured assembly. Write a machine-readable inventory
that records each factory leaf's path, type, link target, declaring overlay,
consumer, and current justification. Keep flat assemblies in a separate report
because they do not use first-boot factory copying.

Classify every leaf into one of these concrete groups:

1. The machine owner must retain it, such as account databases, a chosen
   hostname, local host aliases, filesystem mounts, provisioned network
   connections, mutable Libvirt objects, or another explicit site choice.
2. The machine owner enables, overrides, or masks a service through a link or
   file below `/etc/systemd/system`.
3. A fixed-path outside consumer needs a stable compatibility adapter and the
   assembly actually contains that consumer.
4. The selected deployment owns the value, so the file must move below `/usr`
   or disappear in favor of an existing vendor default.
5. A service or boot step generates the value below `/run` or `/var`; the
   factory must not store the generated result.

Do not retain a path without naming its real consumer and owner. Add a focused
repository checker with positive and negative fixtures. It must reject links
from factory `/etc` into exact `/nex/pkg` checksums. It must also reject known
vendor-default locations that this plan moves below `/usr`, unless a documented
machine-state or adapter allowlist names the exact path and reason. Keep the
allowlist small and readable.

For each group 4 path, inspect the program before editing it. Use a standard
upstream build option when one exists. Otherwise patch the reader to check
`/etc`, `/run`, and `/usr` with the complete-file or drop-in rules documented in
`CONFIGURATION.md`. Use `libeconf` only when those exact semantics fit. Add an
executable package smoke that changes each tier and proves replacement, mask,
or merge behavior through the real installed program. A compile, file check,
or source grep does not suffice.

Move assembly-owned unit definitions to `/usr/lib/systemd/system`. Keep only
the links that enable them and the masks that express machine policy in factory
`/etc/systemd/system`. Start the live units in a guest and verify their
behavior. Remove factory PAM links when Nex-built PAM already finds the vendor
service files. Keep mutable Libvirt definitions only if Libvirt really writes
them after first boot; move read-only templates below `/usr` and instantiate
them explicitly when needed.

Remove `/etc/os-release` from factory state and test a real reader across both
deployment versions. Replace each checksum-pinned link with one of three
outcomes: no link because the reader checks `/usr`; a stable link to a canonical
path below `/usr` or `/run` because an included outside consumer needs it; or a
regular initial machine file because a provisioner selected its value. Never
put a deployment checksum in persistent `/etc`.

Define a small provisioning interface for values that the reusable system
cannot choose. Reuse standard files and existing installer inputs rather than
inventing a Nex configuration database. At minimum, prove explicit handling
for hostname, local host entries, timezone, locale, console layout, filesystem
mounts, administrator account, and network connection. The interface may leave
a value absent when the system has a safe standard fallback. Add focused tests
for `localhost` and the selected hostname through the real name-service reader.

Build a two-version test from disposable artifacts. Version 1 must have a
recognizable vendor default and factory seed. Boot it and confirm first-boot
copying. Change one administrator file, add one administrator drop-in, and
retain one seeded machine file. Version 2 must change a vendor default, remove
one factory seed, and change a package or assembly checksum that used to appear
in a link. Boot version 2 against the same writable `/etc` and `/var`. Prove the
new vendor behavior appears when no administrator override exists, prove the
administrator override wins, prove the old machine-owned seed remains, and
prove every adapter resolves. Roll back to version 1 and prove its vendor value
returns while the same machine-owned files remain. Finally boot a fresh version
2 image and prove that the removed version 1 seed is absent.

Use the repository's real deployment selector and first-boot code. Do not fake
the central claim by copying directories in a standalone shell test. A small
self-test may cover checker edge cases, but QEMU or an equivalent isolated boot
must prove the upgrade sequence.

After package work passes, build the assembly graph parent before child and run
each strict build twice. Boot the Systemd base, graphical desktop, and installer
path. Run the two-version upgrade test against a small assembly when possible,
then repeat a focused upgrade assertion with Desktop VWL if desktop-specific
factory state changed. Check out final roots and produce a sorted inventory of
every remaining factory leaf with its classification.

Audit Edgebox and other flat assemblies separately. Move immutable defaults or
generated certificate caches out of their final `/etc` when the same ownership
rule requires it, but do not add factory-copy machinery merely to make a flat
assembly resemble a Nex-structured one. Record the deliberate differences in
the final report and tests.

## Concrete Steps

Run all commands from the repository root. Prefix shell commands with `rtk` as
required by the local agent setup.

1. Verify the dependency and inspect the worktree:

       rtk test -f .agents/execplans/done/014-make-desktop-vwl-generic.md
       rtk git status --short --untracked-files=all
       rtk ls .agents/knowledge
       rtk rg -n 'nex_structure:\s*true' asm --glob '*.yaml'
       rtk rg -n '(^|/)(etc|usr/share/factory/etc)(/|$)|/nex/pkg/' asm scripts

2. Locate the first-boot copy code, deployment selector, and existing upgrade
   tests with `semeja search`. Record their paths and the exact current behavior
   in `Surprises & Discoveries` before changing them.

3. Generate the starting inventory from freshly checked-out assembly refs. Add
   the reusable inventory or checker script and its fixtures to Git. Keep large
   generated reports under `tmp/` unless a compact report forms part of the
   plan's final proof.

4. Run the new factory policy check and its self-test. Record the final command
   after choosing the script path. The negative fixtures must include a
   checksum-pinned `/nex/pkg` link, a copied vendor default, and an undocumented
   compatibility link. Positive fixtures must include machine state, Systemd
   enablement, a deliberate mask, and a documented stable adapter.

5. For every package manifest changed by reader work, run:

       rtk ./src/cli/target/debug/nex check <package-manifest>
       rtk ./nex build <package-manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

   Run the strict package command twice, require the same checksum, and require
   its package smoke to exercise the installed reader.

6. Check every changed assembly manifest:

       rtk ./src/cli/target/debug/nex check <assembly-manifest>

7. Build every affected assembly from parent to child. Run this exact form
   twice per manifest:

       rtk ./nex build <assembly-manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

8. Run shell syntax checks, checker self-tests, provisioning tests, name-service
   tests, and the real two-version boot test. Add the final exact commands to
   this section as the scripts take shape.

9. Boot final system paths with current Zub:

       rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub scripts/qemu-test-systemd.sh systems/nex-systemd/0.0.1 --timeout 120
       rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300
       rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub TARGET_REF=systems/desktop-vwl/0.0.1 scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --headless --timeout 180

10. Check out every final Nex-structured assembly and emit a sorted factory
    inventory. Fail on a dangling link or exact `/nex/pkg` target. Run the
    separate flat-assembly audit and its relevant finished-root tests.

11. Before each commit, run every integrated check that matches the staged
    files. Stage exact paths, keep each commit buildable and useful to
    `git bisect`, and push checked checkpoints because the user requested them.

## Validation and Acceptance

The plan is complete only when all of the following statements are true:

- Every remaining factory `/etc` leaf in every Nex-structured assembly has a
  recorded owner, consumer, class, and reason. No immutable vendor default
  remains there merely because upstream historically used `/etc`.
- No factory file or link contains an exact `/nex/pkg/.../<checksum>` target.
  Every compatibility adapter points at a stable canonical path below `/usr`
  or `/run` and names the included fixed-path consumer that requires it.
- `/etc/os-release` is not a persistent copied release marker. A real reader
  reports the selected deployment's `/usr/lib/os-release` value after upgrade
  and rollback.
- Custom unit definitions live below `/usr/lib/systemd/system`. Factory `/etc`
  retains only machine-owned unit overrides, enablement links, and deliberate
  masks.
- Nex-built readers use their correct `/etc`, `/run`, and `/usr` order. Each
  changed package has an executable smoke that proves the real reader's tier
  and masking or replacement rules.
- The plan records why it used or rejected `libeconf` for each reader considered
  for that library. No package gains it without a concrete benefit.
- Provisioning can supply the machine's hostname, local host entries, timezone,
  locale, console layout, filesystems, administrator account, and network
  connection through standard files or documented installer inputs. Tests
  prove local hostname and `localhost` resolution.
- The two-version test boots version 1, retains one writable `/etc` and `/var`,
  boots version 2, rolls back to version 1, and boots a fresh version 2. It
  proves new vendor values, administrator precedence, persistent machine state,
  valid adapters, rollback behavior, and absence of a removed seed on the fresh
  version 2 machine.
- The final first boot seeds only proven machine state, unit choices, and
  stable adapters. The final report compares its leaf count and byte size with
  the 88-leaf Desktop VWL baseline, but correctness decides the result rather
  than a target count.
- Flat Edgebox and any other flat assembly have a separate ownership audit and
  runtime check. The report explains why each retained `/etc` path differs from
  the Nex-structured factory model.
- `nex check` passes for every changed manifest. Two strict builds of every
  affected package and assembly produce matching checksums.
- Systemd, graphical Chromium, installer, finished-root, checker, provisioning,
  and upgrade tests all pass. The plan records exact commands and results and
  names every skipped runtime path.

## Idempotence and Recovery

Build parent packages and assemblies before their children. Strict builds may
update checksums, dependency refs, and generated output metadata, so inspect
the diff after each build and keep one coherent checked checkpoint. Rerunning a
successful strict command must produce the same output checksum.

The upgrade test must create a unique disposable disk and store path for each
run. It may delete only paths that it created and validated. Keep the version 1
and version 2 inputs immutable during the switch so a failed run can be
repeated. Never point cleanup at the repository root, user home, or shared Zub
store.

When a package patch fails, keep its metadata and test with the patch in the
same commit. Do not commit a manifest that refers to an unavailable patch or a
consumer that cannot read the new vendor path.

If the real two-version test exposes a Zub defect, reproduce it in the sibling
Zub repository, add a regression test, make one focused Zub commit, and push
that repository before resuming Nex. Do not replace a real deployment switch
with a mock merely to avoid fixing the tool.

## Artifacts and Notes

The initial policy checker passed these focused checks:

    rtk bash -n scripts/check-factory-etc-policy.sh
    rtk scripts/check-factory-etc-policy.sh --self-test

The self-test accepts machine account state and a documented unit enablement,
then proves that a copied `os-release`, an undocumented adapter, and an exact
package-checksum link each fail. `shellcheck` is not installed on this host;
`rtk shellcheck scripts/check-factory-etc-policy.sh` exited 127. The seven
fresh-root scans passed only Nex Minimal and rejected the known stale paths in
all other roots. Their TSV output and concise error files live under
`.nex/tmp/ep015-baseline.ko361R/logs/`.

The Systemd package checkpoint passed:

    rtk ./src/cli/target/debug/nex check pkg/core/init/systemd.yaml
    rtk scripts/check-package-config-paths.sh
    rtk ./nex build pkg/core/init/systemd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk ./nex build pkg/core/init/systemd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

Each strict command built the package twice. All four builds produced
`0b01357e5cb84af8c29e4da21b1075c0a576159cf4dbf0846984b0ffe233d165`.
The embedded install checks found `systemd-user` and `systemd-run0` below
`/usr/lib/pam.d` and found no package factory tree. Full logs are
`.nex/tmp/ep015-systemd-build1.log` and
`.nex/tmp/ep015-systemd-build2.log`.

The Readline package checkpoint passed:

    rtk ./src/cli/target/debug/nex check pkg/libs/system/readline.yaml
    rtk scripts/check-package-config-paths.sh
    rtk ./nex build pkg/libs/system/readline.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk ./nex build pkg/libs/system/readline.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

Each strict command built Readline twice. All four builds produced
`6c7351f248a8d763d445600cfa6ed6a1a992e609d4bc052cd721543dde03a36a`.
The chrooted executable proved vendor, runtime, administrator, empty
administrator mask, user, and explicit `INPUTRC` behavior through the newly
built shared library. Full logs are `.nex/tmp/ep015-readline-build1.log` and
`.nex/tmp/ep015-readline-build2.log`.

The IWD package checkpoint passed:

    rtk ./src/cli/target/debug/nex check pkg/net/wifi/iwd.yaml
    rtk scripts/check-package-config-paths.sh pkg/net/wifi
    rtk ./nex build pkg/net/wifi/iwd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk ./nex build pkg/net/wifi/iwd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

Each strict command built IWD twice. All four builds produced
`855d3981ef2cc3b9d22835967b7def71b3b706c41ef8a883899344f6d79a0990`.
The daemon loaded vendor, runtime, administrator, and empty administrator
fixtures through its real installed reader. Full logs are
`.nex/tmp/ep015-iwd-build1.log` and `.nex/tmp/ep015-iwd-build2.log`.

The Nex Systemd assembly checkpoint passed:

    rtk ./src/cli/target/debug/nex check asm/nex-systemd.yaml
    rtk ./nex build asm/nex-systemd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk ./nex build asm/nex-systemd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk scripts/check-factory-etc-policy.sh .nex/tmp/ep015-systemd-final.QJSC9n

Both strict commands built the assembly twice. All four builds produced
`1372f413a125aafb12e2f1c394ce7d7301ce1ae73d8c8f328348e29d220070a8`.
The finished-root checker accepted all 14 factory leaves and wrote their TSV
inventory to `.nex/tmp/ep015-nex-systemd-factory.tsv`. Full strict logs are
`.nex/tmp/ep015-nex-systemd-build1.log` and
`.nex/tmp/ep015-nex-systemd-build2.log`.

The Podman package checkpoint passed:

    rtk ./src/cli/target/debug/nex check pkg/apps/containers/podman.yaml
    rtk scripts/check-package-config-paths.sh pkg/apps/containers
    rtk ./nex build pkg/apps/containers/podman.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk ./nex build pkg/apps/containers/podman.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

Each strict command built Podman twice. All four builds produced
`2b40400b74f55d0471a645ce288c5d9a6945a1c64438e88042ab3ac14eb24189`.
Every build ran the `containers.conf`, `registries.conf`, registry drop-in,
and `policy.json` tests through the vendored production parsers. Full logs are
`.nex/tmp/ep015-podman-build1.log` and
`.nex/tmp/ep015-podman-build2.log`.

The Desktop VWL assembly checkpoint passed:

    rtk ./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
    rtk ./nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk ./nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk scripts/check-factory-etc-policy.sh .nex/tmp/ep015-desktop-final.4p7hz0

Both strict commands built the assembly twice. All four builds produced
`fc9df3e68a32a7016735b6d6d61c0af891b56061d10a1ea97d006308e0edfeb8`.
The finished-root checker accepted all 51 leaves and wrote their TSV inventory
to `.nex/tmp/ep015-desktop-vwl-factory.tsv`. Full strict logs are
`.nex/tmp/ep015-desktop-vwl-build1.log` and
`.nex/tmp/ep015-desktop-vwl-build2.log`. Desktop VWL packages `scripts/` as a
source, so the final checksum must be rebuilt after this plan finishes editing
test scripts.

The EP013 baseline Desktop VWL factory tree contains 88 leaves: 58 regular
files and 30 links, with 31 directories. Its apparent leaf size is 16,350
bytes. EP014 will change this starting point before EP015 begins, so EP015 must
record a new post-EP014 inventory as its authoritative baseline.

EP013 recorded these system checksums:

- Nex Minimal: `6bf44276a0de73189bc0c9b9a5d7060b401749f7600a0ecd57eb895d66f3a6e7`
- Nex Systemd: `e4834b42b6be0c89737c948e00a90120666cf6a9f9b4b497bbd0b06b12470e80`
- Installer: `499b0b54a68b50427488a1b001a719b2c449c8bb64f50b55b630c162a8ba0c50`
- Desktop VWL: `50ed467e695adcb994ce924ad3ed5123d0feeb92ced1b0db828c7673165d7809`
- Nvidia 580 desktop: `d5b5547967fa8d24cc799bd2d814079faca50eb1445dd0d180f843ba02260077`
- Nvidia current desktop: `4d29fe18f4067c1c1d8de6305ba6a665efc40cec988963a42c1a5985d8485413`
- Desktop development system: `00e27517697fa1c68fbec05963fb6be654de40d81c9b12b42279c79b873a5c96`
- Edgebox: `4fffdc9c8b871c1afefda29fd4bc86283087c52d00a4a54744b8fda22f47941e`

These checksums identify historical evidence, not required final output.

## Interfaces and Dependencies

This plan may change package build options, generic local source patches,
package and assembly manifests, Systemd unit locations, installer inputs,
first-boot tests, and repository policy checks. It must preserve the public
ownership and reader rules in `CONFIGURATION.md`.

Programs should use an upstream UAPI-compatible interface when one exists. A
generic patch may add `/usr`, `/run`, and `/etc` tiers without mentioning Nex,
its store, or one assembly. `libeconf` is available but optional. Product
assemblies and installers may create ordinary machine files and stable adapters
for outside fixed-path consumers.

The runtime proof depends on the Nex boot path, the current Zub binary at
`/home/wegel/work/perso/zub/target/debug/zub`, QEMU, and enough local storage to
hold two immutable system versions plus one disposable machine disk. Record a
hard blocker only if one of those required facilities cannot work after the
safe in-scope checks and fixes allowed by this plan.
