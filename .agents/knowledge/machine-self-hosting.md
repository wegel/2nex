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
