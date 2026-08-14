# Freeze OSTreefy Parity Matrix

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Create the source-of-truth matrix that Ralph will use to drive full parity
with the human's OSTreefy personal system. After this sub-EP, a new agent can
open `.agents/ostreefy-parity-matrix.md` and see every old package or behavior,
the current Nex status, the sub-EP that should handle it, and the proof needed
before marking it complete.

## Progress

- [x] (2026-06-28 04:36Z) Found live OSTreefy source files at
  `/home/wegel/work/wegelcorp/ostreefy`.
- [x] (2026-06-28 05:23Z) Extracted the package list from the base and personal OSTreefy
  Containerfiles.
- [x] (2026-06-28 05:23Z) Classified every package into one current Nex status.
- [x] (2026-06-28 05:23Z) Wrote `.agents/ostreefy-parity-matrix.md`.
- [x] (2026-06-28 05:23Z) Ran the matrix consistency check:
  `expected=163 rows=163 unique_rows=163`.
- [x] (2026-06-28 05:24Z) Updated the main EP with the completed matrix result.

## Surprises & Discoveries

- Observation: The live OSTreefy tree exists outside this checkout.
  Evidence: `rg -n "Containerfile.wegel|ostreefy|qemu-full|wlroots0.20|nvidia-580xx" /home/wegel/work/wegelcorp` found `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/Containerfile` and `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/examples/Containerfile.wegel`.
- Observation: The live package list has 163 unique matrix inputs when base
  packages, personal packages, the AUR loop, and the local Chromium archive
  entry are counted once.
  Evidence: The matrix consistency script printed
  `expected=163 rows=163 unique_rows=163` after checking
  `.agents/ostreefy-parity-matrix.md` against both live Containerfiles plus
  the AUR and local archive entries.

## Decision Log

- Decision: Use `/home/wegel/work/wegelcorp/ostreefy` as the primary package
  source and `OSTREEFY_REPLICATION_REPORT.md` as a cross-check.
  Rationale: The live tree is the most direct source for the old personal
  image, while the report records earlier mapping work and Nex-specific
  judgment.
  Date/Author: 2026-06-28 / Ralph

- Decision: Track program and behavior parity rather than exact Arch package
  name parity.
  Rationale: The human clarified that Nex package names do not need to match
  old Arch package names, but the programs the human uses must be packaged.
  Date/Author: 2026-06-28 / Ralph

- Decision: Defer the local ungoogled Chromium archive entry by request.
  Rationale: The human said it is okay to skip the custom ungoogled Chromium
  package for now.
  Date/Author: 2026-06-28 / Ralph

## Outcomes & Retrospective

`.agents/ostreefy-parity-matrix.md` now lists 163 old inputs. Each row has one
status, a handling sub-EP when future work remains, and a concrete proof that a
later sub-EP must run before final parity.

## Context and Orientation

Primary source files:

- `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/Containerfile`
- `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/examples/Containerfile.wegel`
- `OSTREEFY_REPLICATION_REPORT.md`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-dev.yaml`

Durable notes:

- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/package-manifests.md`

The matrix must not treat Arch boot packages as automatic Nex package work.
Nex replaces `ostree`, `grub`, and `mkinitcpio` through its own zub and boot
path unless a user-visible behavior remains missing.

## Plan of Work

1. Extract package names from both OSTreefy Containerfiles.
2. Compare the extracted list with `OSTREEFY_REPLICATION_REPORT.md`.
3. Compare package names to current Nex manifests and assembly package entries.
4. Write `.agents/ostreefy-parity-matrix.md` with grouped rows.
5. Update this sub-EP and the main EP.

## Concrete Steps

Run from the repository root:

```bash
sed -n '1,260p' /home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/Containerfile
sed -n '1,260p' /home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/examples/Containerfile.wegel
rg -n "Missing Package Manifests|Step 1|Step 2|Step 3|Step 4|Step 5|Step 6|Step 7" OSTREEFY_REPLICATION_REPORT.md
```

## Validation and Acceptance

This sub-EP is complete when `.agents/ostreefy-parity-matrix.md` exists and:

- lists every package from the two OSTreefy Containerfiles
- assigns one status to every package
- names a handling sub-EP for every package that is not already covered or
  deliberately replaced
- records where the package came from: base Containerfile, personal
  Containerfile, AUR loop, local package archive, or copied tree behavior
- names the proof that will be required before a future sub-EP marks that row
  complete

This is planning work, so it does not need a package build. Its concrete check
is a matrix consistency check that proves there are no duplicate or untriaged
rows.

## Idempotence and Recovery

The matrix can be regenerated from the two OSTreefy Containerfiles and the
current Nex manifests. If a later sub-EP implements a package, update the row
status rather than replacing the matrix.

If the live OSTreefy tree disappears, use the archived copy at
`/home/wegel/work/wegelcorp/nex.archive-20260627T143546Z/tmp/ostreefy`.

## Artifacts and Notes

The matrix artifact will be:

```text
.agents/ostreefy-parity-matrix.md
```
