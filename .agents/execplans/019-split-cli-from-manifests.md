# Split the Nex CLI out of the manifests repository

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this plan, the Nex CLI lives in its own repository and `pkg/core/nex/nex.yaml`
builds it from a pinned release, exactly as `pkg/core/nex/zub.yaml` already
builds zub from a pinned Git revision. The manifests repository then contains
manifests, and a machine that runs `git pull` sees package changes rather than
tool changes.

The gain is not size. Measured on 2026-08-19, `src/cli` is 645 blobs and 7.7 MB
across the whole history against 154.8 MB for everything, so removing it shrinks
a bundle by about 5 percent. The gains are these:

- A tool release stops touching every machine's package repository.
- The CLI gets its own tests and release cadence, independent of 700 manifests.
- Nex builds its own tool the way it builds everything else, from a pinned
  external source, which is the property the project claims.

The constraint this introduces: a manifests repository at some revision needs a
CLI new enough to read it. `schema: 1` appears in every manifest and is the hook
for stating that contract, but today nothing enforces it.

A user must still be able to build the entirety of their system, the CLI
included. This plan changes which repository carries the CLI source, never
whether it can reach a machine.

## Progress

- [ ] Decide the compatibility contract and where it is checked.
- [ ] Create the CLI repository with full history for `src/cli`.
- [ ] Point `pkg/core/nex/nex.yaml` at a pinned revision of it.
- [ ] Prove a strict two-build of the nex package from the new source.
- [ ] Remove `src/cli` from the manifests repository.
- [ ] Confirm a machine can still build the CLI.
- [ ] Run EP017's journeys.

## Surprises & Discoveries

(none yet)

## Decision Log

- Decision: Do not ship the crate vendor bundle in the image.
  Rationale: Rebuilding the CLI on a machine would then need no network, but no
  other package has that property. Building any package fetches its upstream
  source first, which is why `inputs_cache` on the development host is 19 GB.
  Pre-carrying roughly 326 MB to make exactly one package offline-buildable is
  a halfway position that buys little and costs image size. A machine that
  rebuilds packages has network; a machine that does not cannot rebuild
  anything regardless. If offline rebuilds are wanted later, the coherent
  answer is a source mirror, not a special case for the CLI.
  Date/Author: 2026-08-19 / Human

- Decision: Preserve `src/cli` history in the new repository rather than
  starting fresh.
  Rationale: `git filter-repo --path src/cli` keeps authorship and lets a
  bisect cross the split. Starting fresh discards the record of how the tool
  reached its current behaviour.
  Date/Author: 2026-08-19 / Claude

## Outcomes & Retrospective

(fill in at completion)

## Context and Orientation

`pkg/core/nex/zub.yaml` is the model to copy. It builds a Rust program from a
pinned Git revision with a `cargo_lock` source that records a sha256 of the
generated vendor bundle, then `cargo build --offline --locked`. Twenty packages
in this repository already use `cargo_lock`.

`pkg/core/nex/nex.yaml` today builds from a `dev:` source through
`src/cli/.nex-dev-prepare`. EP018 deletes that script. If EP018 has landed, this
plan starts from a manifest that already has no `dev:` source; if it has not,
this plan must remove it.

The CLI binary already ships as a package: `base/nex-systemd.yaml:97` includes
`x86_64/pkg/core/nex/nex/0.1.0/outputs/bin`. So a machine gets a working `nex`
without any of this. What this plan governs is rebuilding it.

## Plan of Work

Three phases, each independently verifiable.

**Phase 1, the contract.** Decide what a manifest revision requires of a CLI
version, and where a mismatch is reported. The candidates are: a minimum CLI
version recorded in the manifests repository, a `schema` version the CLI
declares support for, or both. Nothing else in this plan is safe until a
mismatch produces a clear message rather than a parse error.

**Phase 2, the new repository.** Extract `src/cli` with history, publish it,
and repoint `pkg/core/nex/nex.yaml` at a pinned revision plus `cargo_lock`.
At this point both repositories carry the code and everything still builds.

**Phase 3, the removal.** Delete `src/cli` from the manifests repository, and
confirm a machine can still build the CLI from the pinned source.

## Concrete Steps

1. Write the compatibility contract into `MANIFESTS_CODE_STYLE.md`, and
   implement the check with a test that a too-old CLI reports it clearly.
2. `git filter-repo --path src/cli` into a fresh clone, verify the history and
   that it builds standalone.
3. Publish the CLI repository. Record its URL here.
4. Rewrite `pkg/core/nex/nex.yaml` on the `zub.yaml` pattern: a pinned source,
   a `cargo_lock` source with a recorded sha256, and
   `cargo build --release --offline --locked`.
5. Build it with `--single --check --update-checksum` and record the checksum.
6. Remove `src/cli` from the manifests repository, along with any references,
   including the `nex_src_cli` source if EP018 has not already removed it.
7. Rebuild every assembly and record the new checksums.
8. Run EP017's journeys, and add one that builds the nex package on the machine,
   since "build the entirety of my system" now includes the tool.

## Validation and Acceptance

The plan is done when:

- `pkg/core/nex/nex.yaml` names a pinned revision and a `cargo_lock` sha256, and
  reproduces its checksum across two strict builds.
- The manifests repository contains no `src/cli`, and `nex check` passes on
  every manifest.
- A booted machine builds the nex package successfully.
- A manifest revision that needs a newer CLI produces a clear message naming the
  required version, proven by a test.
- EP017's journeys pass, plus the new CLI-build journey.

## Idempotence and Recovery

Phases 2 and 3 are separated on purpose: after phase 2 both repositories hold
the code and nothing is lost, so phase 3 is the only irreversible step and it
happens after the pinned build is proven. If phase 3 goes wrong, the manifests
repository can be restored from the commit before the removal.

The extraction is done on a throwaway clone. The manifests repository is not
rewritten by this plan; `src/cli` is removed by an ordinary commit so that
history stays intact and bisect still works.

## Artifacts and Notes

Record here: the new repository URL, the pinned revision, the `cargo_lock`
sha256, the nex package checksum before and after, and the size change to a
bundle produced by EP018.

The roughly 326 MB crate vendor does not ship; see the Decision Log.

## Interfaces and Dependencies

New external interface: the CLI repository, and the compatibility contract
between a manifests revision and a CLI version.

Depends on EP017 for verification, and reads better after EP018, which removes
the `dev:` source this plan would otherwise have to unwind first. It does not
strictly require EP018.
