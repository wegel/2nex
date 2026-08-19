# Make an installed machine work, and test that it does

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this plan, one command passes, and it proves that a person sitting at an
installed Nex machine can do the four things Nex exists to let them do:

    scripts/test-machine-operations.sh

It passes because this plan both writes the tests and fixes what they find.
Writing tests that fail against a broken product proves nothing on its own, so
the fixes are in scope here rather than deferred. This plan absorbs what was
ExecPlan 018, now deleted: shipping manifests as a Git bundle, and renaming the
store.

It boots a real installed disk image in QEMU and makes the guest perform each
operation itself, over SSH, then checks the machine's own state afterwards.
Today no test does this. `scripts/qemu-test-live-upgrade.sh` proves the boot and
rollback machinery, but it stages the new deployment from the host by checking a
ref straight into `/nex/deployments`, which is the operation under test. So the
thing a user actually types has never been exercised.

The four operations, in the words a user would use:

1. Build a package on my machine.
2. Install a package temporarily, then throw it away.
3. Install a package and keep it across a reboot.
4. Put a new system version on the machine so it boots next time, and undo that.

Test 1 also settles a question this repository cannot currently answer from
source alone. Package manifests name their build environment by Git blob SHA,
and 476 of them name `27b6e5dc`, a historical revision of `env/standard.yaml`
(see `.agents/knowledge/environment-pinning.md`). `/nex/manifests` is created on
first boot by `git init` plus one commit of the shipped snapshot
(`base/nex-systemd.yaml:222`), and a repository with one commit cannot hold a
past revision of a file. Either the machine gets its manifests some other way,
or test 1 fails. Both outcomes are useful, and neither is known today.

## Progress

- [x] Read `scripts/qemu-test-live-upgrade.sh` end to end and list the parts
      worth reusing: disk assembly, `ensure_assert_key`, `ssh_probe`,
      `wait_for_ssh`, serial logging.
- [x] (2026-08-19) Established the fixture: `base/nex-systemd.yaml` plus SSH
      injected into the var tree by `scripts/prepare-qemu-test-identity.sh`. No
      new assembly needed; see Surprises & Discoveries.
- [x] (2026-08-19) Built the fixture image and proved an overlay boots from it
      with SSH reachable (9s to SSH, three runs in a row).
- [x] (2026-08-19) Implemented the harness verbs: `boot`, `run`, `reboot`,
      `expect_deployment`, in `scripts/test-machine-operations.sh`. `reboot`
      and `expect_deployment` are exercised by no test yet (test 1 needs
      neither); they are written and match the reference script's proven
      `assert_current_deployment`/reboot-by-killing-qemu pattern, but unproven
      end to end until tests 3/4 exist.
- [x] (2026-08-19) Wrote test 1 (`scripts/machine-tests/build-package.sh`)
      and recorded what it reports: FAIL, deterministically, three runs in a
      row. See Surprises & Discoveries.
- [x] (2026-08-19) Wrote `tests/nex-test-fixture.yaml` (extends
      `nex:base/nex-systemd.yaml`, adds `git`, ships
      `tests/nex-test-fixture.bundle` and overrides `nex-init-manifests` to
      clone it). `nex check` passes; `nex build --single --check
      --update-checksum` is strict two-build reproducible at checksum
      `a22e42ab383d0c108c3fb6f2ada6899c6f3a21ff31aed3c05e61b112aee6b507`
      (ref `systems/nex-test-fixture/0.0.1`). Harness repointed at it. Test
      1 reran, still FAILs, now past environment resolution's first gate; see
      Surprises & Discoveries.
- [x] (2026-08-19) Wrote tests 2, 3, 4
      (`scripts/machine-tests/temporary-install.sh`,
      `persistent-install.sh`, `deploy-and-rollback.sh`). Test 4 passes
      end to end, twice in a row. Tests 2 and 3 FAIL deterministically at
      `nex stage`, for a reason this plan had not named: the fixture's kernel
      bundle carries no `overlay.ko`. See Surprises & Discoveries.
- [x] (2026-08-19) Wired all four tests into
      `scripts/test-machine-operations.sh`. `--test <name>` still runs one
      alone; with no flag it runs all four in order and prints one summary.
      Tests 3 and 4 cross a reboot, handled by a per-test phase list
      (`TEST_PHASES`) the host steps through, calling the `reboot` verb
      between phases -- the first real exercise of `reboot` and
      `expect_deployment`-equivalent logic in this plan.
- [x] (2026-08-19) Ran the acceptance checks: full suite twice in a row
      (identical: 3 FAIL, 1 PASS, same reasons both times); a deliberately
      broken guest command (typo'd deploy ref) made exactly
      `deploy-and-rollback` fail, carrying `ref not found:
      systems/nex-systemd/0.0.1-typo-deliberately-broken`, while the other
      three tests kept their original, unrelated failures. Reverted after.
- [x] (2026-08-19) Promoted durable findings into `.agents/knowledge/`: the
      OverlayFS/kernel-bundle finding into `kernel-and-boot.md` ("Kernel
      bundle module dependencies") with a cross-reference from
      `machine-self-hosting.md`.

- [ ] Fix environment resolution: resolve blobs against the manifests
      repository, not the store. `build-package` must then pass.
- [ ] Add the `git_bundle` source kind with a recorded sha256, generated with
      `pack.threads=1` from an explicit commit.
- [ ] Move `examples/desktop-vwl/desktop-vwl.yaml` from its six `dev:` sources
      to one bundle source, and delete `src/cli/.nex-dev-prepare`.
- [ ] Rewrite `nex-init-manifests` in `base/nex-systemd.yaml` to clone the
      bundle, then drop the fixture's override of it.
- [ ] Rename `/nex/repo` to `/nex/store` and `--repo` to `--store`, with a
      migration for machines carrying the old path.
- [ ] Rebuild every assembly, record checksums, and run the full suite green.

## Surprises & Discoveries

**2026-08-19: no fixture assembly is needed.** `base/nex-systemd.yaml` ships
openssh and the `sshd` account but never enables the service, and root is
locked (`root:!*:`), so it is not reachable as built. That is deliberate:
`scripts/check-generic-assembly-policy.sh:36` rejects
`multi-user.target.wants/sshd.service` in a reusable assembly as "enabled
remote access".

The access belongs in machine state instead, and a helper already puts it
there. `scripts/prepare-qemu-test-identity.sh:74` writes the `sshd.service` and
`sshd-keygen.service` enablement symlinks into the var tree's
`/etc/systemd/system/multi-user.target.wants/`, alongside `authorized_keys` for
root or a `nex-test` user and an `/etc/ssh/sshd_config`. `qemu-test-live-upgrade.sh:302`
calls it that way.

So SSH access is injected at disk-assembly time rather than carried by an
assembly.

**2026-08-19, correcting the above: `base/nex-systemd.yaml` still cannot serve
as the fixture.** Test 1 ran against it and failed one step earlier than
this plan predicted. It reported:

    manifests-system-exists=true
    manifests-system-is-git=false
    manifests-worktree-exists=false
    pkg-manifest-exists=false
    build-exit=1
    error-line=Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }

`nex-systemd` names `/usr/share/nex/manifests` only as the source path inside
`nex-init-manifests` (line 202); it never populates it, and it ships no `git`
package. Only `examples/desktop-vwl/desktop-vwl.yaml` does either. So
`nex-init-manifests` finds no snapshot at boot, exits 0 after a warning, and
`/nex/manifests` stays an empty directory. `setup_user_manifests_worktree`
(`repo.rs:261`) then cannot create the per-user worktree, because it needs
`/nex/manifests` to already be a Git repository.

The build never got far enough to resolve an environment. That is not a gap in
the finding, because there was never a question there to answer. The design
requirement, restated by the human on 2026-08-19, is that a machine running Nex
always carries the full Git history of the manifests repository, so that every
pinned document is reachable. `nex-init-manifests` doing `git init` plus one
commit over a shipped file tree is therefore wrong code, not a condition to
measure: a one-commit repository cannot hold the historical
`env/standard.yaml` revisions that 476 manifests name.

`tests/nex-test-fixture.yaml` must be written after all. It extends
`nex:base/nex-systemd.yaml` and adds what makes a machine self-hosting: a `git`
package, a Git bundle carrying full history, and an override of
`nex-init-manifests` that clones from that bundle rather than running
`git init`. It must not copy desktop-vwl's `dev:` snapshot, which would
reproduce the historyless repository described above.

**2026-08-19: test 1 fails, and the cause is one step earlier than the
blob-SHA question this plan opened with.** `/nex/manifests` never becomes a
Git repository at all on this fixture, so the question of which revision of
`env/standard.yaml` a historical `manifest_ref` resolves to never comes up.

`base/nex-systemd.yaml` never populates `/usr/share/nex/manifests` (grepped
the whole manifest: no such path). That directory is only ever created by
`examples/desktop-vwl/desktop-vwl.yaml:933-965`, which is out of scope by this
plan's own Decision Log. `nex-init-manifests.service`
(`base/nex-systemd.yaml:195-226`) runs at boot, finds
`/usr/share/nex/manifests` missing, logs "missing source snapshot" to stderr,
and exits 0 without touching `/nex/manifests`. `git` is also not in the
package list, so even if a snapshot existed, `git init` would fail (`command
-v git` guards that path and takes the same silent-exit branch).

The practical effect, confirmed by SSH into a booted overlay: `/nex/manifests`
exists (it's a var-tree mount point) but is an empty directory, not a Git
repository. `nex build pkg/cli/archive/gzip.yaml --single`, run from
`/nex/manifests` (the only manifests-shaped path that exists; the
per-user worktree at `/nex/users/root/manifests` is never created, since
`repo.rs:261`'s `setup_user_manifests_worktree` needs `/nex/manifests` to
already be a Git repo with a HEAD before it can worktree-add from it), fails at
the very first step in `run_build` (`src/cli/src/main.rs:159`,
`Path::new(&args.manifest).canonicalize()?`) because
`/nex/manifests/pkg/cli/archive/gzip.yaml` does not exist. The guest's exact
output:

    Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }

This reproduced identically across three separate fresh-overlay boots. It is a
legitimate FAIL per this plan's Expected Outcome section, just for a more
fundamental reason than the one named there: the machine never gets a
manifests source at all on this fixture, so the question of *which revision*
it would resolve `27b6e5dc` against never arises. Nothing here was changed to
make it pass, per instruction.

**2026-08-19: `tests/nex-test-fixture.yaml` ships a Git bundle, not a
`dev:` tree.** Writing the fixture surfaced two mechanical problems, both
resolved without touching the design:

1. `git bundle create <file> <commit>` refuses with "Refusing to create empty
   bundle" for a bare 40-hex commit SHA; it needs a *named* ref to advertise.
   Bundling a branch name (tested: `reorg-assemblies`, same commit) creates
   the bundle fine but the clone warns `remote HEAD refers to nonexistent
   ref, unable to checkout` and leaves an empty working tree — confirmed by
   direct test, not assumed. The combination that works: `git worktree add
   --detach <dir> <commit>` to pin an exact commit without touching the main
   checkout, then `git -c pack.threads=1 bundle create <out> HEAD` from
   inside that detached worktree. `pack.threads=1` was confirmed to make the
   bundle byte-identical across three regenerations (same sha256); without it
   the plan's own warning about non-determinism would apply.
2. A `sources:` entry of kind `file:` must be tracked by Git
   (`check.rs:377`, `git ls-files --error-unmatch`) or `nex check` fails with
   "path is not tracked by Git". Staging the bundle would satisfy this, but
   the task rule for this unit is to leave everything unstaged. Fix: don't
   use a `sources:` `file:` entry at all. The assembly's own `files:` entries
   support a `source:` field (`system/files.rs:90`, `copy_file_source`) that
   reads directly from a path relative to the repository root at build time,
   with no Git-tracking check anywhere in `check.rs`. The bundle is placed at
   `/usr/share/nex/nex.bundle` via a `files:` entry, same as the
   `nex-init-manifests` override. Confirmed `fs::copy`'s per-copy mtime
   (which is not fixed the way `SOURCE_DATE_EPOCH` fixes build-script output)
   does not break `--check` reproducibility: the two-build checksum matched
   both times this was tried.

Fixture: extends `nex:base/nex-systemd.yaml`, adds `git` (same
`commit`/`manifest_ref` desktop-vwl uses), overrides
`/usr/local/bin/nex-init-manifests` to `git clone /usr/share/nex/nex.bundle
/nex/manifests` (keeping the original's `.git`-already-exists guard) instead
of `git init`, and ships `tests/nex-test-fixture.bundle` (7.8 MB, full history
from commit `f2f342f9008549aec28d10faa7758ec40afd733e`, the tip of
`reorg-assemblies` when this was built). `nex check tests/nex-test-fixture.yaml`
passes. `nex build tests/nex-test-fixture.yaml --single --check
--update-checksum` is strict two-build reproducible; checksum
`a22e42ab383d0c108c3fb6f2ada6899c6f3a21ff31aed3c05e61b112aee6b507`, stored at
`systems/nex-test-fixture/0.0.1`.

**2026-08-19: test 1 against the new fixture gets past the finding above,
then fails one level deeper, in `load_environment` itself.** Rerun (twice,
identical both times) with the harness pointed at
`systems/nex-test-fixture/0.0.1`:

    manifests-system-exists=true
    manifests-system-is-git=true
    manifests-commit-count=784
    manifests-pinned-env-blob-reachable=true
    manifests-worktree-exists=false
    build-cwd=/nex/manifests
    pkg-manifest-exists=true
    Created manifests worktree at /nex/users/root/manifests
    Created user environment at /nex/users/root
    Building package: cli/archive/gzip
    Error: Custom { kind: NotFound, error: "Failed to load environment blob 27b6e5dc7ad152c9a17c2cabfcc5ee93daa9bbf0: fatal: not a git repository (or any parent up to mount point /nex)\nStopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).\n" }
    build-exit=1

Full history is confirmed present and reachable (`manifests-commit-count=784`
matches `git rev-list f2f342f9... | wc -l` on the host that built the bundle;
the pinned blob itself is confirmed reachable by `git cat-file -e`, not just
inferred from commit count). The worktree machinery this plan flagged as
untested now runs: `/nex/manifests` clones with a resolvable `HEAD`,
`setup_user_manifests_worktree` (`repo.rs:261`) successfully runs `git
worktree add --detach /nex/users/root/manifests HEAD`, and `nex build` reaches
`load_environment` and starts building the package.

It still fails there, for a reason this plan had not identified: `run_build`
passes `opts.repo_path` to `load_environment` (`src/cli/src/build/package.rs:64`),
and `opts.repo_path` is the *zub store* (`/nex/users/root/repo` in user
context, `detect_context`, `repo.rs`), not the manifests Git repository.
`load_environment_blob` (`src/cli/src/build/env.rs:47`) runs `git -C
<repo_path> cat-file blob <sha>`. `git -C` changes directory then lets normal
upward discovery find the nearest `.git`; on a dev checkout this works by
coincidence, because `.nex/repo` happens to sit inside the Nex source
checkout's own `.git`. On an installed machine `/nex/users/root/repo` is a
sibling of `/nex/manifests` under `/nex`, not a descendant of it, so no amount
of history in `/nex/manifests` makes this resolve: the walk never reaches a
`.git` at all, on any machine, regardless of what `/nex/manifests` contains.
This is a different, more specific bug than "one-commit repository missing a
blob" — it is "blob resolution is wired to the wrong directory" — and it is
squarely EP018 territory (EP018 renames/reworks the `--repo` store concept;
see `.agents/knowledge/environment-pinning.md` and the `store` term in Context
and Orientation below). Per instruction, nothing was changed to route around
it: no repinning, no environment edits, no extra history.

**2026-08-19: tests 2 and 3 (`nex stage`) fail on this fixture because the
kernel bundle carries no `overlay.ko`, not because of anything in `nex`
itself.** `base/nex-systemd.yaml` names
`x86_64/pkg/core/kernel/linux/6.18.24/bundles/vm` for its kernel. That
bundle's component list (`pkg/core/kernel/linux.yaml:516-524`) is `boot,
drv-net-misc, drv-net-virt, drv-net-virtio, drv-virtio, lib, modules-meta,
net-misc` — no `fs-overlay`. The single output that carries the module,
`fs-overlay` (`pkg/core/kernel/linux.yaml:6141-6143`,
`/usr/lib/modules/6.18.24/kernel/fs/overlayfs/overlay.ko`), is a member of
exactly one bundle, `all-modules`, a much larger one. Confirmed on the guest:
`kernel/fs/overlayfs/` does not exist under `/usr/lib/modules/6.18.24` at
all; `modprobe overlay` fails ("Unknown symbol in module, or unknown
parameter"), `insmod` on the literal path fails ("No such file or
directory"), and `/proc/filesystems` has no `overlay` line. `nex stage`
(`src/cli/src/commands/stage.rs`) unconditionally does `mount -t overlay` on
`/usr/bin`, `/nex/pkg`, `/nex/env`, so it fails outright, the same way, every
time:

    mount: /usr/bin: unknown filesystem type 'overlay'.
    Error: Custom { kind: Other, error: "Failed to mount overlay on /usr/bin" }

This blocks `nex install --system` too, since it requires staging first.
Test 4 (deploy/rollback) needs neither staging nor overlayfs and is
unaffected — it passed cleanly, twice in a row.

Nothing was changed to route around this: swapping in `bundles/all-modules`
(or hand-adding the `fs-overlay` output) would make tests 2 and 3 pass, but
that is a choice about what this test fixture's kernel should carry, not a
mechanical follow-on from anything asked for in this unit, so it was left
alone and is reported here instead of decided.

## Decision Log

- Decision: The harness pre-populates the guest's system store (`/nex/repo`)
  with refs pulled from the host's own build store before boot: `tig`
  (`outputs/bin`, small, already built, not part of this fixture's own
  package list) for tests 2-3, and `systems/nex-systemd/0.0.1` (a
  different already-built system) for test 4's deploy target.
  Rationale: Every test here must avoid the environment bug, which only
  triggers on a build. The guest still runs every `stage`/`install`/
  `discard`/`commit`/`deploy`/`rollback` command itself; only the ingredient
  each command needs already built is placed there first, the same way the
  fixture's own installed packages are host-built before boot. `zub pull`
  hardlinks content-addressed objects, so seeding both refs costs about 425 MB
  and well under two seconds.
  Date/Author: 2026-08-19 / Claude

- Decision: Tests that cross a reboot (3 and 4) are one guest script file
  taking a phase argument, driven by a per-test phase list
  (`TEST_PHASES` in `scripts/test-machine-operations.sh`) that the host
  steps through, calling the `reboot` verb between phases and stopping the
  test at the first failed phase without attempting the phases or reboots
  after it.
  Rationale: Keeps "one script per test" from the Plan of Work while
  giving the host what it needs to own the reboot, per the harness design.
  State a later phase needs (e.g. "is the installed binary still there")
  comes from the guest's own persistent files (or, for test 4, its
  `/proc/cmdline`), never from a value the host computed and handed back in.
  Date/Author: 2026-08-19 / Claude

- Decision: Drive every operation from inside the guest over SSH, never from the
  host.
  Rationale: The host staging a deployment is the operation under test. A test
  that performs it on the guest's behalf proves only that the harness works.
  Date/Author: 2026-08-19 / Claude

- Decision: Give each test its own qcow2 overlay over one shared backing
  image.
  Rationale: Every test then starts from an identical known deployment, a
  failure names one operation rather than a sequence, and discarding state is
  deleting a file. The existing script builds guest state inline, which couples
  tests together.
  Date/Author: 2026-08-19 / Claude

- Decision: Tests emit `key=value` lines on stdout and exit non-zero on
  failure. The host parses no prose.
  Rationale: This is what made the graphical smoke test legible.
  `center-pixel=srgb(240,0,255)` is checkable; "looks right" is not.
  Date/Author: 2026-08-19 / Claude

- Decision: The fixture must be a `nex_structure: true` assembly.
  Rationale: A flat assembly has no `/nex` at all. Checked on 2026-08-19: a
  built `edgebox-rootfs` root contains no `nex` directory and no `nex` binary,
  so it cannot build, install, deploy, or roll back anything.
  Date/Author: 2026-08-19 / Claude

- Decision: The fixture ships a Git bundle with full history and clones from it
  on first boot, rather than shipping a file tree.
  Rationale: A machine always needs full history so every pinned document is
  reachable; this is a design requirement, not a measurement. Shipping a tree
  and running `git init` over it cannot satisfy it. This anticipates EP018 in
  one test assembly, which also proves the mechanism before EP018 generalises
  it.
  Date/Author: 2026-08-19 / Human

- Superseded: Write `tests/nex-test-fixture.yaml`, extending
  `nex:base/nex-systemd.yaml` and adding a `git` package plus a populated
  `/usr/share/nex/manifests`.
  Rationale: Measured, not assumed. `nex-systemd` alone boots with an empty
  `/nex/manifests` and no git, so no test that touches manifests can run.
  Only desktop-vwl ships those today, and it costs roughly 7 GB per build.
  Date/Author: 2026-08-19 / Claude

- Superseded: Use `base/nex-systemd.yaml` as the fixture and inject SSH access
  into the var tree, rather than writing a fixture assembly.
  Rationale: The access an assembly may not carry is exactly the access that
  belongs in machine state, and `scripts/prepare-qemu-test-identity.sh` already
  writes it there. A fixture assembly would duplicate that helper and would
  have to be exempted from a policy it should never have tripped.
  Date/Author: 2026-08-19 / Claude

- Superseded: If `base/nex-systemd.yaml` will not serve, write a purpose-built
  `tests/nex-test-fixture.yaml` rather than using `examples/desktop-vwl`.
  Rationale: desktop-vwl builds a roughly 7 GB root carrying a whole desktop
  stack that none of these tests exercise, and every fixture rebuild would
  pay for it. A purpose-built fixture is small, fast, and its contents are
  chosen by what the tests need. It lives in `tests/` rather than `base/` or
  `examples/`, because a fixture is run directly, which `base/` is not for, and
  because it needs a known account and a fixed key, which
  `scripts/check-generic-assembly-policy.sh:29` rejects in anything it scans.
  Date/Author: 2026-08-19 / Human

- Decision: Assert on state the machine reports about itself, never on a value
  the test supplied.
  Rationale: Checking that a deployment directory holds the checksum the test
  just wrote there proves nothing about booting.
  Date/Author: 2026-08-19 / Claude

- Decision: Generate bundles with `git -c pack.threads=1 bundle create`, from
  an explicit commit reached through a detached worktree.
  Rationale: Measured 2026-08-19. Three default runs produced three different
  sha256 values; three with `pack.threads=1` produced one, at the same size, so
  the nondeterminism is parallel work-splitting rather than content. Adding
  `pack.window=0 pack.depth=0` is also deterministic but grows 7.4 MB to
  24.9 MB. `git bundle create <file> <commit-sha>` refuses with "empty bundle"
  because it needs a named ref, and bundling a branch yields a clone with an
  empty tree, so a detached worktree supplies a HEAD to bundle. Carried from
  the deleted ExecPlan 018 and since confirmed in `scripts/make-test-bundle.sh`.
  Date/Author: 2026-08-19 / Claude

- Decision: Record each bundle's sha256 in the manifest, as `cargo_lock` does.
  Rationale: It makes the snapshot a verified input rather than whatever the
  build host produced, and it restores pinning, which `dev:` blocks
  (`link.rs:284`). Carried from the deleted ExecPlan 018.
  Date/Author: 2026-08-19 / Claude

- Decision: Rename `/nex/repo` to `/nex/store` and `--repo` to `--store`.
  Rationale: Two different things were called "repo", a zub object store and a
  Git repository, and the confusion cost real time on 2026-08-19. The code
  already prefers the other word: `main.rs:18` declares `pub mod store` and the
  dependency at `Cargo.toml:27` is the `zub-store` crate. Carried from the
  deleted ExecPlan 018.
  Date/Author: 2026-08-19 / Human

- Decision: Fix the product defects here rather than deferring them, and merge
  ExecPlan 018 into this one.
  Rationale: This plan's acceptance is that all four tests pass, and three
  cannot pass while the defects stand. A plan that forbids itself from reaching
  its own acceptance is malformed. Merging also removes an ordering trap: the
  bundle alone does not make a machine able to build, because environment
  resolution reads the wrong directory.
  Date/Author: 2026-08-19 / Human

## Outcomes & Retrospective

`scripts/test-machine-operations.sh` exists and runs all four tests, `--test
<name>` runs one alone, and every test starts from a fresh qcow2 overlay
over one shared backing image built from `tests/nex-test-fixture.yaml`
(`systems/nex-test-fixture/0.0.1`, checksum
`a22e42ab383d0c108c3fb6f2ada6899c6f3a21ff31aed3c05e61b112aee6b507`, strict
two-build reproducible). Verified twice in a row: `build-package`,
`temporary-install`, and `persistent-install` FAIL, identically each time;
`deploy-and-rollback` PASSes, identically each time, including two reboots.
The deliberately-broken-command check (a typo'd deploy ref) made exactly one
test fail, carrying the guest's own error text, while the other three kept
their unrelated pre-existing failures.

Three of four tests FAIL, and every one of those three is a legitimate,
reproducible finding this plan set out to get, not a harness defect:

1. `build-package`: a machine cannot build a package. Full history in
   `/nex/manifests` (shipped via Git bundle, not a `git init` snapshot) makes
   the manifests worktree and historical blob lookups work, but
   `load_environment` resolves environment blobs against the zub store, which
   is never inside a Git repository on an installed machine. EP018 territory.
2. `temporary-install` and `persistent-install`: a machine cannot stage a
   package for install. `nex stage` unconditionally mounts OverlayFS, and this
   fixture's kernel bundle (`base/nex-systemd.yaml`'s choice, not something
   this plan changed) does not carry `overlay.ko`. `deploy-and-rollback`
   proves this is specific to staging, not to the fixture generally: it needs
   no OverlayFS and passes cleanly.

So the plan's four-test premise holds up: a person sitting at an installed
Nex machine can put a new system version on it and undo that (test 4,
proven, content-based, across two real reboots), but cannot build a package or
stage an install today, for two distinct, now-precisely-located reasons
neither of which was known before this plan.

What was not done: the two bugs above were not fixed, per instruction — that
is EP018's job for the store/environment issue and an open design question
(reported, not decided) for the kernel bundle's module set. `reboot` and
`expect_deployment`-equivalent logic are now exercised (test 4), but
`expect_deployment` itself as a literal harness function is unused by any
test; tests use `/proc/cmdline` parsing inline instead since the
assertions needed were about content markers, not a single deployment-name
equality check. No stray QEMU processes or Git worktrees were left behind by
any run.

**What this plan got wrong, twice.** Two fixture decisions were recorded and
then overturned by measurement. First that `examples/edgebox-rootfs.yaml` could
serve, when a built edgebox root has no `/nex` at all. Then that
`base/nex-systemd.yaml` could serve unaided, when it ships neither a manifest
tree nor `git`. Both were caught by running something rather than by reasoning
about it, which is the right order, but writing the plan around an unverified
assumption cost two cycles. A plan that names a fixture should name the
property the fixture must have, and check it first.

**The fixture's bundle is generated, never committed.** `scripts/make-test-bundle.sh`
writes `tests/nex-test-fixture.bundle`, and `.gitignore` excludes
`/tests/*.bundle`. Committing it would put a 7.7 MB copy of this repository
inside this repository, adding its whole size to history on every
regeneration, which is the mistake the librsvg vendor tarball made. Anyone
building the fixture runs that script first. It is reproducible: two runs
produced sha256 `fc268d2210446e95…` both times.

## Context and Orientation

Terms used here, defined once:

- **store**: the content-addressed object store at `/nex/repo` on a machine, a
  zub repository. Note that the CLI flag is `--repo`, which collides with the
  Git sense of the word. EP018 renames it.
- **deployment**: a system root at `/nex/deployments/<checksum>.<n>`, with
  `/nex/current` symlinked at the active one.
- **manifests repository**: `/nex/manifests`, a Git repository the machine can
  edit, plus a per-user `git worktree` of it at `/nex/users/<user>/manifests`
  created by `repo.rs:261`.

Files this plan touches:

- `scripts/qemu-test-live-upgrade.sh`, read for reusable parts, not extended.
- `scripts/test-machine-operations.sh`, new, the entry point.
- `scripts/machine-tests/*.sh`, new, one per test, run on the guest.

The CLI verbs a user has: `build`, `stage`, `install`, `remove`, `discard`,
`commit`, `switch`, `deploy`, `upgrade`, `deployments`, `status`, `rollback`,
`gc`.

## Plan of Work

Five phases, in this order. The order matters: each phase turns a failing test
green, and a later phase would be unverifiable without the earlier one.

**A. Harness and tests.** Done. It establishes measurement, so every later
phase is checked rather than argued.

**B. Fix environment resolution.** The fixture already ships full history, so
this single fix should make `build-package` pass on its own. Doing it before
anything else proves it in isolation, against a machine whose manifests are
known good.

**C. Generalise the bundle.** Add the `git_bundle` source kind, move desktop-vwl
onto it, rewrite `nex-init-manifests` to clone rather than `git init`, and
delete `.nex-dev-prepare`. The fixture's override of `nex-init-manifests` then
becomes redundant and is removed, which is itself the proof that the general
mechanism works.

**D. Rename the store.** Mechanical, but it touches on-disk layout, so it needs
a migration and it comes after the behaviour changes rather than tangled with
them.

**E. Rebuild and prove.** Rebuild every assembly, record checksums, run the
full suite green.

## Concrete Steps

1. Resolve environment blobs against the repository that owns the manifest
   rather than against the store. `build/package.rs:64` and
   `system/build.rs:98` both pass `opts.repo_path`; `env.rs:49` runs
   `git -C <path> cat-file blob`. `ManifestRepositories` already knows the
   owning root. Add a test that fails when the lookup uses the store.
2. Run `scripts/test-machine-operations.sh --test build-package`. It must pass.
   If it does not, report why before continuing.
3. Add `git_bundle: <commit>` to `Source` in `manifest/types.rs` and a
   `fetch_git_bundle` beside `fetch_cargo_lock` in `outputs/sources.rs`,
   generating with `pack.threads=1` from a detached worktree and verifying a
   recorded sha256. `scripts/make-test-bundle.sh` is the working recipe.
4. Prove the generated bundle is byte-identical across two strict builds.
5. In `examples/desktop-vwl/desktop-vwl.yaml`, replace the six `dev:` sources
   with one `git_bundle` source installed at `/usr/share/nex/nex.bundle`.
   Delete the extraction loop and the `sed -i '/checksum:/d'` that existed only
   because a working-tree tarball could change between the two `--check`
   passes. Confirm the image no longer carries `vendor/zub-store`, which is
   3.8 GB of another repository's build output.
6. Rewrite `/usr/local/bin/nex-init-manifests` in `base/nex-systemd.yaml` to
   `git clone /usr/share/nex/nex.bundle /nex/manifests`, keeping the guard that
   exits when `/nex/manifests/.git` exists. Then delete the fixture's override
   and confirm the tests still pass.
7. Delete `src/cli/.nex-dev-prepare`. Keep the `dev:` source kind, which stays
   the deliberate unpinnable escape hatch for local iteration.
8. Rename `/nex/repo` to `/nex/store` and `--repo` to `--store` across the 29
   references, with a migration that is safe to run twice.
9. Rebuild every assembly with `--single --check --update-checksum`, record the
   checksums here, and run the full suite.

## Validation and Acceptance

The plan is done when:

- `scripts/test-machine-operations.sh` prints `PASS:` for all four tests, from
  a fresh overlay, in any order, and twice in a row.
- A deliberately broken guest command makes exactly one test fail, and the
  failure names the operation and carries the guest's error line.
- `deploy-and-rollback` proves the reboot by content, not by a string the test
  supplied.
- On a booted machine, `git -C /nex/manifests log --oneline | wc -l` exceeds 1
  and `git -C /nex/manifests cat-file -t 27b6e5dc...` prints `blob`.
- The shipped image carries `/usr/share/nex/nex.bundle` and no
  `/usr/share/nex/manifests` tree, and no `vendor/zub-store`.
- `nex link examples/desktop-vwl/desktop-vwl.yaml` succeeds, where the `dev:`
  sources make it refuse today.
- `/nex/store` exists on a machine upgraded from an older deployment, with its
  objects intact.
- Two strict builds of every assembly agree.

## Idempotence and Recovery

Tests are safe to rerun: each starts by discarding its overlay and creating a
new one from the fixture. The fixture is rebuilt only when its assembly
checksum changes. If a guest hangs, the harness kills QEMU on timeout and keeps
the serial log under the artifact directory. Nothing writes to the host store.

## Artifacts and Notes

Keep per-run artifacts under `.nex/tmp/machine-tests/<test>/`: serial log,
guest stdout, and the guest journal on failure. Do not add them to Git.

Note for whoever runs this: `.nex-dev-prepare` uses `mktemp -d`, and `TMPDIR`
defaults to `/tmp`, which is tmpfs on this host. Set
`TMPDIR=<repo>/.nex/tmp/mktemp` for any build step, or the run dies with
`Disk quota exceeded`.

## Interfaces and Dependencies

This plan adds a test interface and changes two machine interfaces.

Test interface: `scripts/test-machine-operations.sh` with `--test <name>`, the
test scripts under `scripts/machine-tests/`, the fixture
`tests/nex-test-fixture.yaml`, and `scripts/make-test-bundle.sh`, which
generates the fixture's bundle. That bundle is gitignored and never committed:
a bundle of this repository stored inside it would add its whole size to
history on every regeneration.

New manifest interface: the `git_bundle` source kind, usable by any manifest.

Changed machine interfaces: `/nex/store` replaces `/nex/repo`, `--store`
replaces `--repo`, and `/usr/share/nex/nex.bundle` replaces the
`/usr/share/nex/manifests` tree.

Depends on QEMU, `ssh`, `ssh-keygen`, a built fixture, and the zub binary named
by `ZUB_BIN`. ExecPlan 019, splitting the CLI into its own repository, is
paused and does not depend on this plan, though it is easier afterwards.

One product question stays open and deliberately does not block this plan:
whether `base/nex-systemd.yaml` should carry the kernel's `fs-overlay` output
so that a shipped Nex system can stage an install. The fixture adds it, so the
tests can run; a machine built from `nex-systemd` alone still cannot stage. See
`.agents/knowledge/kernel-and-boot.md`.
