# Ship manifests as a Git bundle and rename the store

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this plan, an installed machine holds a real Git clone of the Nex
repository, with history, produced offline from a single file in the image. A
user upgrades the machine by `git pull`, `nex build`, `nex deploy`, which is the
design this repository was always aiming at.

Two changes land together because both rewrite first boot, and editing
`nex-init-manifests` twice is worse than editing it once.

**The manifests arrive as a Git bundle.** Today `examples/desktop-vwl/desktop-vwl.yaml`
declares six `dev:` sources that tar working-tree directories into
`/usr/share/nex/manifests`, and first boot runs `git init` plus one commit over
the result (`base/nex-systemd.yaml:222`). That produces a repository with no
past, which is why manifests that name environments by historical blob cannot
resolve on a machine. It also ships 4.1 GB: `.nex-dev-prepare` nests a copy of
`/var/home/wegel/work/perso/zub` at `vendor/zub-store/zub`, whose `target/`
directory of 3.8 GB escapes the `rm -rf` on the line below. A bundle of the
whole repository is 7.7 MB and `git bundle verify` reports it records a complete
history.

**`repo` becomes `store`.** The path `/nex/repo` and the flag `--repo` name a
zub content-addressed object store, while `/nex/manifests` names a Git
repository. Two things called "repo" cost real time in August 2026. The code
already prefers the other word: `src/cli/src/main.rs:18` declares `pub mod
store`, the dependency at `Cargo.toml:27` is the `zub-store` crate, and prose
says "the store" 15 times against 29 uses of `/nex/repo`.

## Progress

- [ ] Confirm which `git` the `standard` build environment provides.
- [ ] Add the `git_bundle` source kind with a recorded sha256.
- [ ] Prove two strict builds produce a byte-identical bundle.
- [ ] Replace the six `dev:` sources in desktop-vwl with one bundle source.
- [ ] Rewrite `nex-init-manifests` to clone from the bundle and set `origin`.
- [ ] Delete `src/cli/.nex-dev-prepare`.
- [ ] Rename `/nex/repo` to `/nex/store` and `--repo` to `--store`.
- [ ] Write the migration for machines carrying the old path.
- [ ] Rebuild every affected assembly and record checksums.
- [ ] Run EP017's journeys against the result.

## Surprises & Discoveries

(none yet)

## Decision Log

- Decision: Generate the bundle with `git -c pack.threads=1 bundle create`.
  Rationale: Measured on 2026-08-19. Three default runs produced three
  different sha256 values. Three runs with `pack.threads=1` produced one value,
  at the same 7.37 MB. Adding `pack.window=0 pack.depth=0` is also
  deterministic but grows the file to 24.87 MB, so delta compression stays on.
  The nondeterminism was parallel work-splitting, not content.
  Date/Author: 2026-08-19 / Claude

- Decision: Bundle an explicit commit, never a branch name or `HEAD`.
  Rationale: A demo clone failed with `remote HEAD refers to nonexistent ref`
  because the bundle carried `main-prep` while HEAD named `main`. A commit also
  removes any dependence on the working tree being clean.
  Date/Author: 2026-08-19 / Claude

- Decision: Record the bundle's sha256 in the manifest, as `cargo_lock` already
  does.
  Rationale: It makes the snapshot a verified input rather than whatever the
  build host happened to produce, and it restores pinning, which `dev:` blocks
  (`link.rs:284`).
  Date/Author: 2026-08-19 / Claude

## Outcomes & Retrospective

(fill in at completion)

## Context and Orientation

A **Git bundle** is one file holding what Git would send over the network for a
clone: a text header naming each ref and its commit, then a standard packfile.
`git clone <file> <dir>` treats it as a remote. It can carry a complete history
or a range with prerequisites, and `git bundle verify` checks it.

What lives where on a machine today:

- `/usr/share/nex/manifests`, the snapshot inside the deployment, read-only.
- `/nex/manifests`, the writable copy first boot creates.
- `/nex/users/<u>/manifests`, a `git worktree` of that (`repo.rs:261`).
- `/nex/repo`, the zub store, persistent across deployments.
- `/nex/db/pkg`, yaml-only manifests written at assembly time by
  `deploy_manifests_to_nex_db`, describing what this deployment materialized.

`/nex/db/pkg` stays. It is pinned per dependency and yaml-only, whereas the
clone tracks a branch and carries the non-yaml build inputs.

## Plan of Work

Do the bundle first and prove it, then the rename, then rebuild once.

The bundle source kind mirrors `cargo_lock`: a declaration in the manifest, a
generator in `src/cli/src/outputs/sources.rs`, and a sha256 the build verifies.
The generator runs `git -c pack.threads=1 bundle create <out> <commit>` against
the repository that owns the manifest.

The rename is mechanical but touches on-disk layout, so it needs a migration:
an existing machine has `/nex/repo` populated and must not lose it. A symlink
from the old path to the new one is the cheapest correct answer; decide and
record it.

## Concrete Steps

1. Check which `git` the `standard` environment provides, and confirm the
   bundle is generated inside the build sandbox rather than by the host's git.
   Pack format can differ between Git versions, which would move the checksum.
2. Add `git_bundle: <commit>` to `Source` in `src/cli/src/manifest/types.rs`
   and a `fetch_git_bundle` beside `fetch_cargo_lock` in `outputs/sources.rs`.
3. Build the bundle twice in one strict run and confirm the checksums match.
4. In `examples/desktop-vwl/desktop-vwl.yaml`, replace `nex_pkg`, `nex_base`,
   `nex_examples`, `nex_installer`, `nex_scripts`, `nex_src_cli` with one
   `nex_bundle` source. Install it at `/usr/share/nex/nex.bundle`. Delete the
   extraction loop and the `sed -i '/checksum:/d'` that existed only because a
   working-tree tarball could change between the two `--check` passes.
5. Rewrite `/usr/local/bin/nex-init-manifests` in `base/nex-systemd.yaml` to
   `git clone /usr/share/nex/nex.bundle /nex/manifests`, then
   `git remote set-url origin <the real remote>` so a networked machine can
   pull. Keep the existing guard that exits when `/nex/manifests/.git` exists.
6. Delete `src/cli/.nex-dev-prepare`. Keep the `dev:` source kind itself; it
   remains the deliberate unpinnable escape hatch for local iteration.
7. Rename `/nex/repo` to `/nex/store` across the 29 references, rename the
   `--repo` flag to `--store`, and add the migration.
8. Rebuild every assembly with `--single --check --update-checksum` and record
   the new checksums here.
9. Run EP017's four journeys. Journey 1 is the acceptance test for the whole
   plan: a machine that can build a package is a machine whose manifests
   resolved.

## Validation and Acceptance

The plan is done when:

- Two strict builds of desktop-vwl produce the same checksum, proving the
  bundle is byte-reproducible in the sandbox.
- The shipped image contains `/usr/share/nex/nex.bundle` and no
  `/usr/share/nex/manifests` tree.
- The image no longer contains `vendor/zub-store`, and desktop-vwl's root drops
  by roughly 3.6 GB.
- On a booted machine, `git -C /nex/manifests log --oneline | wc -l` is greater
  than 1, and `git -C /nex/manifests cat-file -t 27b6e5dc...` prints `blob`.
- `nex link examples/desktop-vwl/desktop-vwl.yaml` succeeds, where today it
  refuses because of the `dev:` sources.
- EP017's journeys pass, journey 1 included.
- `/nex/store` exists on a machine upgraded from an older deployment, with its
  objects intact.

## Idempotence and Recovery

`nex-init-manifests` already exits when `/nex/manifests/.git` exists, so a
rerun is a no-op. Bundle generation is a pure function of a commit, so a failed
build can simply be rerun. The rename migration must be safe to run twice:
check for the new path before touching the old one.

If the bundle turns out not to be reproducible in the sandbox, stop and record
why in `Surprises & Discoveries` before changing the design. The fallback is to
generate it once and treat it as a `file:` source with a recorded sha256, which
loses automatic regeneration but keeps determinism.

## Artifacts and Notes

Record in this plan: the git version in the sandbox, the bundle sha256, its
size, and the before and after image sizes.

Set `TMPDIR=<repo>/.nex/tmp/mktemp` for builds. `/tmp` is tmpfs on this host and
assembly builds exhaust it.

## Interfaces and Dependencies

New manifest interface: the `git_bundle` source kind, usable by any manifest.

Changed machine interface: `/nex/store` replaces `/nex/repo`, `--store`
replaces `--repo`, and `/usr/share/nex/nex.bundle` replaces the
`/usr/share/nex/manifests` tree.

Depends on EP017 for verification. Does not depend on EP019, but makes it
easier, since a repository split is simpler once the machine consumes a bundle
rather than six tarred directories.
