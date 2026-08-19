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

- [ ] Read `scripts/qemu-test-live-upgrade.sh` end to end and list the parts
      worth reusing: disk assembly, `ensure_assert_key`, `ssh_probe`,
      `wait_for_ssh`, serial logging.
- [ ] Build the fixture image from `base/nex-systemd.yaml` and prove an overlay
      boots from it with SSH reachable.
- [ ] Implement the harness verbs: `boot`, `run`, `reboot`, `expect_deployment`.
- [ ] Write journey 1 and record what it reports, pass or fail.
- [ ] Write journeys 2, 3, 4.
- [ ] Wire the four journeys into `scripts/test-machine-operations.sh`.
- [ ] Promote durable findings into `.agents/knowledge/`.

## Surprises & Discoveries

(none yet)

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
   building anything on top of it; if `nex-systemd` does not, fall back to
   `examples/desktop-vwl/desktop-vwl.yaml` and record why.
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
