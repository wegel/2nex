# Get a machine what it does not have

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

A person on an installed Nex machine asks for a package. Two things can be
true of any package in its dependency closure: some other machine has already
built it, or nobody has. The machine must handle both. It pulls what a remote
store holds, and it builds locally what nothing holds. Everything is
reproducible, so the two paths produce the same bytes and the choice between
them is only about time.

That is the use the whole project is for, and no test covers either half of
it.

ExecPlan 017 proved four operations on a machine, but every one of them ran
against a store the harness had already filled from the build host.
`seed_system_store()` pulls tig's runtime closure and `seed_gzip_build_closure()`
pulls all 15 of gzip's direct build dependencies before the machine boots. So
`build-package` proves a guest compiles. It proves nothing about how a guest
obtains what it compiles against. The flag it uses, `--single`, is a developer's
flag: build this one manifest, assume its dependencies are already resolvable.

Underneath the test gap sits a product gap. `nex install` does build a package
that is not cached (`commands/install.rs:155`), but through
`build_package_to_user_repo`, which calls `build::build_single`
(`commands/install.rs:541`). The graph walker that fills gaps,
`build_with_dependencies` (`build/orchestration.rs:117`), is reachable only
from a bare `nex build` with no `--single`. So `nex install X` succeeds when
X's direct dependencies already resolve and fails otherwise. It never builds
them. Deciding what it should do instead is the substance of this plan, not a
detail of it.

The pull half exists and is wired: `Store::resolve_ref` tries the primary
store, then each fallback, then `pull_from_remote` (`store/mod.rs:228-243`),
over a local path or SSH. It has one unit test,
`resolve_ref_pulls_from_configured_local_remote`, and has never run on a
booted machine.

## Progress

- [ ] Decide what `nex install` does when a dependency is missing everywhere.
- [ ] Give the guest kernel `CONFIG_FUSE_FS` and `CONFIG_VIRTIO_FS`.
- [ ] Export the host store into the guest read-only, and configure it as a
      remote in the machine's store.
- [ ] Make the harness's seeding selective, so a closure can be deliberately
      absent from the machine's own store.
- [ ] Add a test that pulls a package's closure from the remote.
- [ ] Add a test that builds locally what the remote does not have.
- [ ] Fix `ref_is_available` so a dependency in a fallback store is not queued
      for a rebuild.
- [ ] Rebuild the kernel-carrying assemblies and run every suite green.

## Surprises & Discoveries

Nothing yet.

## Decision Log

- Decision: Export the host's own zub store into the guest over virtiofs,
  read-only, rather than building a second disk image per run.
  Rationale: Measured 2026-08-20. The host store is 26 GB, so copying it is
  out, and copying only the closure still costs a fresh image every run
  (`build_backing_image` already starts with `rm -rf "$WORK_DIR"`,
  `test-machine-operations.sh:517`, so the harness rebuilds everything each
  time; this plan must not add to that). A read-only export costs nothing per
  run and nothing per test, and every test in a run shares one export. The
  guest pulls only the closure it asks for. `virtiofsd` is present at
  `/usr/lib/virtiofsd` with QEMU 11.1.0. `pull_local` reads the source and
  writes only to the destination (`zub/src/transport/pull.rs:33-47`), so a
  read-only export is sufficient and cannot corrupt the developer's store.
  Date/Author: 2026-08-20 / Claude

- Decision: Add the kernel symbols rather than work around their absence.
  Rationale: `pkg/core/kernel/linux.yaml:168` starts from `make allnoconfig`,
  so nothing is enabled unless a fragment enables it, and no fragment mentions
  FUSE, virtiofs or 9p. The human's instruction was explicit: not having them
  now is not a problem, add what is needed. New symbols go in
  `fragments/builtins.frag` under the verification loop at `linux.yaml:233`,
  the same loop that caught `CONFIG_IPC_NS` silently dropping for want of
  `CONFIG_SYSVIPC`.
  Date/Author: 2026-08-20 / Claude

- Decision: SSH to the host is the fallback, not the design.
  Rationale: It would work today with no kernel change. QEMU user-mode
  networking already routes `10.0.2.2` to the host, the fixture ships
  `/usr/bin/ssh`, and `RemoteSource::Ssh` parses `user@host:/path`
  (`store/mod.rs:37`). The cost is that the suite starts requiring the
  developer's machine to run sshd and accept a key from a VM. Everything else
  in the harness needs only QEMU and a built fixture.
  Date/Author: 2026-08-20 / Claude

- Decision: 9p stays available as a second option.
  Rationale: `-virtfs local,…,readonly=on` needs no daemon and makes
  read-only a QEMU-side guarantee rather than a guest-side mount option. It is
  slower per file, and the closure here is gcc, glibc and binutils dev
  bundles, which is exactly where metadata cost is felt. Both symbol sets are
  small, so carry both and choose per test if virtiofs proves awkward.
  Date/Author: 2026-08-20 / Claude

## Outcomes & Retrospective

Nothing yet.

## Context and Orientation

Read first: `.agents/execplans/done/017-machine-operation-tests.md`, whose
harness this plan extends, and `.agents/knowledge/machine-self-hosting.md`.

The pieces this plan touches:

- `scripts/test-machine-operations.sh`, the harness. `stage_root_and_var`
  builds the machine image and calls `seed_system_store()` unconditionally,
  for every test. `boot()` and `stop_guest()` own the QEMU process and are
  where a virtiofsd daemon belongs. `qemu_args` builds the command line.
- `src/cli/src/store/mod.rs`, the layered lookup: primary store, fallback
  chain, then remotes.
- `src/cli/src/build/orchestration.rs` and `orchestration/graph.rs`, the
  dependency graph and what counts as already built.
- `src/cli/src/commands/install.rs`, which builds a missing package the narrow
  way.
- `pkg/core/kernel/fragments/builtins.frag` and `pkg/core/kernel/linux.yaml`.

One known defect is in scope only if it blocks a test:
`scripts/test-live-upgrade-hardlinks.sh` fails with `uid 0 not mapped in
namespace`, identically before and after the 2026-08-20 store rename, so it is
a pre-existing failure of the same class ExecPlan 017 root-caused for the
machine harness. Confirm before assuming it is new.

## Plan of Work

**Phase A: decide what `nex install` owes the user.** Read
`build_with_dependencies` and `build_single` and write down what each does when
a dependency is missing. Then decide: does `nex install` build the closure, or
does it refuse with a message naming what is missing and the command that
would build it? Record the decision and its reason before writing code. This
is the plan's design question and everything else is mechanics.

**Phase B: give the guest a remote.** Kernel symbols, virtiofsd lifecycle in
the harness, the memfd and `-numa` arguments in `qemu_args`, the guest-side
mount, and `[[remotes]]` in the machine's store config. Prove it by pulling one
small ref by hand from a booted guest before writing a test around it.

**Phase C: make seeding selective.** Today every test gets everything. A test
needs to say which refs its machine starts with, so a closure can be absent
from the machine and present on the remote, or absent from both.

**Phase D: the two tests.** One package whose closure the remote has, pulled.
One package whose closure nothing has, built. Both end with a binary that runs.

**Phase E: the fallback-chain fix, rebuilds, and every suite green.**

## Concrete Steps

1. Read `build_with_dependencies` (`build/orchestration.rs:117`) and
   `ref_is_available` (`build/orchestration/graph.rs:282`) and record what
   each does with a dependency that is missing locally, present on a remote,
   or present only in a fallback store.
2. Decide and record the `nex install` behaviour. Do not implement before the
   decision is written down.
3. Add `CONFIG_FUSE_FS=y` and `CONFIG_VIRTIO_FS=y` to
   `pkg/core/kernel/fragments/builtins.frag`, and add both to the symbol check
   at `linux.yaml:233`. Rebuild the kernel and confirm both are `=y` in the
   produced config.
4. Start virtiofsd from `boot()` and reap it in `stop_guest()`. Add
   `-object memory-backend-memfd,id=mem,size=${MEMORY}M,share=on` and
   `-numa node,memdev=mem` to `qemu_args`.
5. Mount the export read-only in the guest and add `[[remotes]]` naming it to
   the machine store's `config.toml`. Pull one ref by hand over SSH to prove
   the path before automating it.
6. Give the harness a per-test seed set, so a test declares what its machine
   starts with rather than getting everything.
7. Write the pull test: a package absent from the machine, present on the
   remote, installed and run.
8. Write the build test: a package whose closure nothing has, built locally
   and run. Expect this to fail against `nex install` as it stands; that is
   the point of phase A.
9. Fix `ref_is_available` to consult the fallback chain.
10. Rebuild every assembly the kernel change reaches, with
    `--single --check --update-checksum`, and record the checksums here.

## Validation and Acceptance

The plan is done when:

- A booted machine pulls a package's closure from a remote it did not have,
  and the installed binary runs.
- A booted machine builds a package whose closure no store holds, and the
  installed binary runs.
- Both facts are read from the machine, not from a string the test supplied,
  and the pull is proven by object count rather than by absence of an error.
- `nex install` behaves as phase A decided, and a test names that behaviour.
- A dependency present only in a fallback store is not rebuilt.
- Every ExecPlan 017 test still passes, five green twice in a row.
- Two strict builds of every assembly agree.
- The suite is no slower per run than it was before the export existed.

## Idempotence and Recovery

Tests stay safe to rerun: each discards its overlay and creates a new one from
the fixture. The export is read-only, so nothing a guest does can reach the
host store. If virtiofsd is left running by a crashed test, `stop_guest` must
reap it the way it reaps QEMU, and a stale daemon must not make the next run
fail silently.

## Artifacts and Notes

Record here: the closure sizes actually pulled, the time a pull takes over
virtiofs against 9p if both get measured, and the per-run cost of the export
against the disk-image approach it replaced.

`TMPDIR` still defaults to `/tmp`, which is tmpfs on this host. Set
`TMPDIR=<repo>/.nex/tmp/mktemp` for any build step.

## Interfaces and Dependencies

New test interface: a remote store exposed to the guest, and a per-test seed
set in `scripts/test-machine-operations.sh`.

Possible new machine behaviour: what `nex install` does when a dependency is
missing everywhere. Phase A decides whether that changes.

New kernel symbols: `CONFIG_FUSE_FS` and `CONFIG_VIRTIO_FS`, which every
assembly carrying the kernel inherits.

Depends on ExecPlan 017's harness and fixture, and on virtiofsd being
installed. ExecPlan 019, splitting the CLI into its own repository, is paused
and does not depend on this plan.
