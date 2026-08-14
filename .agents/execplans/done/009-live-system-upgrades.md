# Make Installed Nex Systems Upgrade In Place

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

An installed Nex machine should be able to move from one built system root to
another without reinstalling from USB. The user should run one clear command,
reboot, and then see the new system. If the new system fails, the user should
run one rollback command, reboot, and see the previous system again.

Nex already stores installed system roots under
`/nex/deployments/<checksum>.<serial>`. A deployment is one complete checked-out
system root. The bootloader and initramfs choose a deployment through the
`zub=` kernel command-line value. The CLI already has `nex deploy`,
`nex rollback`, and `nex deployments`, but these commands do not yet form a
complete tested live-upgrade story. In particular, the installed machine needs
a clear way to fetch or receive a new system ref into `/nex/repo`, stage a new
deployment safely, reboot into it, and prove rollback.

After this plan, a user can install an older Nex desktop image in QEMU, upgrade
it in place to a newer built system ref, reboot into the new deployment, roll
back, and reboot into the previous deployment. A coding agent must be able to
run the same proof without visual judgment or manual disk edits.

## Progress

- [x] (2026-07-03 16:01Z) Ran the Ralph worktree pre-task. The tracked
  worktree was clean; `.agents/` is ignored, so this ExecPlan remains local
  Ralph state and is not a commit candidate.
- [x] (2026-07-03 16:01Z) Read the required root docs and relevant knowledge
  notes: `installer.md`, `kernel-and-boot.md`, `system-assemblies.md`,
  `cli-testing.md`, `reproducibility.md`, and `graphical-qemu.md`.
- [x] (2026-07-03 16:01Z) Inspected the current `nex deploy`, `nex rollback`,
  `nex deployments`, installer, zub remote, and QEMU install code paths.
- [x] (2026-07-03 16:09Z) Defined the installed-machine remote and fetch
  contract for `/nex/repo`: installed machines pull built system refs into
  their local repo from configured zub remotes, and remotes can be SSH URLs or
  local filesystem repo paths. Committed
  `5c9ea13 cli: support local zub remotes`.
- [x] (2026-07-03 16:02Z) Hardened `nex deploy` so it resolves or pulls the
  target ref before metadata lookup, requires `nex.system.checksum` by
  default, rejects duplicate checksums unless `--force`, stages into a hidden
  temp directory, syncs, and renames into the final deployment name. Committed
  `69d7618 cli: stage system deployments atomically`.
- [x] (2026-07-03 16:03Z) Added `nex upgrade` as an explicit command that
  reuses the deploy path and appears in `nex --help`. Committed
  `ba2edc5 cli: expose upgrade command`.
- [x] (2026-07-03 16:06Z) Taught `scripts/nex-install` to convert
  `nex.remote=name=url` kernel arguments into real zub `config.toml` remote
  entries while preserving the old `remotes/` marker files. Added
  `scripts/test-nex-install-remotes.sh`, rebuilt `asm/installer/installer.yaml`
  reproducibly, and committed `af3e225 installer: seed zub remotes`.
- [x] (2026-07-03 19:12Z) Added
  `scripts/test-live-upgrade-hardlinks.sh` as the fast host-side proof for
  deployment hardlinks.
- [x] (2026-07-03 19:12Z) Added
  `scripts/qemu-test-live-upgrade.sh` as the full guest proof for upgrade,
  reboot, rollback, reboot, and hardlink reuse.
- [x] (2026-07-03 19:25Z) Added a direct-initramfs boot path to
  `scripts/qemu-test-installer.sh`; this catches initramfs mount regressions
  in minutes before rebuilding the Linux package.
- [x] (2026-07-03 19:48Z) Rebuilt `pkg/core/kernel/initramfs.yaml`,
  `pkg/core/kernel/linux.yaml`, `asm/desktop-vwl/desktop-vwl.yaml`,
  `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`, and
  `asm/installer/installer.yaml` with reproducible checksums.
- [x] (2026-07-03 20:00Z) Ran checks: `bash -n` for changed scripts,
  `scripts/test-nex-install-remotes.sh`, `scripts/test-live-upgrade-hardlinks.sh`,
  `cargo test --manifest-path src/cli/Cargo.toml`,
  `scripts/check-builder-style.sh`, `nex check` for changed manifests, and
  `git diff --check`. All passed.
- [x] (2026-07-03 20:06Z) Ran
  `scripts/qemu-test-installer.sh --headless --rebuild --autoinstall --assert-boot`.
  The installed desktop booted with initramfs v7 and printed
  `ASSERT-BOOT-PASS`.
- [x] (2026-07-03 20:14Z) Ran `scripts/qemu-test-live-upgrade.sh`. The guest
  pulled `systems/desktop-vwl-nvidia-580/0.0.1`, deployed it, proved
  `/etc/os-release` hardlinked to a `/nex/repo` blob, rebooted into it, rolled
  back with hardlinks, rebooted into the rollback deployment, and printed
  `LIVE-UPGRADE-PASS`.
- [x] (2026-07-03 20:17Z) Committed the checked live-upgrade changes as
  `86e2288 cli: hardlink live upgrades`.
- [x] (2026-07-03 20:20Z) Promoted durable notes into
  `.agents/knowledge/cli-testing.md`, `.agents/knowledge/installer.md`,
  `.agents/knowledge/kernel-and-boot.md`, and
  `.agents/knowledge/reproducibility.md`.
- [x] (2026-07-04 01:50Z) Reopened this plan after review found that
  `nex rollback` copied straight into the final highest-serial directory. A
  failed copy could leave a partial directory that the bootloader might choose.
- [x] (2026-07-04 01:56Z) Changed `nex rollback` to copy into
  `.checksum.serial.tmp`, sync the staged tree, rename the temp directory into
  the final deployment name, and remove the temp directory when the copy fails.
- [x] (2026-07-04 01:57Z) Added rollback unit tests. One test runs the real
  rollback command and checks stale temp cleanup plus hardlinks. One test runs
  the publish helper with a fake copy command that writes a partial staged file
  and exits nonzero, then checks that no final deployment directory exists.
- [x] (2026-07-04 01:58Z) Ran
  `cargo test --manifest-path src/cli/Cargo.toml commands::rollback`. Both
  rollback tests passed.
- [x] (2026-07-04 02:01Z) Ran the full CLI checks after the rollback fix:
  `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`,
  `cargo test --manifest-path src/cli/Cargo.toml`,
  `scripts/check-builder-style.sh`, and `git diff --check`. All passed.
- [x] (2026-07-04 02:01Z) Rebuilt `pkg/core/nex/nex.yaml` with the fixed
  rollback command and recorded checksum
  `565ec0d24aad191c9054901d98ac8809e4c69c39ae27afa675455c13198f44c8`.
  `./src/cli/target/debug/nex check pkg/core/nex/nex.yaml` passed.
- [x] (2026-07-04 02:01Z) Rebuilt and checked the assemblies that the
  live-upgrade proof uses or boots through:
  `asm/nex-systemd.yaml`,
  `asm/desktop-vwl/desktop-vwl.yaml`,
  `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`, and
  `asm/installer/installer.yaml`. All strict builds and `nex check` commands
  passed.
- [x] (2026-07-04 02:01Z) Ran `scripts/test-live-upgrade-hardlinks.sh`. The
  fast host proof passed and printed `live-upgrade-hardlink-pass`.
- [x] (2026-07-04 02:02Z) Tried `scripts/qemu-test-live-upgrade.sh` after the
  rebuilt package and assemblies. The script stopped before booting because
  the host lacked `mcopy`.
- [x] (2026-07-04 02:04Z) Installed host packages `mtools` and `dosfstools`,
  which provide `mcopy` and `mkfs.vfat`.
- [x] (2026-07-04 02:09Z) Reran `scripts/qemu-test-live-upgrade.sh`. The
  guest booted the old deployment, pulled and deployed
  `systems/desktop-vwl-nvidia-580/0.0.1`, proved the upgraded
  `etc/os-release` hardlinked to a repo blob, rebooted into the upgraded
  deployment, ran the fixed rollback path through a hidden staging directory,
  proved rollback hardlinks, rebooted into the rollback deployment, and
  printed `LIVE-UPGRADE-PASS`.

## Surprises & Discoveries

- Observation: `.agents/` is ignored in this checkout, including active
  ExecPlans and scratch knowledge.
  Evidence: `git check-ignore -v .agents/execplans/009-live-system-upgrades.md`
  reports `.gitignore:20:/.agents/`.

- Observation: `nex deploy` opened the zub store and immediately read
  `nex.system.checksum` metadata before it called `Store::resolve_ref`.
  Evidence: the old `src/cli/src/commands/deploy.rs` called
  `store.get_metadata(&args.system_ref, "nex.system.checksum")` before any
  method that can pull a missing ref from configured remotes.

- Observation: `Store::resolve_ref` can pull a missing ref from configured SSH
  remotes, but `Store::get_metadata` does not try remote pull by itself.
  Evidence: `src/cli/src/store/mod.rs` calls `pull_from_remote` in
  `resolve_ref`, while `get_metadata` calls `get_commit_info`, which calls
  `resolve_ref_to_hash_and_repo` and only checks the primary and fallback
  repos.

- Observation: The first deploy hardening slice has focused tests.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml commands::deploy`
  passed five tests at 2026-07-03 16:01Z.

- Observation: `nex upgrade` was only a hidden alias on `deploy`, so top-level
  help did not show it as the main user path.
  Evidence: `src/cli/src/main.rs` had `#[clap(alias = "upgrade")]` on the
  `Deploy` variant before commit `ba2edc5`; after that commit,
  `./src/cli/target/debug/nex --help` lists both `deploy` and `upgrade`.

- Observation: `scripts/nex-install` seeded `/var/nex/repo/remotes/<name>`
  files, but `Store::open` reads zub remotes from `/var/nex/repo/config.toml`.
  Evidence: `seed_installer_remotes` wrote only files under `remotes/`, while
  `src/cli/src/store/mod.rs` loads `Config::load(path.join("config.toml"))`
  and iterates `config.remotes`.

- Observation: The installer assembly checksum changes when
  `scripts/nex-install` changes because the overlay copies that script into
  `/usr/bin/nex-install`.
  Evidence: strict installer rebuild updated
  `asm/installer/installer.yaml` from
  `31af8698988c3baa8917607a6a2ab2170f504e43abd9f4af5ceaf9582ed10863` to
  `cd0493b49d6cb88e7bf379a4eda39ed79307220d546ae4b8b2f776249f4c50ed`.

- Observation: zub remotes can now name a local filesystem repo path, not only
  an SSH URL.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml store::store_tests`
  passed after adding `resolve_ref_pulls_from_configured_local_remote`, and
  full `cargo test --manifest-path src/cli/Cargo.toml` passed 148 unit tests
  plus three blob tests before commit `5c9ea13`.

- Observation: a direct initramfs QEMU path is the right fast loop for
  `pkg/core/kernel/initramfs-init.sh`. It proves the mount contract without
  rebuilding the Linux package.
  Evidence: `scripts/qemu-test-installer.sh --direct-initramfs --assert-boot`
  booted initramfs v7, verified `/sysroot` is read-only, verified root-backed
  `/nex/repo` and `/nex/staging` are writable, and printed `ASSERT-BOOT-PASS`.

- Observation: the normal boot path still needs a Linux package rebuild after
  initramfs changes because `pkg/core/kernel/linux.yaml` embeds the packaged
  initramfs through `CONFIG_INITRAMFS_SOURCE`.
  Evidence: only rebuilding `pkg/core/kernel/initramfs.yaml` and assemblies
  left QEMU booting older mount logic until `pkg/core/kernel/linux.yaml` was
  rebuilt.

- Observation: live upgrade hardlinks only work when the deploy command opens
  the repo through the same filesystem view as the deployment target.
  Evidence: the full QEMU test proved
  `/sysroot/nex/deployments/<checksum>.1/etc/os-release` has link count 2 and
  shares an inode with a blob under `/nex/repo/objects/blobs`.

- Observation: the full live-upgrade harness is slower because it creates
  separate root and var images and then appends them into a combined disk
  image.
  Evidence: `ps` showed the harness running `dd` against the 12 GiB
  `var.img`, while `.nex/tmp/live-upgrade` temporarily used about 45 GiB.

- Observation: the first completed EP009 proof tested successful rollback but
  did not test rollback copy failure. `nex rollback` created the final
  higher-serial directory before it ran `cp -a -l`, so a failed copy could
  leave a partial directory that the bootloader might choose.
  Evidence: before the 2026-07-04 fix,
  `src/cli/src/commands/rollback.rs` called `fs::create_dir_all(&dst)` and
  then ran `cp -a -l <source>/. <dst>`.

- Observation: the full live-upgrade QEMU proof needs host `mcopy` and
  `mkfs.vfat`.
  Evidence: on 2026-07-04, `scripts/qemu-test-live-upgrade.sh` first stopped
  with `error: mcopy not found`, then stopped with `error: mkfs.vfat not
  found`. Installing Arch packages `mtools` and `dosfstools` supplied those
  commands, and the next run printed `LIVE-UPGRADE-PASS`.

## Decision Log

- Decision: Build live upgrades on `/nex/deployments/<checksum>.<serial>`,
  `nex deploy`, and `nex rollback` instead of adding a separate root layout.
  Rationale: Installed systems already use this layout, and QEMU tests already
  know how to boot it. Reusing it keeps upgrade and rollback inside the same
  boot contract.
  Date/Author: 2026-07-03 / Carlos

- Decision: Make `nex upgrade` consume already-built system refs from a zub
  repo rather than rebuilding packages or assemblies on the target machine.
  Rationale: Nex treats Git manifests and built zub refs as the source of
  reproducible system artifacts. A live machine should not update checksums,
  mutate manifests, or create a new system checksum during a normal upgrade.
  Date/Author: 2026-07-03 / Carlos

- Decision: Require QEMU to prove both upgrade and rollback before this plan is
  complete.
  Rationale: A CLI check that creates a directory is not enough. The changed
  path must prove that the bootloader and initramfs select the expected
  deployment after reboot.
  Date/Author: 2026-07-03 / Carlos

- Decision: Keep `deploy` as the lower-level command and expose `upgrade` as a
  separate CLI variant that runs the same implementation.
  Rationale: Scripts can keep using `deploy`, while humans see and run the
  clearer `upgrade` command from top-level help.
  Date/Author: 2026-07-03 / Ralph

- Decision: Support local zub remotes in the same `[[remotes]]` config used by
  SSH remotes.
  Rationale: Real installed machines can pull from SSH remotes, while QEMU
  tests can use a repo path inside the guest. Both paths exercise the same
  `Store::resolve_ref` fetch contract.
  Date/Author: 2026-07-03 / Ralph

- Decision: Mount the physical root filesystem read-write in the initramfs,
  keep the selected deployment and `/sysroot` views read-only, and bind
  `/nex/repo` plus `/nex/staging` from the root filesystem as writable.
  Rationale: installed upgrades need to write repo objects and deployment
  trees, while the booted deployment root should remain immutable.
  Date/Author: 2026-07-03 / Ralph

- Decision: Keep `/nex/users` and `/nex/manifests` on the var partition, but
  keep `/nex/repo`, `/nex/staging`, and `/nex/deployments` on the root
  partition.
  Rationale: package and deployment objects need same-filesystem hardlinks;
  user and manifest state remains mutable machine state.
  Date/Author: 2026-07-03 / Ralph

- Decision: Use `etc/os-release` as the deterministic QEMU hardlink probe.
  Rationale: scanning every deployment file and then searching every repo blob
  was too slow on a full desktop image. A fixed regular file proves the same
  invariant cheaply.
  Date/Author: 2026-07-03 / Ralph

- Decision: Make rollback publish the same way deploy publishes: copy into a
  hidden temp directory, sync it, rename it, and delete the temp directory when
  the copy fails.
  Rationale: the bootloader chooses the highest serial directory. A final
  directory must not appear until it contains a complete rollback tree.
  Date/Author: 2026-07-04 / Carlos

## Outcomes & Retrospective

Implemented, tested, and committed the first live-upgrade work as
`86e2288 cli: hardlink live upgrades`. Installed systems can now pull a system
ref from a configured zub remote, deploy it under `/nex/deployments`, reboot
into the highest serial deployment, roll back by creating a higher-serial copy
of the older deployment, and reboot back. The first QEMU proof verified that
deployed files and rollback files reused hardlinks.

Reopened on 2026-07-04 because rollback did not publish atomically. The fix
changes rollback so the final higher-serial directory appears only after the
hardlink copy and sync succeed. The rollback unit tests now cover both the
successful hardlink copy and a failed copy that must not publish a final
directory. The fixed CLI package and the booted assemblies have been rebuilt
and checked. The full guest proof passed after installing the missing host
tools. The plan is complete.

The fast loop for future initramfs mount work is
`scripts/qemu-test-installer.sh --direct-initramfs --assert-boot`. The full
proof remains `scripts/qemu-test-live-upgrade.sh` after the Linux package and
assemblies have been rebuilt.

## Context and Orientation

Read these files before editing:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `RUST_CODE_STYLE.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/knowledge/installer.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/cli-testing.md`
- `.agents/knowledge/reproducibility.md`

Files and commands that matter:

- `src/cli/src/commands/deploy.rs` creates a new deployment directory under a
  mounted sysroot.
- `src/cli/src/commands/rollback.rs` creates a higher-serial copy of an older
  deployment.
- `src/cli/src/commands/deployments.rs` lists installed deployment directories
  and marks the current one from `/proc/cmdline`.
- `src/cli/src/main.rs` defines CLI subcommands and already aliases `deploy`
  as `upgrade`.
- `scripts/nex-install` creates the installed disk layout and initializes
  `/var/nex/repo`.
- `scripts/create-installer-usb` embeds an installer root and one target
  system into an image.
- `scripts/qemu-test-installer.sh` creates and boots installer images.
- `scripts/qemu-test-efi.sh`, `scripts/qemu-desktop.sh`, and
  `scripts/qemu-test-graphical.sh` contain boot checks and deployment layout
  examples.
- `src/cli/src/store/mod.rs` wraps zub repo access, refs, checkout, and remote
  lookup behavior.
- `zub` stores built package and system refs. A system ref looks like
  `systems/desktop-vwl-nvidia-580/0.0.1`.

The target live-upgrade operator flow should be:

```bash
nex upgrade systems/desktop-vwl-nvidia-580/0.0.1
reboot
nex deployments
```

The rollback flow should be:

```bash
nex rollback --yes
reboot
nex deployments
```

## Plan of Work

1. Run the worktree pre-task from `AGENTS.md`.
2. Read the current CLI and installer code paths listed above.
3. Write a small design note in this ExecPlan that describes how an installed
   machine gets a new system ref into `/nex/repo`.
4. Harden `nex deploy`:
   - reject a missing `nex.system.checksum` unless the caller passes a flag
     that explicitly allows commit-hash deployment
   - reject deploying the same checksum as the current boot unless the caller
     passes `--force`
   - checkout into a temporary path under `/nex/deployments`
   - sync files and parent directories where Rust or a small helper can do so
   - rename the temporary path to `<checksum>.<serial>` only after checkout
     succeeds
   - remove stale temporary paths when a previous run failed
5. Add or complete `nex upgrade`:
   - keep `nex deploy` as the low-level command
   - make `nex upgrade` fetch the target system ref from configured remotes
     when `/nex/repo` lacks it
   - print the current deployment, target ref, target checksum, new deployment
     name, and reboot instruction
   - support `--dry-run`
6. Teach the installer to seed installed systems with a useful remote
   configuration:
   - preserve current `nex.remote=<name>=<url>` kernel command-line support if
     it exists
   - make hardware installer docs name the default remote story
   - do not put machine-local secrets or host-only paths into Git
7. Build a QEMU live-upgrade test:
   - create an installed disk from an older or deliberately distinct system ref
   - boot it
   - make the newer target ref available to the guest through the installed
     `/nex/repo` remote path or a controlled host-side zub remote
   - run `nex upgrade <target-ref>` inside the guest
   - reboot the guest
   - assert `/proc/cmdline` names the new `<checksum>.<serial>` deployment
   - run `nex rollback --yes`
   - reboot the guest
   - assert `/proc/cmdline` names the previous checksum with a higher serial
8. Update docs and knowledge:
   - document the user commands
   - document how test images seed remotes
   - document how to recover from a failed upgrade, failed boot, and rollback
9. Commit each focused slice after matching checks pass.

## Concrete Steps

Run these commands from the repository root.

Start with orientation and pre-task:

```bash
git status --short --untracked-files=all
ls .agents/knowledge
sed -n '1,220p' AGENTS.md
sed -n '1,220p' PHILOSOPHY.md
sed -n '1,220p' RUST_CODE_STYLE.md
sed -n '1,220p' MANIFESTS_CODE_STYLE.md
sed -n '1,220p' .agents/TESTING.md
```

Read relevant code:

```bash
sed -n '1,280p' src/cli/src/commands/deploy.rs
sed -n '1,320p' src/cli/src/commands/rollback.rs
sed -n '1,260p' src/cli/src/commands/deployments.rs
sed -n '1,260p' src/cli/src/main.rs
sed -n '1,520p' scripts/nex-install
sed -n '1,380p' scripts/create-installer-usb
```

Run Rust checks while changing CLI code:

```bash
cargo fmt --manifest-path src/cli/Cargo.toml -- --check
cargo test --manifest-path src/cli/Cargo.toml
scripts/check-builder-style.sh
```

Run manifest checks when installer or assembly manifests change:

```bash
./src/cli/target/debug/nex check asm/installer/installer.yaml
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl-nvidia-580.yaml
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl-nvidia-current.yaml
```

Build any changed assembly with the strict command that updates the checked-in
system checksum only when the assembly was intentionally changed:

```bash
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl-nvidia-580.yaml --verbose --check --update-checksum --force
```

Do not use `--update-checksum` during a read-only audit rebuild. Use it only
when this plan intentionally changes the assembly output.

The final QEMU proof should be a script command that an agent can run, for
example:

```bash
scripts/qemu-test-live-upgrade.sh \
  --from systems/desktop-vwl/0.0.1 \
  --to systems/desktop-vwl-nvidia-580/0.0.1 \
  --timeout 600
```

The exact script name may change if the implementation fits better as an
option on `scripts/qemu-test-installer.sh`. The final command must appear in
`Outcomes & Retrospective`.

## Validation and Acceptance

This plan is complete only when all of these checks pass:

- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`
- `cargo test --manifest-path src/cli/Cargo.toml`
- `scripts/check-builder-style.sh`
- `git diff --check`
- every changed manifest passes `./src/cli/target/debug/nex check <path>`
- every changed package or assembly builds with the strict command that matches
  the changed behavior
- the live-upgrade QEMU test installs an older system, boots it, upgrades to a
  newer system ref, reboots into the newer deployment, rolls back, and reboots
  into the previous deployment

The QEMU test must assert concrete facts from inside the guest:

- `nex deployments` lists at least two deployment directories after upgrade
- `/proc/cmdline` contains the expected `zub=` deployment after the upgrade
  reboot
- the current deployment checksum equals the target ref checksum after the
  upgrade reboot
- `/proc/cmdline` contains a higher-serial deployment for the previous checksum
  after rollback
- `systemctl is-system-running` reaches `running` or `degraded` without failed
  units outside a documented allowlist

Human-facing acceptance:

- A user sees a clear upgrade command, a clear reboot instruction, and a clear
  rollback command.
- A failed checkout cannot leave a half-populated deployment that the boot path
  might select.
- A normal upgrade never edits package manifests, assembly manifests, or
  package checksums on the installed machine.

## Idempotence and Recovery

All commands should be safe to rerun after a failed test.

`nex upgrade` and `nex deploy` must use a temporary deployment path such as
`/nex/deployments/.<checksum>.<serial>.tmp` and remove it before retrying. They
must not create the final `<checksum>.<serial>` directory until checkout
finishes.

If a QEMU test fails before reboot, rerun the test with a fresh temporary disk
image. If it fails after upgrade but before rollback, inspect the guest disk by
mounting the root partition and checking `/nex/deployments`.

If a real machine fails after an upgrade, boot an older deployment from the
bootloader if that UI exists, or boot installer media and inspect
`/nex/deployments`. Once the old system boots, `nex rollback --yes` should
create a new higher-serial copy of the previous system so the default boot
chooses it.

The zub store is a cache of built artifacts. Do not rely on uncommitted store
state when writing tests. Tests must either build the named refs, import them,
or fail with an actionable message that names the missing ref.

## Artifacts and Notes

Record here:

- exact QEMU command lines and log paths
- source and target system refs
- source and target system checksums
- deployment names before and after upgrade
- rollback deployment name
- any zub remote configuration written by the installer
- any failed boot log excerpts that shaped the final implementation
