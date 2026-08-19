# Machine Self-Hosting

## A machine needs the full manifest history

Package manifests name their build environment by Git blob SHA, and those SHAs
are usually historical revisions rather than the current file. 476 manifests
name `27b6e5dc`, an older `env/standard.yaml`. See
`.agents/knowledge/environment-pinning.md` for why pinning works that way.

A machine therefore needs the full history of the manifests repository, not a
snapshot of its current files. `nex-init-manifests` in `base/nex-systemd.yaml`
creates `/nex/manifests` with `git init` plus one commit, which cannot satisfy
that and is wrong.

Shipping a Git bundle works. Measured on 2026-08-19 in
`tests/nex-test-fixture.yaml`: a full-history bundle of this repository is
7.8 MB, first boot clones it, and the guest reported
`manifests-commit-count=784` matching the host with
`manifests-pinned-env-blob-reachable=true`.

Two mechanical traps when generating one:

- `git bundle create <file> <commit-sha>` refuses with "empty bundle". It needs
  a named ref. Bundling a branch name produces a bundle whose clone leaves an
  empty tree, failing with `remote HEAD refers to nonexistent ref`. The working
  recipe is `git worktree add --detach <dir> <commit>`, then
  `git -c pack.threads=1 bundle create <out> HEAD` from inside it.
- `pack.threads=1` is required for byte-reproducibility. Three default runs
  produce three different sha256 values; three runs with that setting produce
  one, at the same size. Delta compression is deterministic, parallel
  work-splitting is not.

## Environments are looked up in the store, not in the manifests repository

This is a bug, confirmed on 2026-08-19, and it stops any correctly provisioned
machine from building.

`src/cli/src/build/package.rs:64` calls
`load_environment(&opts.repo_path, &manifest.build.environment)`, and
`src/cli/src/system/build.rs:98` does the same. `opts.repo_path` is the zub
store: `/nex/repo` (`repo.rs:161`) or `/nex/users/<user>/repo`
(`repo.rs:189`). `load_environment_blob` (`build/env.rs:49`) then runs
`git -C <that path> cat-file blob <sha>`.

On a machine the store is a sibling of `/nex/manifests`, so Git's upward search
for a `.git` finds nothing and the build fails:

    Error: Failed to load environment blob 27b6e5dc...: fatal: not a git
    repository (or any parent up to mount point /nex)

No amount of history in `/nex/manifests` fixes this, because the lookup never
looks there.

It appears to work in a development checkout only by accident: `.nex/repo` sits
inside the Nex source repository, so the upward search finds that checkout's
`.git` and every historical blob with it.

The fix is to resolve environment blobs against the repository that owns the
manifest rather than against the store. `ManifestRepositories` already knows
that root.

## Staging needs a kernel feature the test fixture's kernel bundle lacks

A machine also cannot `nex stage` an install, but for an unrelated,
environment-specific reason (EP017, 2026-08-19): `nex stage`
(`src/cli/src/commands/stage.rs`) unconditionally mounts OverlayFS on
`/usr/bin`, `/nex/pkg`, `/nex/env`, and the `bundles/vm` Linux kernel bundle
`base/nex-systemd.yaml` names carries no `overlay.ko`. Full details and the
exact bundle membership are in
`.agents/knowledge/kernel-and-boot.md` ("Kernel bundle module dependencies").
`nex install --system` needs staging first, so this blocks it too.
`nex deploy`/`nex rollback` need neither staging nor OverlayFS and are
unaffected — proven on a `vm`-kernel guest across two real reboots.

Update, 2026-08-19: the human decided the module belongs in
`base/nex-systemd.yaml` itself ("nobody should be running a plain nex-systemd
machine that cannot install a package"), not only in the test fixture, and
moved it there. `nex stage` succeeds on both. New `nex-systemd` checksum
`a271d6d1246076e032c99b3a8d2c060baff9004e428c6ae1f1fb9ddb258d31de`, strict
two-build reproducible. `tests/nex-test-fixture.yaml` no longer carries its
own copy and inherits it; its checksum is unchanged, since
`merge_packages`'s name-based dedup produces the identical merged package
list either way.

## Installing an already-built package still needs its dependencies' own store refs

`nex stage` succeeding surfaced a further, distinct gap (EP017, 2026-08-19):
`nex install <manifest> <target> --system`, even for a package already built
and present in the store as its own ref, fails unless every one of that
package's *runtime dependencies* also has its own `/files` commit resolvable
in the same store. Reproduced exactly, twice in a row, installing
`x86_64/pkg/dev/vcs/tig/2.6.0/outputs/bin` (that one ref alone pulled into
`/nex/repo`, nothing else):

    Error: Custom { kind: NotFound, error: "4 unresolved runtime dependency requirement(s):
      pkg/dev/vcs/git (run: nex compute-deps pkg/pkg/dev/vcs/git.yaml)
        needed by: /usr/bin/tig needs git - missing 51b15102f212/files commit
      pkg/libs/system/glibc (run: nex compute-deps pkg/pkg/libs/system/glibc.yaml)
        needed by: /usr/bin/tig needs glibc - missing bf348eabcec2/files commit
      pkg/libs/system/ncurses (run: nex compute-deps pkg/pkg/libs/system/ncurses.yaml)
        needed by: /usr/bin/tig needs ncurses - missing 05d611ad0722/files commit
      pkg/libs/system/readline (run: nex compute-deps pkg/pkg/libs/system/readline.yaml)
        needed by: /usr/bin/tig needs readline - missing 6c7351f248a8/files commit" }

Traced to `src/cli/src/materializer/mod.rs`: `materialize()` calls
`runtime_closure()`, which (when `config.resolve_deps`, the default) calls
`resolve_runtime_deps_precomputed` to read each requested package's
precomputed `needs` metadata (from `nex compute-deps`) and looks up a
`/files` commit for every dependency it names, against `config.repo_path`
plus `config.fallback_repo_paths` — not just the requested package's own
ref. Any unresolved requirement makes `reject_unresolved_dependencies` fail
the whole install before anything is checked out. This is a different
mechanism from the "flatten runtime deps into the package's own capsule"
step seen during *system assembly* builds (`system::build`, "Flattened N
libs into ..." in build logs) — `nex install`'s path does not flatten;
it requires the dependency closure to be independently resolvable in the
store.

Update, 2026-08-19: this was correctly identified as a test-harness gap, not
a product defect. Fixed by seeding the requested package's *full* runtime
closure, resolved fresh on the host each run with `nex resolve <package> -v`
(the same algorithm `nex install` itself uses) and `zub pull`'d one ref at a
time — see `seed_system_repo()` in `scripts/test-machine-operations.sh`.
Confirmed working: `nex install` now reports `Runtime closure: 6 commit(s)`,
matching exactly what gets seeded, and proceeds to a real checkout. It then
hits a new, different error; see "A hardlinked checkout can fail with 'uid 0
not mapped in namespace'" below.

## A hardlinked checkout can fail with "uid 0 not mapped in namespace"

Found 2026-08-19, after the fix above. `nex install`'s checkout and `nex
deploy`'s checkout both now fail, reproduced deterministically twice:

    Error: Custom { kind: Other, error: "uid 0 not mapped in namespace" }

For `nex deploy` this is a regression: `deploy-and-rollback` had passed every
run before this session's rebuild of `pkg/core/nex/nex.yaml`,
`base/nex-systemd.yaml`, and `tests/nex-test-fixture.yaml`. Traced the error
message to `zub::Error::UnmappedUid`
(`/home/wegel/work/perso/zub/src/error.rs:45`), raised by
`inside_to_outside(uid, &ns.uid_map)` returning `None`
(`/home/wegel/work/perso/zub/src/object/blob.rs:37,132`) — a call inside
`write_blob`, a *commit-time* function. It gets reached from what looks like
a plain checkout because a hardlinked checkout can find its target blob
already modified (comment in that file: "A hardlinked checkout may have
modified an older store object... The atomic rename below repairs a
mismatching object") and repairs it by writing a fresh blob, which needs to
map the file's uid/gid.

The guest's system repo config sets `uid_map = []` / `gid_map = []`
(`write_zub_config()` in `scripts/test-machine-operations.sh`, copied
verbatim from the established `scripts/qemu-test-live-upgrade.sh` pattern).
An empty map matches nothing, so any call to `inside_to_outside` fails for
any real uid, including root's — `NsConfig::identity()`
(`zub/src/namespace/mapping.rs:49`) is what an actual unrestricted mapping
looks like, one entry covering the whole range, not an empty list.

Fixed, 2026-08-19, by the human: `write_zub_config()` now writes an explicit
identity range instead of an empty list,

    [[namespace.uid_map]]
    inside_start = 0
    outside_start = 0
    count = 65536

and the same for `gid_map`. The guest runs as real root, so identity is
correct (contrast the *host's* own `.nex/repo/config.toml`, which maps
inside 0 to outside 1000, the rootless mapping for this developer's own
user — a different, also-correct case). Confirmed fixed: `deploy-and-rollback`
passes again, twice in a row.

`scripts/qemu-test-live-upgrade.sh:168` carries the same empty-map pattern
and therefore the same latent bug. It has simply never reached a blob-writing
path (its guest never runs `nex install` or anything else that hits this).
Left alone deliberately, per instruction — recorded here, not fixed there.

## `nex install --system` can't create `/nex/env` on a machine that never had one

Found 2026-08-19, after the uid-mapping fix above got `nex install` and
`nex deploy` both checking out for real. `temporary-install` and
`persistent-install` now get all the way through materialization —

    Checking out dev/vcs/tig/2.6.0 (1 output) -> /nex/pkg/dev/vcs/tig/2.6.0/0cbb5716
    Flattened 6 libs into dev/vcs/tig/2.6.0/0cbb5716
    Materialization complete.
    Error: Os { code: 30, kind: ReadOnlyFilesystem, message: "Read-only file system" }

reproduced deterministically twice — then fail immediately after, before
`nex commit` is ever reached.

Confirmed directly (diagnostic added to the guest script, then reverted):
before `nex install` runs at all, `/nex/pkg` already exists (it's part of
the assembled deployment, real directory, `apps/cli/core/dev/libs`
populated) but is read-only (`touch /nex/pkg/x` -> "Read-only file system"),
and `/nex/env` does not exist at all ("No such file or directory").
`base/nex-systemd.yaml`'s build script creates `/nex/repo`,
`/nex/deployments`, `/nex/users`, `/nex/staging`, `/nex/manifests` — no
`/nex/env`.

`nex install`'s materialize step succeeds because staged writes go through
`physical_root` (the staging overlay's upper dir,
`src/cli/src/commands/install.rs`: "system installs when staged write to
overlay upper dir"), not the real `/nex/pkg`. The failure comes right after,
in `mount_nex_overlays()` (`src/cli/src/commands/stage.rs`): it mounts the
`/nex/pkg` overlay fine (the mountpoint already exists), then does the same
for `/nex/env` — `if !Path::new("/nex/env").exists() {
fs::create_dir_all("/nex/env")?; }` — and creating that directory means
writing into `/nex`, which lives on the read-only deployment root. Nothing
in the assembly ever created `/nex/env`, so this is every first
`nex install --system` on a machine built this way, not specific to `tig` or
to this fixture.

Not fixed, not routed around: whether `/nex/env` belongs in the assembly's
own directory list (alongside the others `base/nex-systemd.yaml` already
creates) or `mount_nex_overlays()` should create it somewhere writable is a
product design question.

Update, 2026-08-19: fixed by the human, adding `/target/nex/env` to
`base/nex-systemd.yaml`'s mkdir line, with a comment. Confirmed: `nex
install --system tig outputs/bin` now completes cleanly end to end --
checkout, flatten, symlink into `/usr/bin`, `Installed dev/vcs/tig 2.6.0 (set
as current)`, `install-exit=0`, and the installed `tig`/`git` binaries run
(`tig version 2.6.0`, `run-exit=0`). This is the first time any journey in
this plan has completed a real `nex install`. Two further, later findings
below, one per install test.

## `nex discard` can fail: `/nex/pkg` unmount reports busy, then removing staging state hits "Read-only file system"

Found 2026-08-19, after `/nex/env` was fixed. `temporary-install`, after a
fully successful `nex stage` / `nex install` / run, calls `nex discard
--force` and fails, reproduced identically twice:

    Discarding staging changes...
    umount: /nex/pkg: target is busy.
    Error: Os { code: 30, kind: ReadOnlyFilesystem, message: "Read-only file system" }
    discard-exit=1

`cleanup_staging()` (`src/cli/src/commands/stage.rs`) unmounts `/usr/bin`,
`/nex/env`, `/nex/pkg` in that order (each via `let _ = Command::new("umount")...`,
so a failed unmount is not itself fatal), then does
`fs::remove_dir_all(STAGING_STATE_DIR)?` (`/nex/staging`), which is the call
that actually fails here. `/usr/bin`'s unmount produced no message (so it
presumably succeeded); only `/nex/pkg`'s did.

Not chased to a root cause -- reporting only what was directly observed, not
a guess at the mechanism this time. What is NOT yet known: what still holds
`/nex/pkg` open at that point (the installing `nex install` process has
already exited by the time `nex discard` runs as a separate SSH command), and
whether the `Read-only file system` error is `/nex/staging` itself or
something reachable through it (its `upper/nex/pkg` subtree is the live
overlay's own upperdir, still mounted at the point of the `remove_dir_all`
call, since the unmount above it failed).

## `nex commit` cannot find a "current deployment" on a fresh machine -- confirmed

This is the gap flagged as a hypothesis back when `nex stage` still failed
outright (this file's staging section, and the ExecPlan's Surprises &
Discoveries). With staging and install both now working, `persistent-install`
reaches `nex commit` for the first time and the hypothesis is confirmed,
reproduced identically twice:

    Committing changes: install tig for persistent-install test
    Error: Custom { kind: NotFound, error: "Could not determine current deployment" }
    commit-exit=1

`get_current_deployment_ref()` (`src/cli/src/commands/commit.rs:156`) looks
for a ref under `nex/deployments/` in the store, then falls back to a literal
`nex/base` ref; neither exists on a machine assembled by this fixture (or,
so far as grepping `src/cli`, `scripts`, and `installer` turned up, on any
machine built by anything in this repository today -- nothing ever creates
either). So `nex commit`'s store-ref path (taken whenever `/nex/repo`
exists, which it does here) cannot proceed past this point on a first
install, on any machine. Not fixed, not routed around: whether the
installer should seed a `nex/base` ref, or `nex commit` should fall further
back to something else, is a product design question.
