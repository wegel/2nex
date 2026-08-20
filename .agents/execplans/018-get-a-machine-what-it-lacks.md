# Get a machine what it does not have

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

A person on an installed Nex machine asks for a package. The machine pulls
each package that a configured remote store already holds and builds each
package that no store holds. The person runs one `nex install` command in both
cases. Reproducible builds make the local and remote results identical; only
the time differs.

ExecPlan 017 proved five machine operations, but its harness filled the guest
store from the build host before every boot. `seed_system_store()` pulls tig's
runtime closure, and `seed_gzip_build_closure()` pulls gzip's build inputs.
Those tests prove that a guest can install and compile after it has every
required ref. They do not prove how the guest obtains a missing ref.

`nex install` currently builds an uncached target through
`build_package_to_user_repo`, which calls `build::build_single`
(`src/cli/src/commands/install.rs`). That call assumes every build dependency
already resolves. `build_with_dependencies` in
`src/cli/src/build/orchestration.rs` already walks the dependency graph, pulls
available refs from configured remotes, and builds the remaining nodes. This
plan makes `nex install` use that graph.

`Store::resolve_ref` already checks the primary store, every fallback store,
and configured remotes. A booted-machine test has never exercised its remote
path. The dependency graph also mishandles fallback-only refs: its first
availability check ignores fallbacks, and its later manifest-hash check reads
only the primary store.

## Progress

- [x] (2026-08-20 12:54Z) Decide that `nex install` builds every missing node
      in the requested package's build-dependency closure.
- [x] (2026-08-20 12:54Z) Confirm that the current kernel already builds FUSE,
      CUSE, and virtiofs as modules in `outputs/fs-fuse`; no kernel change is
      needed.
- [x] (2026-08-20 13:28Z) Add the existing kernel `fs-fuse` output to the
      machine fixture and prove a booted guest loads `virtiofs` plus `fuse`.
- [x] (2026-08-20 13:28Z) Export the host store with virtiofsd `--readonly`
      only for tests that request it. Guest write probes fail, and cleanup
      removes every daemon and socket.
- [x] (2026-08-20 13:28Z) Give each machine test explicit tig, gzip, deploy,
      and remote switches. The original five keep all EP017 seeds and receive
      no remote device; the two new tests start without optional refs.
- [x] (2026-08-20 13:28Z) Add `remote-install`: tig and its runtime closure
      start absent, six refs pull from the read-only host store, the primary
      target matches the remote commit, and `tig --version` runs.
- [x] (2026-08-20 13:28Z) Add `build-missing-closure`: one install builds a
      guest-only leaf before its root, keeps both refs out of the remote, and
      runs the root command to print the leaf marker.
- [x] (2026-08-20 13:08Z) Change `nex install` to send an uncached package
      through the dependency graph, with a focused test that finds a missing
      leaf and root through that install helper.
- [x] (2026-08-20 13:08Z) Make dependency-graph ref and manifest-hash checks
      inspect the primary store plus every fallback store. The fresh and stale
      fallback tests pass, as does the full 208-test CLI suite.
- [x] (2026-08-20 13:15Z) Rebuild `pkg/core/nex/nex.yaml` through its strict
      two-build path. Both builds produced checksum `a0d7a7b1`, and the
      packaged `usr/bin/nex --version` printed `nex 1.0`.
- [x] (2026-08-20 13:36Z) Run focused and full Rust checks, rebuild the Nex
      package and four systems reproducibly, run each machine test alone, and
      run all seven twice with `failures: 0`.

## Surprises & Discoveries

- Observation: The built Linux 6.18.24 config already has
  `CONFIG_FUSE_FS=m`, `CONFIG_CUSE=m`, and `CONFIG_VIRTIO_FS=m`.
  Evidence: `zub cat-file` on
  `x86_64/pkg/core/kernel/linux/6.18.24/outputs/boot:boot/config-6.18.24`.

- Observation: The existing `outputs/fs-fuse` ref contains `cuse.ko`,
  `fuse.ko`, and `virtiofs.ko`, while the fixture inherits a `vm` bundle that
  does not include `fs-fuse`.
  Evidence: `zub ls-tree -r
  x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-fuse` and
  `pkg/core/kernel/linux.yaml`.

- Observation: `GraphBuilder::ref_is_available` called
  `ensure_branch_exists(repo_path, ref)` before trying a remote, so it skipped
  every configured fallback. `cached_for_hash` then passed the same primary
  path to `build_exists_for_manifest`, which repeated the omission for
  artifact refs and commit metadata.
  Evidence: the old paths in
  `src/cli/src/build/orchestration/graph.rs` and the passing tests
  `fresh_fallback_dependency_is_not_scheduled` and
  `stale_fallback_dependency_is_scheduled`.

- Observation: `/nex/users/root/manifests` does not exist on the first SSH
  command because Nex creates that Git worktree lazily. A read-only test can
  start in `/nex/manifests`; a test that must edit manifests first creates the
  detached user worktree explicitly.
  Evidence: the first `remote-install` run stopped at `cd`, then both corrected
  guest scripts passed alone and in both full suites.

- Observation: The host's rootless zub setup returns `EPERM` while a direct
  system checkout applies `/boot` ownership. Running the checkout under
  `fakeroot` completes the tree, after which `unshare --root` runs the packaged
  Nex binary normally.
  Evidence: all four checked-out systems printed `nex 1.0` after the fakeroot
  checkout; the first direct checkout stopped at `.nex/tmp/ep018-nex-systemd-root/boot`.

## Decision Log

- Decision: `nex install` calls the dependency-graph builder when its target is
  missing or stale.
  Rationale: One install request must obtain every required package. Refusing
  and asking the person to run a separate build command would contradict this
  plan's machine workflow and leave the known product gap intact.
  Date/Author: 2026-08-20 / Human and Codex

- Decision: Keep FUSE and virtiofs modular and add
  `x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-fuse` only to
  `tests/nex-test-fixture.yaml`.
  Rationale: The guest mounts the test remote after SSH is ready. It does not
  need virtiofs to mount its root or finish early boot. The kernel already
  produces the required modules, so changing built-in policy or rebuilding
  every kernel consumer would add risk without adding capability.
  Date/Author: 2026-08-20 / Human and Codex

- Decision: Export the host's zub store with virtiofsd `--readonly` and also
  mount it read-only in the guest.
  Rationale: The host store is about 26 GB, so copying it per run is too costly.
  The server-side flag prevents guest root from remounting the export writable.
  `pull_local` reads the source store and writes only to the guest's primary
  store.
  Date/Author: 2026-08-20 / Claude and Codex

- Decision: Enable the virtiofs device only for tests that explicitly request
  it.
  Rationale: The five ExecPlan 017 tests must keep testing their declared seed
  data. A remote on those guests could silently fill a missing seed and hide a
  harness regression.
  Date/Author: 2026-08-20 / Codex

- Decision: Do not add a 9p path or an SSH transport path in this plan.
  Rationale: Virtiofsd and QEMU are present on the test host. A second transport
  would add kernel modules, branches, and failure cases without proving another
  product behavior.
  Date/Author: 2026-08-20 / Codex

- Decision: Put layered artifact and manifest-history queries on `Store`, and
  pass that same open `Store` through the dependency graph.
  Rationale: `Store` already owns primary, fallback, and remote lookup order.
  Path-only helpers cannot inspect that full chain and recreated the exact
  fallback bug this plan targets.
  Date/Author: 2026-08-20 / Codex

- Decision: Build one backing image per seed tuple and reuse it for tests with
  the same tuple during a suite.
  Rationale: The original tests keep their exact seed set, both new tests share
  the empty optional set, and a seven-test suite avoids rebuilding the same
  multi-gigabyte disk image seven times.
  Date/Author: 2026-08-20 / Codex

- Decision: Give the two synthetic packages a committed host-mode build
  environment inside the guest's disposable manifests worktree.
  Rationale: The test needs to prove graph order, not bootstrap a compiler or
  shell. The leaf installs one shell command, and the root must execute that
  command from its materialized dependency before it can build.
  Date/Author: 2026-08-20 / Codex

## Outcomes & Retrospective

One `nex install` now gets a package through either path the plan required. If
a configured store holds the package, Nex pulls it and its runtime closure. If
no store holds it, Nex walks the build graph and builds each missing dependency
before the requested package. The graph now treats fallback stores consistently
when it checks both artifact refs and manifest freshness.

The machine harness proves both paths without weakening the five earlier
tests. Only the two new tests receive the read-only virtiofs remote. The remote
test pulled tig plus five runtime refs, matched the target commit in both
stores, and ran tig. The local-build test kept both synthetic refs out of every
source store, built its leaf before its root, and ran the root command with the
leaf's marker. Each of seven tests passed alone, and the full suite passed twice
with no changed host-store counts, daemon, or socket left behind.

Three commits carry the finished work: `32d57f98` changes the CLI and its Rust
tests, `28319670` rebuilds the Nex package, and `00a349ef` adds the fixture,
machine tests, harness support, and rebuilt system checksums.

The only skipped repository-wide check is `cargo fmt --check`: it reports
pre-existing formatting drift in unrelated committed Rust files. Rustfmt
accepted every Rust file changed by this plan. Direct rootless system checkout
also cannot apply `/boot` metadata on this host, so the artifact exercise used
`fakeroot`; all four resulting roots ran their packaged Nex binary.

## Context and Orientation

Read these files before editing:

- `.agents/execplans/done/017-machine-operation-tests.md`
- `.agents/knowledge/machine-self-hosting.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/build-invalidation.md`
- `.agents/knowledge/cli-testing.md`
- `RUST_CODE_STYLE.md`
- `.agents/MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`

The plan changes these existing files:

- `src/cli/src/commands/install.rs`, where an uncached install calls the
  single-package builder.
- `src/cli/src/build/orchestration/graph.rs` and, if needed,
  `src/cli/src/build/orchestration/manifest_lookup.rs`, where graph cache checks
  must use the primary store plus every fallback.
- Matching Rust test modules under `src/cli/src/`.
- `pkg/core/nex/nex.yaml`, whose `dev: src/cli` source means the package must be
  rebuilt before a guest can run the changed CLI.
- `scripts/test-machine-operations.sh`, which seeds stores, starts QEMU, and
  owns guest-process cleanup.
- `base/nex-systemd.yaml`, `tests/nex-test-fixture.yaml`,
  `examples/desktop-vwl/desktop-vwl.yaml`, and `examples/desktop-dev.yaml`,
  which ship the rebuilt Nex package. The fixture must also carry the existing
  `fs-fuse` output.
- `.agents/cleanup-workdirs.sh`, which must know the EP018 root-check paths.

The plan adds two guest scripts under `scripts/machine-tests/`:

- `remote-install.sh` installs tig from the mounted remote.
- `build-missing-closure.sh` writes two temporary package manifests, installs
  the root package, and proves that Nex built the leaf first.

The host store remains the remote source. The guest mounts it at
`/run/nex-host-store` with the virtiofs tag `nex-host-store`. The guest's own
store remains the destination. The two paths must never refer to the same
filesystem.

One known unrelated defect remains outside this plan unless it blocks a named
check: `scripts/test-live-upgrade-hardlinks.sh` can fail with
`uid 0 not mapped in namespace`. Confirm that known signature before treating
it as a regression.

## Plan of Work

**Phase A: make the fixture load the existing modules.** Add the kernel's
`outputs/fs-fuse` ref to `tests/nex-test-fixture.yaml`. In a booted guest, run
`modprobe virtiofs` and verify that `virtiofs`, `fuse`, and their declared
dependencies load. Do not edit `pkg/core/kernel/linux.yaml` or its fragments.

**Phase B: attach an opt-in read-only remote.** For a remote-enabled test,
start `/usr/lib/virtiofsd` before QEMU with the canonical host store path,
an artifact-local socket, and `--readonly`. Wait for the socket. Add the QEMU
memfd, NUMA, socket chardev, and `vhost-user-fs-pci` device. After SSH becomes
ready, load `virtiofs`, mount the tag read-only at `/run/nex-host-store`, and
append one `[[remotes]]` entry to the guest store config. Repeat the mount
after each reboot. Stop virtiofsd and remove its socket after QEMU exits or a
test fails.

The QEMU side needs all four pieces, with unique IDs when the harness already
uses one:

    -object memory-backend-memfd,id=mem,size=${MEMORY}M,share=on
    -numa node,memdev=mem
    -chardev socket,id=char-vfs,path=<socket>
    -device vhost-user-fs-pci,chardev=char-vfs,tag=nex-host-store

The daemon must include:

    /usr/lib/virtiofsd \
      --shared-dir <canonical-host-store> \
      --socket-path <socket> \
      --readonly

**Phase C: give each test an explicit starting state.** Keep the current seed
set as the default for the five existing tests. Let a test disable tig's
runtime seed, gzip's build seed, or all optional package seeds. Let a test
request the remote separately. Log the chosen seed and remote policy before
building its overlay.

**Phase D: prove the pull path.** `remote-install.sh` starts with tig's target
and runtime closure absent from the guest's primary store. The host remote must
resolve the exact target before QEMU starts; fail setup if it does not. The
guest stages, installs tig system-wide, and runs `tig --version`. Capture zub's
`Pulled <ref> from '<name>': <bytes> bytes, <objects> objects` output. After the
install, require the target ref in the guest's primary store and require its
commit to match the remote. The pull message plus the new primary ref and the
working binary distinguish a pull from a no-op.

**Phase E: prove the local graph build.** `build-missing-closure.sh` creates two
temporary manifests below the guest's manifests worktree in a test-only
namespace. Commit the leaf manifest in the guest repository, capture that Git
commit, and put it in the root dependency's `manifest_ref`. The root package's
build script runs the leaf command to create its installed marker. Before the
install, require both exact refs to be absent from the guest store and the
mounted host store. Run one `nex install` for the root package. Require the
guest build plan to name leaf before root, require both refs to appear only in
the guest store, and run the installed root command. The command must print a
marker produced by the leaf during the root build.

**Phase F: repair fallback cache checks.** Make both the ref lookup and the
manifest-hash lookup inspect the complete `Store`, not only `repo_path`. Add a
focused test with an empty primary store and a fallback containing the exact
fresh dependency. The graph must schedule zero builds. Add a stale fallback
case that schedules the dependency, so the test cannot pass by treating every
fallback ref as fresh. Name the tests
`fresh_fallback_dependency_is_not_scheduled` and
`stale_fallback_dependency_is_scheduled`.

**Phase G: rebuild and run all checks.** Rebuild `pkg/core/nex/nex.yaml` first,
then rebuild `base/nex-systemd.yaml`, `tests/nex-test-fixture.yaml`,
`examples/desktop-vwl/desktop-vwl.yaml`, and `examples/desktop-dev.yaml` in
dependency order. Check out each finished system and run its packaged
`/usr/bin/nex --version` inside that root. This plan does not change the kernel
package, flat systems, the installer, or out-of-tree kernel modules. Run both
new machine tests alone, the original five alone, and then all seven twice.

## Concrete Steps

Run commands from the repository root. Put long output under
`.nex/tmp/ep018/` and inspect only the pass markers or relevant errors.

1. Record the clean pre-task state:

       git status --short --untracked-files=all

2. Reconfirm the module state before changing the fixture:

       zub --repo .nex/repo cat-file \
         'x86_64/pkg/core/kernel/linux/6.18.24/outputs/boot:boot/config-6.18.24' \
         | grep -E '^CONFIG_(FUSE_FS|CUSE|VIRTIO_FS)=m$'
       zub --repo .nex/repo ls-tree -r \
         x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-fuse

3. Add `kernel-fs-fuse` with commit
   `x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-fuse` to
   `tests/nex-test-fixture.yaml`, then check it:

       ./src/cli/target/debug/nex check tests/nex-test-fixture.yaml

4. Add the opt-in seed and virtiofsd lifecycle to
   `scripts/test-machine-operations.sh`. Use virtiofsd `--readonly`, wait for
   its socket, pass the chardev and filesystem device to QEMU, mount under
   `/run`, and make cleanup safe when startup stops halfway.

5. Add `scripts/machine-tests/remote-install.sh`. Run it alone and record the
   guest's before refs, pull statistics, after refs, remote commit, primary
   commit, and `tig --version` output:

       scripts/test-machine-operations.sh --test remote-install

6. Change `build_package_to_user_repo` so it calls
   `build_with_dependencies(manifest_path, &opts.manifest_dirs, &opts)`. Keep
   the same primary store, detected manifest roots, and complete fallback list.
   Add `install_builds_missing_dependency_closure` for this call path.

7. Make graph freshness checks inspect fallback stores and add
   `fresh_fallback_dependency_is_not_scheduled` plus
   `stale_fallback_dependency_is_scheduled`.

8. Add `scripts/machine-tests/build-missing-closure.sh` and run it alone:

       scripts/test-machine-operations.sh --test build-missing-closure

9. Run Rust formatting and tests:

       cargo fmt --manifest-path src/cli/Cargo.toml -- --check
       cargo test --manifest-path src/cli/Cargo.toml \
         install_builds_missing_dependency_closure
       cargo test --manifest-path src/cli/Cargo.toml \
         fresh_fallback_dependency_is_not_scheduled
       cargo test --manifest-path src/cli/Cargo.toml \
         stale_fallback_dependency_is_scheduled
       cargo test --manifest-path src/cli/Cargo.toml

10. Rebuild the Nex package through its strict package path, then rebuild the
    four systems that ship it. Do not regenerate
    `tests/nex-test-fixture.bundle`: this plan does not change its pinned Git
    source. Write the full output from each build under `.nex/tmp/ep018/`:

       mkdir -p .nex/tmp/ep018 .nex/tmp/mktemp
       for manifest in \
         pkg/core/nex/nex.yaml \
         base/nex-systemd.yaml \
         tests/nex-test-fixture.yaml \
         examples/desktop-vwl/desktop-vwl.yaml \
         examples/desktop-dev.yaml
       do
         ./src/cli/target/debug/nex check "$manifest"
       done
       TMPDIR="$PWD/.nex/tmp/mktemp" ./nex build pkg/core/nex/nex.yaml \
         --verbose --single --check --update-checksum --force \
         --compute-deps --record-profile --generate-outputs \
         > .nex/tmp/ep018/nex.log 2>&1
       TMPDIR="$PWD/.nex/tmp/mktemp" ./nex build base/nex-systemd.yaml \
         --verbose --single --check --update-checksum \
         > .nex/tmp/ep018/nex-systemd.log 2>&1
       TMPDIR="$PWD/.nex/tmp/mktemp" ./nex build tests/nex-test-fixture.yaml \
         --verbose --single --check --update-checksum \
         > .nex/tmp/ep018/nex-test-fixture.log 2>&1
       TMPDIR="$PWD/.nex/tmp/mktemp" ./nex build \
         examples/desktop-vwl/desktop-vwl.yaml \
         --verbose --single --check --update-checksum \
         > .nex/tmp/ep018/desktop-vwl.log 2>&1
       TMPDIR="$PWD/.nex/tmp/mktemp" ./nex build examples/desktop-dev.yaml \
         --verbose --single --check --update-checksum \
         > .nex/tmp/ep018/desktop-dev.log 2>&1

11. Add `.nex/tmp/ep018-*-root` to `.agents/cleanup-workdirs.sh`, run that
    script, check out all four rebuilt systems, and execute the packaged Nex
    binary inside each root:

       .agents/cleanup-workdirs.sh
       for slug in nex-systemd nex-test-fixture desktop-vwl desktop-dev; do
         root=".nex/tmp/ep018-${slug}-root"
         zub --repo .nex/repo checkout --copy \
           "systems/${slug}/0.0.1" "$root"
         unshare --user --map-root-user --root "$root" \
           /usr/bin/nex --version
       done

12. Run the five original tests individually, then both new tests individually.
    Run the complete seven-test suite twice and record both summaries:

       scripts/test-machine-operations.sh --test build-package
       scripts/test-machine-operations.sh --test temporary-install
       scripts/test-machine-operations.sh --test persistent-install
       scripts/test-machine-operations.sh --test deploy-and-rollback
       scripts/test-machine-operations.sh --test store-upgrade
       scripts/test-machine-operations.sh --test remote-install
       scripts/test-machine-operations.sh --test build-missing-closure
       scripts/test-machine-operations.sh
       scripts/test-machine-operations.sh

13. Before each commit, inspect the exact staged paths and record their matching
    checks in this plan. Stage exact files; never use `git add -A`.

## Validation and Acceptance

The plan is done when:

- A booted guest starts without FUSE or virtiofs built into its kernel, loads
  the existing modules, and mounts the host store through virtiofs.
- Virtiofsd enforces read-only access with `--readonly`; a guest write probe
  fails, and the host store's selected ref and object counts do not change.
- The remote-install guest starts without tig or its runtime closure in its
  primary store, reports a nonzero remote pull, installs tig, and runs
  `tig --version`.
- The build-missing-closure guest proves both synthetic refs are absent from
  its primary and remote stores, builds leaf before root from one install
  request, and runs the root command with the leaf-produced marker.
- `nex install` has a focused regression test for building a missing closure.
- A fresh dependency in a fallback store schedules no build, while a stale one
  does.
- `cargo fmt`, focused Rust tests, and the complete CLI test suite pass.
- The Nex package and all four systems pass `nex check` and their strict
  reproducibility builds. Each checked-out system runs its packaged
  `/usr/bin/nex --version` successfully.
- All seven machine tests pass alone, then all seven pass twice in a row.
- Remote-disabled tests start no virtiofsd process and receive no virtiofs QEMU
  device. The new remote support therefore adds no setup work to the original
  five tests.
- No virtiofsd process or socket remains after success, failure, timeout, or
  reboot.

### Completion Check

Completed 2026-08-20. The implementation commits are `32d57f98`, `28319670`,
and `00a349ef`. `git diff --check` and the staged diff check passed before
every commit.

- `cargo test --manifest-path src/cli/Cargo.toml
  install_builds_missing_dependency_closure`,
  `fresh_fallback_dependency_is_not_scheduled`, and
  `stale_fallback_dependency_is_scheduled` each exited 0 with one passing test.
  The complete CLI command exited 0 with 208 passing tests.
- Rustfmt with edition 2021 and `skip_children=true` accepted every changed
  Rust file. The repository-wide `cargo fmt --check` was skipped as a gate
  because unrelated committed files, including `commands/deploy.rs` and
  `manifest/format.rs`, already fail it.
- The strict Nex package build exited 0 twice with checksum
  `a0d7a7b18bfbd1c470f99822d290591331a8cace0ff63db20b504eb84b6be1ce`.
  Its packaged binary printed `nex 1.0`.
- `nex check` exited 0 for `pkg/core/nex/nex.yaml`,
  `base/nex-systemd.yaml`, `tests/nex-test-fixture.yaml`,
  `examples/desktop-vwl/desktop-vwl.yaml`, and
  `examples/desktop-dev.yaml`. The four strict system builds each produced the
  same checksum twice: `623234606cd201ab14b9faf047f9b0f13cf5adce013942ed0cbd60630eaeb251`,
  `142c22a6194d17cc971ae2b20484ecc6f8b530b2dbaa677b6819bd9b42496293`,
  `0e01885b1f037b6f49562f18d2fb558729d3f574c9f4b9e89029dda3f4d4acf9`,
  and `2b566872a6561af0d38336f5ce9f4b1da81bc84850e2e43bb99d6bb339653450`.
  Fakeroot checkouts of all four system refs completed, and each root printed
  `nex 1.0` through `unshare --user --map-root-user --root`.
- Each of the seven named machine-test commands exited 0 alone. Two consecutive
  complete suite commands each reported seven tests and `failures: 0`.
- Both remote-enabled guests loaded `fuse` and `virtiofs`, mounted the host
  store read-only, and failed their write probes. Host counts stayed at 439411
  objects and 16859 refs before and after each test.
- `remote-install` pulled 1,374,989 bytes and 27 objects for tig, pulled five
  runtime refs, matched target commit `efc64fd7` in the primary and remote
  stores, and printed `tig version 2.6.0`.
- `build-missing-closure` reported `leaf-before-root`, published both refs only
  in the guest's primary store, and printed `EP018_LEAF_MARKER` through the
  installed root command.
- The five remote-disabled QEMU logs contain no `vhost-user-fs-pci` argument
  and their artifact directories contain no virtiofsd log. After both full
  suites, `pgrep -a virtiofsd` found no process and no test socket remained.
- `bash -n scripts/test-machine-operations.sh` and `sh -n` on both new guest
  scripts exited 0.

The latest captured guest output for human review is
`.nex/tmp/machine-tests/remote-install/guest-stdout.log` and
`.nex/tmp/machine-tests/build-missing-closure/guest-stdout.log`. The complete
second suite log is `.nex/tmp/ep018/full-suite-2.log`.

## Idempotence and Recovery

Each test discards its qcow2 overlay and creates a new one from the fixture.
The two synthetic manifests live only in that overlay. Repeated runs therefore
start with both synthetic refs absent.

Virtiofsd uses a per-test socket below that test's artifact directory. Cleanup
must tolerate a daemon that never started, a socket that never appeared, QEMU
that failed before assigning `QEMU_PID`, and a guest reboot. Kill and wait for
QEMU before killing virtiofsd. Refuse to reuse a stale socket silently.

The host export stays read-only at the server. If a test reports that a host
ref or object count changed, stop immediately and preserve its logs. Do not run
another remote test until the write path is understood.

Use `TMPDIR="$PWD/.nex/tmp/mktemp"` for builds because `/tmp` is tmpfs on this
host. Use `.agents/cleanup-workdirs.sh` for disposable work directories that
need removal.

## Artifacts and Notes

Keep each test's serial log, QEMU log, virtiofsd log, guest stdout, and failure
journal under `.nex/tmp/machine-tests/<test>/`. Record the exact refs, commits,
bytes, objects, and elapsed time for each pull. Record which existing tests ran
without the remote device.

Do not add command logs or `tests/nex-test-fixture.bundle` to Git.

Rust checks at 2026-08-20 13:08Z:

- `git status --short --untracked-files=all` before work: clean; there were no
  pre-task commit candidates or local-only files to classify.
- `cargo test --manifest-path src/cli/Cargo.toml
  install_builds_missing_dependency_closure`: 1 passed.
- `cargo test --manifest-path src/cli/Cargo.toml
  fresh_fallback_dependency_is_not_scheduled`: 1 passed.
- `cargo test --manifest-path src/cli/Cargo.toml
  stale_fallback_dependency_is_scheduled`: 1 passed.
- `cargo test --manifest-path src/cli/Cargo.toml`: 208 passed.
- `rustfmt --edition 2021 --config skip_children=true` on every changed Rust
  file: passed. The whole-tree `cargo fmt --check` still reports pre-existing
  drift in unrelated committed files, including `commands/deploy.rs` and
  `manifest/format.rs`.

Package check at 2026-08-20 13:15Z:

- `TMPDIR="$PWD/.nex/tmp/mktemp" ./nex build pkg/core/nex/nex.yaml
  --verbose --single --check --update-checksum --force --compute-deps
  --record-profile --generate-outputs`: passed; both builds produced
  `a0d7a7b18bfbd1c470f99822d290591331a8cace0ff63db20b504eb84b6be1ce`.
  Full log: `.nex/tmp/ep018/nex.log`.
- `./src/cli/target/debug/nex check pkg/core/nex/nex.yaml`: passed.
- The package checkout reported `EPERM` while applying directory metadata on
  this rootless host, but it left the complete binary readable;
  `.nex/tmp/ep018-nex-package-root/usr/bin/nex --version` exited 0 and printed
  `nex 1.0`.

System and machine checks at 2026-08-20 13:36Z:

- `nex check` passed for `base/nex-systemd.yaml`,
  `tests/nex-test-fixture.yaml`, `examples/desktop-vwl/desktop-vwl.yaml`, and
  `examples/desktop-dev.yaml`.
- Strict two-build checks produced matching first and second checksums:
  `nex-systemd` `62323460`, `nex-test-fixture` `142c22a6`, `desktop-vwl`
  `0e01885b`, and `desktop-dev` `2b566872`. Full logs are the four matching
  `.nex/tmp/ep018/*.log` files.
- Fakeroot checkouts of all four system refs completed. Running each packaged
  `/usr/bin/nex --version` through `unshare --user --map-root-user --root`
  printed `nex 1.0`.
- Each of the seven `scripts/test-machine-operations.sh --test NAME` commands
  passed alone. The five original test logs also contain no
  `vhost-user-fs-pci` QEMU argument and no virtiofsd log.
- `scripts/test-machine-operations.sh` passed twice consecutively with seven
  tests and zero failures. Logs: `.nex/tmp/ep018/full-suite-1.log` and
  `.nex/tmp/ep018/full-suite-2.log`.
- In both full suites each remote test saw host-store counts of 439411 objects
  and 16859 refs before and after. No `virtiofsd` process or socket remained.
- The remote install pulled tig's target with 1,374,989 bytes and 27 objects,
  then pulled five runtime refs, matched target commit `efc64fd7` in primary
  and remote stores, and ran `tig version 2.6.0`.
- The guest-only build reported `leaf-before-root`, published both exact refs
  only in `/nex/store`, and ran `ep018-root`, which printed
  `EP018_LEAF_MARKER`.

## Interfaces and Dependencies

Changed CLI behavior: `nex install <manifest> <target>` builds missing
build-dependency nodes after every configured store fails to provide them.

Changed test interface: `scripts/test-machine-operations.sh` accepts per-test
seed and remote policy. It exposes the host store to selected guests with tag
`nex-host-store` and mounts it at `/run/nex-host-store`.

Changed fixture input: `tests/nex-test-fixture.yaml` adds the existing
`x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-fuse` ref.

Rebuilt package and systems: `pkg/core/nex/nex.yaml`, `base/nex-systemd.yaml`,
`tests/nex-test-fixture.yaml`, `examples/desktop-vwl/desktop-vwl.yaml`, and
`examples/desktop-dev.yaml`. This plan adds no kernel symbol, changes no kernel
output, and does not rebuild flat systems, the installer, or out-of-tree kernel
modules.

Host dependencies: QEMU with `vhost-user-fs-pci`, `/usr/lib/virtiofsd` with
`--readonly`, `ssh`, `ssh-keygen`, the built fixture, the Nex CLI, and the zub
CLI. The harness must report a missing dependency before changing an image.

ExecPlan 019 remains paused and does not depend on this plan.
