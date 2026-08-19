# Test the operations a user performs on an installed machine

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this plan, one command proves that a person sitting at an installed Nex
machine can do the four things Nex exists to let them do:

    scripts/test-machine-operations.sh

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

Journey 1 also settles a question this repository cannot currently answer from
source alone. Package manifests name their build environment by Git blob SHA,
and 476 of them name `27b6e5dc`, a historical revision of `env/standard.yaml`
(see `.agents/knowledge/environment-pinning.md`). `/nex/manifests` is created on
first boot by `git init` plus one commit of the shipped snapshot
(`base/nex-systemd.yaml:222`), and a repository with one commit cannot hold a
past revision of a file. Either the machine gets its manifests some other way,
or journey 1 fails. Both outcomes are useful, and neither is known today.

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
      and `expect_deployment` are exercised by no journey yet (journey 1 needs
      neither); they are written and match the reference script's proven
      `assert_current_deployment`/reboot-by-killing-qemu pattern, but unproven
      end to end until journeys 3/4 exist.
- [x] (2026-08-19) Wrote journey 1 (`scripts/machine-journeys/build-package.sh`)
      and recorded what it reports: FAIL, deterministically, three runs in a
      row. See Surprises & Discoveries.
- [ ] Write journeys 2, 3, 4.
- [ ] Wire the four journeys into `scripts/test-machine-operations.sh`. (Only
      journey 1 is wired; `ALL_JOURNEYS` in the script is a one-element array
      ready to grow.)
- [ ] Promote durable findings into `.agents/knowledge/`. (Scratch notes added
      to `.agents/SCRATCH_KNOWLEDGE.md`; promotion deferred to plan
      completion, once journeys 2-4 either confirm or revise this reading.)

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
as the fixture.** Journey 1 ran against it and failed one step earlier than
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

The environment blob question is therefore still unanswered: the build never
got far enough to resolve an environment.

`tests/nex-test-fixture.yaml` must be written after all. It extends
`nex:base/nex-systemd.yaml` and adds the two things that make a machine
self-hosting: a `git` package, and a populated `/usr/share/nex/manifests`.

**2026-08-19: journey 1 fails, and the cause is one step earlier than the
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

## Decision Log

- Decision: Drive every operation from inside the guest over SSH, never from the
  host.
  Rationale: The host staging a deployment is the operation under test. A test
  that performs it on the guest's behalf proves only that the harness works.
  Date/Author: 2026-08-19 / Claude

- Decision: Give each journey its own qcow2 overlay over one shared backing
  image.
  Rationale: Every journey then starts from an identical known deployment, a
  failure names one operation rather than a sequence, and discarding state is
  deleting a file. The existing script builds guest state inline, which couples
  journeys together.
  Date/Author: 2026-08-19 / Claude

- Decision: Journeys emit `key=value` lines on stdout and exit non-zero on
  failure. The host parses no prose.
  Rationale: This is what made the graphical smoke test legible.
  `center-pixel=srgb(240,0,255)` is checkable; "looks right" is not.
  Date/Author: 2026-08-19 / Claude

- Decision: The fixture must be a `nex_structure: true` assembly.
  Rationale: A flat assembly has no `/nex` at all. Checked on 2026-08-19: a
  built `edgebox-rootfs` root contains no `nex` directory and no `nex` binary,
  so it cannot build, install, deploy, or roll back anything.
  Date/Author: 2026-08-19 / Claude

- Decision: Write `tests/nex-test-fixture.yaml`, extending
  `nex:base/nex-systemd.yaml` and adding a `git` package plus a populated
  `/usr/share/nex/manifests`.
  Rationale: Measured, not assumed. `nex-systemd` alone boots with an empty
  `/nex/manifests` and no git, so no journey that touches manifests can run.
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
  stack that none of these journeys exercise, and every fixture rebuild would
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

## Outcomes & Retrospective

(fill in at completion)

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
- `scripts/machine-journeys/*.sh`, new, one per journey, run on the guest.

The CLI verbs a user has: `build`, `stage`, `install`, `remove`, `discard`,
`commit`, `switch`, `deploy`, `upgrade`, `deployments`, `status`, `rollback`,
`gc`.

## Plan of Work

Build the harness first and prove it with the cheapest journey, then add the
rest. The harness is three pieces:

**Fixture.** One bootable disk built once from an assembly this repository
already builds, then a qcow2 overlay per journey with the fixture as backing
file. Journeys never mutate the fixture.

**Guest contract.** A journey is a shell script copied to the guest and run
there. It prints `key=value` facts and exits non-zero on failure. The host
collects stdout, the serial log, and the guest journal on failure.

**Transitions.** `boot`, `run`, `reboot`, `expect_deployment`. A reboot is a
step the harness owns, because journeys 3 and 4 cross one and nothing today
does that from inside the guest.

## Concrete Steps

1. Build the fixture from `base/nex-systemd.yaml`. It must be a
   `nex_structure: true` assembly, and that is the cheapest one. Do not use
   `examples/edgebox-rootfs.yaml`: it is flat, so a built edgebox root has no
   `/nex` directory, no `/nex/repo` store, no `/nex/manifests`, and no `nex`
   binary, and every journey here needs all four. The `nex_structure: true`
   assemblies are `base/nex-minimal.yaml`, `base/nex-systemd.yaml`,
   `installer/installer.yaml`, `examples/desktop-vwl/desktop-vwl.yaml`, and
   `examples/desktop-dev.yaml`. Confirm the fixture boots with SSH before
   building anything on top of it. If `nex-systemd` is not suitable, write
   `tests/nex-test-fixture.yaml`, a minimal `nex_structure: true` assembly
   carrying only what these journeys need: systemd, sshd, the `nex` binary, a
   store, and a manifests repository. Do not fall back to
   `examples/desktop-vwl/desktop-vwl.yaml`.
2. Build the fixture and keep its path and checksum in the plan.
3. Write `scripts/test-machine-operations.sh` with the four verbs above and a
   `--journey <name>` flag so one journey can run alone.
4. Journey `build-package`: guest runs
   `nex build pkg/cli/archive/gzip.yaml --single` from its manifests worktree.
   `gzip` has 16 dependencies and 114 lines, the smallest real candidate.
   Report `build-exit`, `output-ref`, and on failure the first error line.
5. Journey `temporary-install`: `nex stage`, `nex install`, assert the binary
   runs, `nex discard`, assert it is gone and the previous state is intact.
6. Journey `persistent-install`: same, but `nex commit`, then reboot, then
   assert the package is still there.
7. Journey `deploy-and-rollback`: guest runs `nex deploy <ref>`, reboots,
   asserts `nex status` names the new deployment and that a marker unique to
   that version is present in the running root, then `nex rollback`, reboots,
   asserts the old deployment is back.
8. Record what journey 1 revealed about `/nex/manifests` and environment
   resolution in `Surprises & Discoveries`, then promote it.

## Validation and Acceptance

The plan is done when:

- `scripts/test-machine-operations.sh` runs all four journeys and prints one
  `PASS:` line per journey plus a final summary.
- Each journey passes from a fresh overlay, in any order, and twice in a row.
- A deliberately broken guest command makes exactly one journey fail, and the
  failure output names the operation and carries the guest's error line.
- Journey 4 proves the reboot by content, not by a string the test supplied.

## Idempotence and Recovery

Journeys are safe to rerun: each starts by discarding its overlay and creating a
new one from the fixture. The fixture is rebuilt only when its assembly
checksum changes. If a guest hangs, the harness kills QEMU on timeout and keeps
the serial log under the artifact directory. Nothing writes to the host store.

## Artifacts and Notes

Keep per-run artifacts under `.nex/tmp/machine-tests/<journey>/`: serial log,
guest stdout, and the guest journal on failure. Do not add them to Git.

Note for whoever runs this: `.nex-dev-prepare` uses `mktemp -d`, and `TMPDIR`
defaults to `/tmp`, which is tmpfs on this host. Set
`TMPDIR=<repo>/.nex/tmp/mktemp` for any build step, or the run dies with
`Disk quota exceeded`.

## Interfaces and Dependencies

This plan adds no product runtime interface. It adds a test interface:
`scripts/test-machine-operations.sh` and the journey scripts beside it.

It depends on QEMU, `ssh`, `ssh-keygen`, a built `nex_structure: true` fixture
assembly, and the zub binary named by `ZUB_BIN`. It does not depend on EP018 or EP019, and it should
land before both, because EP018 changes how manifests reach a machine and these
journeys are how that change gets verified.
