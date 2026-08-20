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

Update, 2026-08-19: half-fixed, and the other half found. The human
published deployment refs both going forward (`nex deploy` now writes
`nex/deployments/<checksum>.<serial>` after checkout) and for the fixture's
initial deployment (`seed_system_repo()` in
`scripts/test-machine-operations.sh` now writes
`refs/heads/nex/deployments/<checksum>.0` directly, since zub's CLI has no
ref-writing command -- what a correct installer would do; `scripts/nex-install`
does not do this today, a separate, known, not-yet-fixed gap).
`persistent-install` still fails identically after this landed and the
cascade rebuilt (`pkg/core/nex/nex.yaml`, `base/nex-systemd.yaml`,
`tests/nex-test-fixture.yaml`), reproduced twice.

Likely second cause, evidence-based but not confirmed by a standalone test:
`get_current_deployment_ref()` finds the ref via
`store.refs(Some("nex/deployments/"))` (`commit.rs:159`) ->
`zub::list_refs_matching(repo, "nex/deployments/")`
(`/home/wegel/work/perso/zub/src/refs.rs:114`), which matches ref names
against a `glob::Pattern` built from that exact string. A pattern with no
wildcard character only matches that exact string. `zub`'s own test of this
function, `test_list_refs_matching`
(`/home/wegel/work/perso/zub/src/refs.rs:381`), passes `"x86_64/*"` (and
separately `"*/pkg/foo/*"`) to get prefix matching -- always with a
wildcard. `commit.rs`'s `"nex/deployments/"` has none, so on this reading
the call returns empty regardless of what refs exist under that prefix, and
`get_current_deployment_ref()` can never see a `nex/deployments/*` ref this
way, published or not. Not fixed, not routed around, and flagged explicitly
as unconfirmed by direct testing -- an earlier note in this file misdiagnosed
a different error by reasoning from adjacent evidence instead of a direct
reproduction, and this one deserves the same caution until someone runs it.

Update, 2026-08-19: the glob theory was confirmed directly by the human (a
two-line repro: `"nex/deployments/"` matches nothing, `"nex/deployments/*"`
finds `nex/deployments/abc123.0`) and fixed at `commit.rs:159`, with a test
in `store/store_tests.rs` pinning the behavior. Committed as `d1e79485`.

Rebuilt the cascade (`pkg/core/nex/nex.yaml`, `base/nex-systemd.yaml`,
`tests/nex-test-fixture.yaml`) and reran: `persistent-install` still failed
identically at first. Found a third, independent cause, this time in the
test harness itself, not product code: `seed_system_repo()`
(`scripts/test-machine-operations.sh`) tries to `rev-parse "$FROM_REF"`
directly against the guest's own store to get the hash to publish as
`nex/deployments/<checksum>.0` -- but `$FROM_REF` (the fixture's own system
ref) is never *pulled* into that store, only *checked out* (via `zub
checkout --copy`, from the host's repo, straight into the root filesystem
content) by an earlier, separate step. So the `rev-parse` always failed,
silently (`2>/dev/null || true`), landing in the `else` branch and never
publishing anything -- confirmed directly in the log: `note: could not
resolve systems/nex-test-fixture/0.0.1, initial deployment ref not
published`, present on every run before this fix. Fixed by pulling
`$FROM_REF` into the guest's store first, same as the other seeded refs.
This is harness code, not product code, and squarely a bug in implementing
the stated intent rather than a design question, so it was fixed directly
rather than reported and left.

With all three fixes in place (the ref publish, the glob pattern, and the
harness's missing pull), `nex commit` now works for real, confirmed by its
own output:

    Committing changes: install tig for persistent-install test
      Current deployment: nex/deployments/<fixture-checksum>.0
      Checking out current deployment...
      Applying staged changes...
      Creating new commit...
      Created deployment: nex/deployments/<unix-timestamp>

But the command still exits non-zero and `persistent-install` still fails,
identically to `temporary-install`'s `nex discard` failure: `commit.rs`'s
`create_deployment()` calls `cleanup_staging()` afterward (the exact same
function `nex discard` calls), which hits `/nex/pkg: target is busy` on
`umount`, then `Read-only file system` on the following
`fs::remove_dir_all("/nex/staging")`. So this is one shared bug in
`cleanup_staging()`, reached from two different commands, not two separate
bugs.

## `/nex/pkg` is structurally busy on a running `nex_structure: true` system -- root cause found by direct observation

Diagnosed by adding temporary probes to a copy of the guest journey script
(reverted after each check; no product code or committed test file was
touched). Observations, reported separately from the mechanism below:

- `fuser` and `lsof` are not installed on this fixture (`command -v` found
  neither).
- Before `nex discard`/`nex commit` runs, `/proc/mounts` shows three active
  overlay mounts: `/usr/bin`, `/nex/pkg`, `/nex/env` (all three get mounted
  by `mount_nex_overlays()`, unconditionally, even though nothing was ever
  installed under `/nex/env`).
- A **manual** `umount /nex/pkg`, run directly in the guest shell before
  `nex discard`/`nex commit` is even invoked, fails the same way: `umount:
  /nex/pkg: target is busy` (exit 32). So this is not specific to how `nex`
  invokes `umount`, and not a timing artifact of running right after
  install.
- After the failed cleanup, `/proc/mounts` shows `/usr/bin` and `/nex/env`
  successfully unmounted; only `/nex/pkg` remains.
- `/nex/staging` itself is still writable both before and after the failed
  cleanup (`touch` succeeds both times). `/nex/pkg/<newfile>` fails with "No
  such file or directory", not "Read-only", after the failure.
  `/nex/staging/upper/nex/pkg` (the still-mounted overlay's own upperdir) no
  longer exists at all after the failed cleanup -- `ls` reports "No such
  file or directory" for it too, even though `/proc/mounts` still lists
  `/nex/pkg`'s overlay as mounted with that exact path as its `upperdir=`.
- Scanning `/proc/*/cwd`, `/proc/*/fd/*`, and `/proc/*/maps` for anything
  under `/nex/pkg`, while `/nex/pkg` was in the confirmed-busy state: PID 1
  (`systemd` itself, the init process) has file descriptor 9 open on
  `/nex/pkg/core/init/systemd/257.5/1223026e/usr/lib/systemd/systemd-executor`,
  and roughly a dozen other PIDs (service and helper processes) have
  `/nex/pkg` paths present in their `/proc/<pid>/maps`. Confirmed separately
  that `/usr/lib/systemd/systemd-executor` is a real symlink pointing at
  exactly that path, and that `/usr/lib` in general is full of symlinks
  into `/nex/pkg/<namespace>/<slug>/<version>/<checksum>/...` (e.g.
  `/usr/lib/environment -> /nex/pkg/libs/security/linux-pam/...`).

Proposed mechanism, kept separate from the observations above: on a
`nex_structure: true` system, `/usr/lib`, `/usr/bin`, and similar FHS paths
are symlinked directly into package capsules under `/nex/pkg`, so every
running process that has ever loaded a shared library or executed a helper
binary holds an open file or a memory mapping backed by `/nex/pkg` --
starting with PID 1 itself, which cannot be stopped to release it. Mounting
an overlay on `/nex/pkg` (`mount_nex_overlays()`) succeeds, because mounting
only needs the mountpoint directory, not an idle target. Unmounting it later
does not: the kernel refuses `umount` on a busy mount, `cleanup_staging()`'s
plain `Command::new("umount")` call (no `-l`/`-f`) does not force it and
silently discards the failure, and the following `fs::remove_dir_all`
attempts to recurse into the still-mounted overlay's own live upperdir,
which is where the visible `Read-only file system` error actually comes
from. On this reading, the failure is not about `tig` or timing; staging an
install that touches `/nex/pkg` on a live `nex_structure: true` system may
be unable to fully unstage while any process is running, by construction,
every time.

Update, 2026-08-19: fixed by the human in `stage.rs` (commit `f1a7dcfb`).
`cleanup_staging` now calls a new `unmount_overlay` that skips a target that
is not mounted, tries a plain unmount, falls back to `umount -l` (lazy) when
busy, and only returns an error (naming the mount) if both fail. Rebuilt the
cascade (`pkg/core/nex/nex.yaml`, `base/nex-systemd.yaml`,
`tests/nex-test-fixture.yaml`) and reran: the mount-level problem is gone,
confirmed directly (diagnostics on a disposable script copy, reverted after;
no product code touched) -- after a failed `nex discard`, `/proc/mounts` no
longer shows any of the three overlays at all (all three detached, lazily
where needed), and `/nex/staging` is confirmed completely empty (`upper/`,
`work/`, the `active` marker all gone). The previously-reported "stale view"
risk did not materialize either: `/usr/bin/tig` correctly disappears,
`command -v tig` finds nothing, `tig --version` reports "command not found".

But `nex discard`/`nex commit` still both fail, identically to before, same
error text:

    Error: Os { code: 30, kind: ReadOnlyFilesystem, message: "Read-only file system" }

One layer deeper now. `cleanup_staging`'s `fs::remove_dir_all(STAGING_STATE_DIR)`
(`/nex/staging`) successfully empties the directory (confirmed: `ls -la
/nex/staging` afterward shows only `.` and `..`) but then fails to remove
`/nex/staging` itself. `/proc/mounts` shows `/nex/staging` is its own
mounted filesystem (`/dev/sda2 /nex/staging ext4 rw,relatime`), separate
from the deployment root -- consistent with the "boot mountpoints" comment
in `base/nex-systemd.yaml`'s build script (bind-mounted from `/var/nex` at
boot, like `/nex/repo`, `/nex/users`, `/nex/manifests`, `/nex/env`).
Removing a directory *entry* (`rmdir`) requires write permission on its
*parent*, regardless of whether the target itself is a separate mounted
filesystem; `/nex/staging`'s parent is `/nex`, on the read-only deployment
root. So `rmdir("/nex/staging")` fails with EROFS even though everything
inside it is on a writable filesystem and was just successfully cleared.
This reads as structural, not timing- or content-dependent: `nex
stage`/`nex discard`/`nex commit` can create and empty `/nex/staging`'s
contents freely, but nothing can ever remove the `/nex/staging` mountpoint
directory itself while the deployment root stays read-only -- `cleanup_staging`
would need to stop trying to remove the mountpoint directory and only clear
its contents. Not fixed, not routed around; reported as an observation
(what was seen) with this explanation offered separately, per the same
discipline as before.

Update, 2026-08-19: fixed by the human (`stage.rs`, commit `eacb11f5`).
`cleanup_staging` now calls `clear_staging_state`, which deletes only the
*contents* of `/nex/staging` and leaves the mount point directory itself
alone -- exactly right, since `/nex/staging` is a mount point
`base/nex-systemd.yaml` creates on purpose. Rebuilt the cascade
(`pkg/core/nex/nex.yaml`, `base/nex-systemd.yaml`,
`tests/nex-test-fixture.yaml`) and reran: **`temporary-install` now passes
end to end**, twice in a row -- stage, install, run the binary, discard,
confirm the binary is gone and staging is inactive. The informational
`umount: /nex/pkg: target is busy.` line from the plain-unmount attempt
still prints (expected: `unmount_overlay` tries plain `umount` first,
prints its stderr, then falls back to lazy `umount -l`), but is no longer
fatal.

## `nex commit` records a new deployment in the store, but never activates it -- reboot boots the same deployment as before

Found 2026-08-19, once `nex commit` itself started succeeding
(`commit-exit=0`, "Deployment created successfully."). `persistent-install`
reboots and its after-reboot assertion fails:

    current-deployment=<fixture-checksum>.0
    post-reboot-binary-present=false
    error-line=tig not present after reboot

Per instruction, checked which deployment the machine actually booted before
concluding the install was lost, using the same `/proc/cmdline` `zub=`
convention used throughout this plan: `current-deployment` after reboot is
`<fixture-checksum>.0` -- the exact same deployment the machine was running
*before* `nex commit` ran. Not a stale view, not a race: `create_deployment()`
(`src/cli/src/commands/commit.rs:108`) only calls `store::commit_tree(NEX_REPO,
&new_ref, ...)` (`commit.rs:146`), which writes a new commit into the *store*
at `/nex/repo` under a ref named `nex/deployments/<unix-timestamp>` -- it
never touches `/sysroot`, `/nex/deployments` (the on-disk directory
`nex deploy`/`nex rollback` create serials under), or anything the
bootloader looks at. So a plain reboot after `nex commit`, with no further
command, can never boot the new content: nothing new was ever placed where
boot-time deployment selection looks. The install itself was captured
correctly (the store ref exists, `nex.deployment.parent` correctly names the
prior deployment, `nex.deployment.message` carries the commit message), it
is simply never promoted to something bootable. Not fixed, not routed
around, and no change made to the test itself to route around it (e.g. by
also calling `nex deploy` on the new ref) -- that would test a different
operation than "install a package and keep it across a reboot" as specified.

Update, 2026-08-19: fixed by the human (commit `a084cf66`). After writing
the store ref, `create_deployment` now calls `deploy::run` on it directly
(`allow_commit_hash: true`, since the ref is named by timestamp and carries
no checksum metadata), reusing the same activation path `nex deploy` uses
rather than growing a second one.

Rebuilt the cascade and reran: the new deployment genuinely gets created and
activated on disk --

    Created deployment: nex/deployments/1787181037
      Activating for next boot...
    System ref:   nex/deployments/1787181037
    ...
    Deployed: 04a0b9ccc15ff9065fdf9cc3b0c8e23ffd144dec5db3f1b6444a043b8ad7a8ad.1
    Reboot to activate (bootloader picks highest serial).

confirmed directly: `ls /sysroot/nex/deployments` afterward shows both the
original `<fixture-checksum>.0` and the new `<new-checksum>.1` on disk. But
`nex commit` still exits non-zero right after, with the same `Read-only
file system` text as before, so `persistent-install` still fails and never
reaches reboot -- one layer further in, and this one is `deploy::run`'s own
remount handling, not `cleanup_staging` again.

## `deploy::run`'s remount to read-only affects more than `/sysroot` when it shares a block device

Confirmed by direct before/after comparison of the full (unfiltered)
`/proc/mounts`, captured immediately around the `nex commit` call in a
disposable copy of the guest script (reverted after; no product code
touched). Before `nex commit` runs:

    /dev/sda2 / ext4 ro,relatime
    /dev/sda2 /sysroot ext4 ro,relatime
    /dev/sda2 /nex/deployments ext4 rw,relatime
    /dev/sda2 /nex/staging ext4 rw,relatime

After (right after `deploy::run` prints "Deployed: ..." and "Reboot to
activate...", i.e. after its `RemountGuard` remounts `/sysroot` back to
read-only on the way out):

    /dev/sda2 / ext4 ro,relatime
    /dev/sda2 /sysroot ext4 ro,relatime
    /dev/sda2 /nex/deployments ext4 ro,relatime
    /dev/sda2 /nex/staging ext4 ro,relatime

`/nex/deployments` and `/nex/staging` flip from `rw` to `ro` too, even
though `deploy.rs`'s `RemountGuard::remount_ro` names only `/sysroot`
(`mount -o remount,ro /sysroot`) and never touches those paths directly. All
four mount points are separate entries for the same block device,
`/dev/sda2`; a `remount` on any one of them changes the underlying
filesystem instance's read-only state for all of them at once. `create_deployment`
then immediately does `fs::remove_dir_all(&staging_dir)`
(`commit.rs:151`, removing `/nex/staging/commit_staging`) -- a call the
human had already checked and confirmed correct on its own terms (parent
`/nex/staging` is normally writable) -- and it fails, because by that point
`/nex/staging` has just been made read-only as a side effect of unrelated
cleanup for `/sysroot`.

Directly confirmed the deployment itself survives this: `ls -la
/sysroot/nex/deployments` right after the failure shows both the original
deployment and the newly activated one, un-corrupted, on disk. So the
activation is real; only the trailing, unrelated cleanup step fails, and
because it fails, the guest's own `nex commit` invocation reports non-zero
and the test harness (by design, stopping at the first failed phase) does
not attempt the reboot. Whether the new deployment would actually have
booted was not tested -- forcing a reboot past a phase the harness is
designed to stop at was judged out of scope for this observation, not
something to decide unilaterally.

On accumulation: this fixture always starts from a fresh overlay per test
run, so "serials climbing across the run" could not be observed across
separate invocations -- every run showed `Next serial: 1` for the new
deployment. Within a single run, only one `nex commit` happens, so no
within-run accumulation was observed either. Worth noting anyway: because
the *activation* step completes and commits its result before the failing
cleanup, a "failed" `nex commit` (exit code 1, from the user's point of
view) still leaves a new, real, on-disk deployment behind. Not fixed, not
routed around.

Update, 2026-08-19: the ordering itself was fixed by the human, commit
`9c94ef56`. `commit.rs` now runs `create_deployment` (returns the new ref),
then `cleanup_staging()`, then `activate_deployment(&new_ref)` last, with a
comment explaining why activation must be last. Rebuilt the cascade and
reran: the reordering is real (`cleanup_staging` now visibly runs before
"Activating for next boot..." in the log, where it used to run after), and
the deployment activation still concretely succeeds -- `ls -la
/sysroot/nex/deployments` after the failure again shows both the original
and the new deployment directories, intact, on disk. But `nex commit` still
fails, on a new, different, downstream mount error:

    mount: /sysroot: mount point is busy.
           dmesg(1) may have more information after failed mount system call.
    mount: /sysroot: mount point is busy.
           dmesg(1) may have more information after failed mount system call.
    Error: Custom { kind: Other, error: "failed to remount /sysroot ro" }

Reproduced identically twice. The doubled message is `RemountGuard`'s own
shape: the explicit `remount_ro()` call fails and its error propagates, then
its `Drop` impl (`let _ = self.remount_ro();`) retries and fails again on
the way out of scope, printing a second, ignored failure.

Confirmed directly (same disposable-diagnostic method, reverted after; no
product code touched): `/proc/mounts` right after the failure shows
`/dev/sda2 /sysroot ext4 rw,relatime` -- the remount-to-read-only genuinely
did not happen, and `/sysroot` is left mounted read-write. `touch
/sysroot/.diagtest` succeeds, confirming it. `/nex/pkg` and `/nex/env`'s
overlays are gone from `/proc/mounts` (cleanup's lazy unmount completed),
and `/nex/deployments`/`/nex/staging` show `rw`, unaffected this time
(unlike the previous unit's failure mode, where they flipped to `ro`).

Proposed mechanism, kept separate from the observation above and not acted
on: this reads as the same underlying tension as before -- `/`, `/sysroot`,
`/nex/deployments`, and `/nex/staging` are all mounts of one block device,
and `/nex/pkg`'s lowerdir is a real directory on that same device (part of
`/`/`/sysroot`) -- but manifesting differently because of the new ordering.
`cleanup_staging`'s lazy unmount of `/nex/pkg` happens immediately before
`activate_deployment` now, and a lazy unmount detaches immediately while the
kernel finishes releasing the underlying mount in the background; if that
release is still in flight when `deploy::run`'s own `mount -o remount,ro
/sysroot` runs moments later, a remount of the shared device could
plausibly be refused as busy until it settles. Not confirmed with a
standalone reproduction; flagged with the same caution as the earlier
`glob` and `rmdir`-parent findings, both of which held up, but this one
has not been separately verified yet.

On the deployment-accumulation question raised last time: with the fix,
this stopped being purely theoretical. This run's failure (a busy remount,
not a cleanup failure) happened *after* activation completed, so the same
"stray deployment on a failed commit" pattern recurred here too -- confirmed
by the same `ls -la /sysroot/nex/deployments` check, both the original and
new deployment present after the failure. Still not fixable-by-me territory
and not fixed; noted as asked.

Update, 2026-08-19: the human fixed the remount retry (`49df0574`):
`RemountGuard::remount_ro` now retries five times with a 200ms gap and, if
`/sysroot` is still busy after all five, prints a warning and returns success
instead of an error. Rebuilt the cascade and reran: `nex commit` now exits 0
and `persistent-install` reaches `PASS` for the before-reboot phase for the
first time in this plan. The warning text appears exactly twice per run
(the explicit `remount_ro()` call, then the `Drop` retry) -- the retry does
not actually resolve the busy condition here, it still exhausts all five
attempts both times, but the machine is left writable rather than the
command failing outright. Reboot succeeds, and `current-deployment=` after
reboot is confirmed different from the pre-commit value in both runs, which
is the direct proof that activation genuinely produced a bootable, booted
result -- not just an on-disk deployment directory as before. See the next
section for what happens after that: the new deployment activates
correctly, but is missing the package the test asked `nex commit` to
persist.

## `nex commit` activates the right deployment, but the new tree is missing the just-installed package

Confirmed by source read plus a direct on-disk check (disposable diagnostic
added to a copy of the guest script, reverted after; no product code
touched) -- not a hypothesis.

With the remount-retry fix above landed, `persistent-install` gets all the
way through `nex commit`, activation, and reboot: `current-deployment=`
correctly shows the new deployment
(`746563fd97dc42f12e41a3312c39e72eb5d71cb5f04e0215905df02846a3ab4f.1`, vs.
the starting `...4e3.0`), reproduced identically twice. But
`post-reboot-binary-present=false`: `tig`, the package the test staged,
installed, and committed, is not usable in the new deployment.

`commit.rs`'s `create_deployment` builds the new deployment's tree by
checking out the current deployment into a scratch dir, then layering the
staging overlay's two upperdirs on top:

    let upper_nex = format!("{}/upper/nex", STAGING_STATE_DIR);       // /nex/staging/upper/nex
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);
    if Path::new(&upper_nex).exists() {
        let nex_target = format!("{}/nex/pkg", staging_dir);
        fs::create_dir_all(&nex_target)?;
        copy_dir_contents(&upper_nex, &nex_target)?;
    }
    if Path::new(&upper_usr_bin).exists() {
        let bin_target = format!("{}/usr/bin", staging_dir);
        fs::create_dir_all(&bin_target)?;
        copy_dir_contents(&upper_usr_bin, &bin_target)?;
    }

`copy_dir_contents(src, dst)` shells out to `cp -a "$src/." "$dst"` -- by
construction it copies `src`'s *contents* into `dst`, not `src` itself as a
subdirectory of `dst`. `upper_nex` is `/nex/staging/upper/nex`, which is the
*parent* of the two overlay upperdirs actually used for staging, confirmed
earlier by `/proc/mounts`:

    overlay /nex/pkg overlay rw,...,upperdir=/nex/staging/upper/nex/pkg,...
    overlay /nex/env overlay rw,...,upperdir=/nex/staging/upper/nex/env,...

So `upper_nex`'s direct children are `pkg/` and `env/`. Passing `upper_nex`
itself as `src`, with `nex_target = "{staging_dir}/nex/pkg"` as `dst`, copies
those two children *into* `nex/pkg`: tig's files land at
`nex/pkg/pkg/dev/vcs/tig/2.6.0/0cbb5716/...` (one extra `pkg/` nesting
level), and `env/`'s content lands at `nex/pkg/env/...` instead of the
intended `nex/env/...`. The correct call should either copy
`upper_nex/pkg` into `nex_target` and separately copy `upper_nex/env` into
an `nex/env` target, or copy `upper_nex` directly into `{staging_dir}/nex`
(one level higher) so its `pkg/`/`env/` children land where they belong
without renaming.

Confirmed directly on the booted new deployment (diagnostic added to a copy
of `persistent-install.sh`'s after-reboot phase, reverted after):

    diag-nex-pkg-listing=... apps cli core dev env libs pkg ...
    diag-nex-pkg-pkg-listing=... dev ...
    diag-usrbin-tig-readlink=../../nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/tig
    diag-usrbin-tig-stat=stat: cannot statx '/usr/bin/tig': No such file or directory

`/nex/pkg` -- the real, currently-booted, top-level FHS path -- has an
unexpected `pkg/` entry alongside the legitimate namespace directories
(`apps`, `cli`, `core`, `dev`, `env`, `libs`), and `dev/vcs/tig/2.6.0/0cbb5716`
sits one level too deep, under that stray `pkg/`. The `/usr/bin/tig` symlink
itself is correctly named and correctly placed (its copy, via
`upper_usr_bin`/`bin_target`, is a separate call whose source and target
shapes actually match, so it has no equivalent bug) but is dangling: it
points at `/nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/tig`, where nothing
exists (the real file is one level deeper, under the extra `pkg/`), so
`stat -L` reports "No such file or directory" and `command -v tig` (and thus
the test's presence check) correctly reports it absent. This fully accounts
for the failure; no further hypothesis is needed. Not fixed (product code,
`commit.rs`, not touched).

Update, 2026-08-19: the human fixed it in both places it existed
(`dc561b55`). `create_deployment` now copies `upper/nex`'s contents to
`{staging_dir}/nex` (a level higher than before) instead of
`{staging_dir}/nex/pkg`, matching `upper/nex`'s real children (`pkg/` and
`env/`). The identical bug also existed in `merge_overlay_changes`
(`commit.rs:87`, the non-store path used when clearing a discard-adjacent
overlay), copying to `/nex` instead of `/nex/pkg` there too. That function
also had its two raw `let _ = Command::new("umount")...` calls (whose
failures were silently discarded) replaced with `stage.rs`'s
`unmount_overlay` helper (now `pub(super)`), the same fix `cleanup_staging`
got earlier, plus an `/nex/env` unmount it was missing -- a swallowed
failure there would have copied into a still-live overlay instead of the
real location, the same class of bug as the original `/nex/pkg` busy-unmount
finding.

Rebuilt the cascade and reran the full suite three times (three, since the
first two runs both landed 3 of 4): identical every time.
`persistent-install` passes completely, including the after-reboot run
(`post-reboot-binary-present=true`, `tig --version` executes and exits 0).

Checked the tree shape directly rather than trusting the presence check
alone, with a disposable diagnostic on a copy of the guest script (reverted
after; no product code touched):

    diag-nex-pkg-has-stray-pkg=false
    diag-nex-env-is-sibling=true
    diag-nex-pkg-env-should-not-exist=false
    diag-usrbin-tig-readlink=../../nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/tig
    diag-usrbin-tig-resolves=true

All three of the specifically-requested checks hold: no stray `/nex/pkg/pkg`,
`/nex/env` is a sibling of `/nex/pkg` rather than nested under it, and
`/usr/bin/tig`'s symlink resolves. The fix is structurally correct, not
merely passing the test's presence check by coincidence.

The `/sysroot is still writable` warning still appeared every run, twice per
`nex commit` call, unchanged from the previous unit -- the five-attempt
retry still does not settle this particular busy remount, but per
instruction this is now an accepted, reported, non-blocking outcome rather
than something to keep chasing.

`build-package` remains the one understood, unfixed gap in this plan:
missing pieces of glibc's `/usr/lib/gconv/*` split output
(`libCNS.so`, `libGB.so`, `libISOIR165.so`, `libJIS.so`, and others),
because the harness's `seed_system_repo` only seeds `tig`'s runtime closure,
not the build-time dependency closure gzip's own build needs. A real
limitation of what the harness seeds, not a product bug; not routed around.

Update, 2026-08-19: this was corrected -- seeding gzip's build-time closure
turned out to be the same legitimate host-side setup tig's runtime closure
already gets, not a limitation worth leaving alone. `seed_gzip_build_closure()`
was added to the harness and does seed it correctly and completely (see
below), but `build-package` still fails, for a *different* reason that no
amount of seeding can fix: `nex build`'s own dependency resolution never
looks at the store the harness seeds, on the code path this test exercises.

## `nex build`'s user-context dependency resolution never consults its own auto-detected `/nex/repo` fallback

Confirmed by direct reproduction, not inferred: added a disposable
diagnostic to a copy of `build-package.sh` (reverted after; no product code
touched) that ran `nex build pkg/cli/archive/gzip.yaml --single --repo
/nex/repo` explicitly, forcing the repo path. With the repo forced,
dependency resolution succeeded completely -- found the full 15-commit
closure including glibc's self-referencing `/files` ref, "Materialization
complete", fetched gzip's source over the network, and started the build
sandbox. The exact same command with no `--repo` flag (what the guest
script actually runs, and what a person at the machine would actually type)
fails at dependency resolution with the same "N unresolved runtime
dependency requirement(s)" error the harness has reported for four units
running, even after the store the harness seeds (`/nex/repo`) was proven,
by a separate direct check, to hold the exact ref by that exact name and
checksum (`x86_64/pkg/libs/system/glibc/2.39/bf348eabcec257edace3e1e05458bf79ddad1a5164f25e706b7e50d93b25190d/files`,
confirmed present at
`/nex/repo/refs/heads/x86_64/pkg/libs/system/glibc/2.39/.../files` on the
booted guest).

Read to find out why, not guessed: `nex build`, run as root without
`--system`, goes through `detect_context`'s *user* context branch
(`src/cli/src/repo.rs`). That branch's primary repo is
`/nex/users/<user>/repo` -- a fresh, empty directory on a machine that has
never built anything as that user, which this fixture's machine has not.
`detect_context` does compute a fallback chain for that context and does
add `/nex/repo` to it when the path exists:

    let mut fallback_repos = Vec::new();
    if Path::new("/nex/repo").exists() {
        fallback_repos.push(PathBuf::from("/nex/repo"));
    }
    ...
    Ok(NexContext {
        repo_path: user_base.join("repo"),
        ...
        fallback_repos,
        ...
    })

But `BuildOpts.fallback_repos` (`src/cli/src/commands/build.rs`) is
populated only from the CLI's own `--fallback-repo` flag
(`fallback_repos: args.fallback_repos.clone()`), never from
`NexContext.fallback_repos`. The `NexContext` this command computes and the
`BuildOpts` it actually builds are two separate structures, and the
fallback chain the context works out for itself never crosses into the one
the builder uses. So `materialize_build_dependencies`
(`src/cli/src/build/rootfs.rs`) constructs its `MaterializeConfig` with an
empty `fallback_repo_paths` unless the user passes `--fallback-repo`
explicitly, and the resolver's self-referencing-need check
(`src/cli/src/materializer/resolver.rs`, `queue_self_file_dependency`,
`self.store.resolve_ref(&files_commit)`) only ever looks in the empty
per-user repo -- exactly matching the observed behavior: the 15 direct
dependency roots print as a closure of the right size (root-listing alone
does not check existence), but the very first self-referencing need
(glibc's own gconv/ld-linux/libc chain) fails to resolve, and
`reject_unresolved_dependencies` aborts before checkout, which is where the
other 14 roots' existence would also have been checked and would also have
failed, for the identical reason.

No amount of host-side seeding closes this: the guest's plain `nex build
pkg/cli/archive/gzip.yaml --single` -- no flags, the exact command a
person at the machine would type, matching this test's own stated purpose
("using only what the machine itself carries") -- never consults
`/nex/repo` for build-dependency resolution at all on this path, regardless
of what is in it. Two ways to close it, both a product or test-scope
decision rather than more seeding: thread `NexContext.fallback_repos` into
`BuildOpts.fallback_repos` in product code, so a build's own dependency
resolution sees the same fallback its own context already computed for
other purposes; or decide this test should invoke `nex build` with
`--system` or an explicit `--repo /nex/repo`, which would change what the
test proves (no longer "the plain command works," but "the command works
when pointed at the system store"). Not fixed, not decided unilaterally,
and not routed around by quietly changing the guest script's invocation.

A second, separate, unrelated failure appears once dependency resolution
is forced to succeed (via the explicit `--repo /nex/repo` diagnostic
above): `unshare: unshare failed: Invalid argument`, from the build
script's sandbox setup (`src/cli/src/build/script.rs`,
`unshare --user --pid --mount --uts --fork --ipc --net --map-root-user`).
Observed, not investigated -- a second blocker behind the first, not yet
chased to a cause.

## Package acquisition on a booted machine

ExecPlan 018 proved two paths on 2026-08-20. A configured read-only remote can
supply a requested package and its runtime closure, and one `nex install`
request can build a missing root plus its build-dependency closure locally.
The seven-test machine suite passed twice in succession.

The dependency graph must ask one `Store` about both artifact availability and
manifest freshness. Checking only the primary repository path causes two bad
answers: a fresh fallback-only ref looks absent, and its manifest history also
looks absent. `Store::artifact_exists` and
`Store::find_commit_by_manifest_hash` now search the primary repository and
every fallback. The tests `fresh_fallback_dependency_is_not_scheduled` and
`stale_fallback_dependency_is_scheduled` prove the two sides of this rule.

An uncached `nex install` must call `build_with_dependencies` with the same
manifest roots that the install command already found. Calling `build_single`
assumes every build dependency already resolves and cannot satisfy a package
request on a fresh machine. `install_builds_missing_dependency_closure` covers
this call path, while `scripts/machine-tests/build-missing-closure.sh` proves
the behavior in a booted guest with a synthetic leaf and root.

The machine harness exposes the host store only to tests that ask for a remote.
It starts `/usr/lib/virtiofsd` with `--readonly`, waits for its socket, gives
QEMU a shared memfd and `vhost-user-fs-pci`, then mounts the `nex-host-store`
tag read-only at `/run/nex-host-store`. The fixture carries the kernel's
existing `outputs/fs-fuse` package so the guest can load `fuse` and `virtiofs`
after boot. A reboot must reap the old daemon and socket before starting a new
one because virtiofsd exits when QEMU disconnects on this host.

`/nex/users/root/manifests` appears lazily. A read-only guest test can use
`/nex/manifests`. A test that writes a manifest before its first Nex command
must create the detached user worktree itself with `git worktree add`.

On the rootless development host, a direct `zub checkout --copy` of a system
can fail with `EPERM` while applying `/boot` metadata. Running the checkout
under `fakeroot` produces a complete tree that works with
`unshare --user --map-root-user --root <tree> /usr/bin/nex --version`.
