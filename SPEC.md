# nex: system specification

---

## 1. Foundations

**Identity.** One hash scheme throughout: git SHA-256 object IDs. The manifest repository is a git repository in SHA-256 object format. A manifest's identity is its git blob ID; the same holds for assemblies. Build outputs and capsule trees are content-addressed in the zub store with the same algorithm.

**Trust.** The git repository is the sole trust root. A signed tag (or a known commit hash) transitively authenticates every manifest, every assembly, every dependency reference, and, via committed checksums, every build output and composed tree. All zub stores, local or remote, are untrusted caches: optional, verifiable on arrival against hashes derived from the repo, and reconstructible by building locally. The single precondition to full local verifiability is the bootstrap (§4).

**Layer rule.** Manifests answer *built against what*: exact, immutable. Assemblies answer *runs against what*: changed by committing a new assembly version, evidence-gated. Updating a library "from under" an executable only ever happens at the assembly layer; manifests never change for it.

**Divergence semantics.** For every runtime edge in a composed system there are exactly three facts: **built-against** (the consumer's manifest pin; immutable, what its testing actually covered), **runs-against** (what the composed tree resolves at that path; the assembly's choice), and **evidence** (why the difference, if any, is believed safe). Everything downstream (divergence reports, policy tiers, substitutions) is bookkeeping over these three. An edge can only diverge through the evidence gate at the tier the assembly's policy demands; divergence is therefore never silent and never unbounded. For calibration: conventional binary distros run permanently in the diverged state, with binaries built against one library revision running against whatever the archive evolved to, and soname stability as the entire unrecorded safety argument. nex records the gap per edge, checks each consumer's imports mechanically, and demands a claim; the gap can additionally be closed in either direction: by convergence rebuilds (making built-against equal runs-against) or by test claims against the composed tree itself (making the composed combination a tested one).

**Ownership rule.** Humans own declarations (manifests, assemblies). The store owns objects and attributes derivable from them. Claims own observations. Anything recomputable from an object is never stored in a file. The single sanctioned exception is the `checksum:` field, which is machine-written output derived from all other bytes of the file plus the build result: the one field that is a function of the rest, excluded from its own hash the way a git commit hash is computed over, not inside, the commit.

**One sealing model.** A manifest and an assembly are the same kind of object at two levels: content that pins its inputs exactly, a build step (compile for manifests, compose for assemblies), and a checksum binding plan to output. Merge-as-seal (§5) applies identically to both.

---

## 2. Repository

**Layout.** One file per package, not per version:

```
pkg/<namespace>/<slug>.yaml        e.g.  pkg/libs/system/libxcrypt.yaml
assemblies/<slug>.yaml             assemblies this repository publishes
```

The `assemblies/` directory holds only the assemblies the repo's owner publishes (for the central repo: the reference assemblies such as nex-systemd). Assemblies in general live wherever their owner keeps them: a private repo, a machine, nowhere durable at all. Publishing an assembly is nothing more than committing it where others can pin it.

Manifests are arch-specific (build flags, patches, and strategy legitimately differ per architecture), but references never spell the arch: the resolver carries the target arch as context and resolves within that arch's subtree. Cross-compilation is expressed through the `environment:` field; the environment manifest defines the toolchain, and deps that vary with the target are the environment's concern. This area is deliberately not elaborated further yet.

**One file per package.** HEAD carrying a single current version per package is a statement, not a restriction: a repo revision *means* "this is the set of current packages." Version coexistence lives where it is needed: in the store (multiple built versions), in assemblies, and in capsule variants. Mid-migration, when two versions must be simultaneously current in the tree, an explicit second file (`glibc-legacy.yaml`) is created for the duration and deleted after; the exception carries the ceremony, not the norm.

**Immutability.** Released manifests are immutable as *objects*, not as *paths*. Editing `libwebp.yaml` from 1.4 to 1.5 creates a new blob; the 1.4 blob persists in git history forever, everything sealed against it keeps verifying, and old blobs resolve from history at any distance. In-place invalidation cannot occur because nothing references a path as its identity; references are blobs.

---

## 3. Manifest format

There is exactly one form. What is committed is what is in the worktree is what humans edit. No projection layer, no rendered views, no tooling that rewrites files for display. `nex` writes files only at the same moments a human does: at commit, on touched files.

```yaml
package:
  slug: libxcrypt
  namespace: libs/system
  arch: x86_64
  version: 4.4.37
  checksum: sha256:fb8aece…        # machine-written; see §3.4. Never written by hand.

sources:
- url: https://github.com/besser82/libxcrypt/releases/download/v4.4.37/libxcrypt-4.4.37.tar.xz
  sha256: 8a5f42a3…

dependencies:
- manifest: blob:3f2a91c…          # git blob ID of the dep manifest: absolute, portable
  path: pkg/libs/system/glibc.yaml # QoL label for humans; covered by the checksum
  use: bundle:dev

build:
  environment: blob:88c02e…        # the build environment is itself a manifest
  script: |
    …

bundles:
  dev: [dev, lib, static]
outputs:
  lib:
    files:
    - path: /usr/lib/libcrypt.so.2.0.0
      needs: [/usr/lib/libc.so.6]

resolution:
  /usr/lib/libc.so.6: glibc        # maps each `needs` entry back to a dependency
```

### 3.1 Dependencies are build-time

All `dependencies:` entries are build-time dependencies by definition: what the build consumes. Runtime dependencies are a *derived subset*: each output file declares what it `needs:` at runtime, and `resolution:` maps those needs back into `dependencies:`. So all runtime dependencies come from `dependencies:`, but many `dependencies:` entries (compilers, build tools) are build-time only and never appear in any resolution.

### 3.2 Dependency references

`manifest:` is the identity: the dep manifest's git blob ID, meaningful in any commit, at any distance; the reference is content, not location. `path:` is a human label beside it. `use:` selects what is consumed: `bundle:<name>`, `output:<name>`, or `output:<name>#<path>` for extracting a single file from an output.

The two maintainer gestures are expressed by editing, not by metadata:

**Update (float to current):** delete the `manifest:` line (or write `manifest: resolve`). At `nex commit`, the ref is resolved from `path:` (worktree first, then HEAD) and the blob is written in. Float is not a stored property; every sealed manifest pins exact blobs, and staying current is an action re-applied whenever a human touches the file.

**Hold:** leave the existing blob untouched. The reason, if worth recording, is a YAML comment: human-owned, checksum-covered, not machine-interpreted. Intent is not a schema field; the dep line itself is the maintainer's decision, made afresh each time the file is touched. The accepted cost is that tooling can see *that* a ref is old but not whether that is deliberate, so `nex modernize` rewrites all stale refs (or proceeds interactively) and the maintainer reverts the lines they meant to keep.

**Path/blob disagreement** has two cases with different handling. Blob differs from the path's content at HEAD: the normal state of an untouched or deliberately held ref; expected, reported by `nex status`, never an error. Path missing or renamed: the label rotted. That is a warning, never a build error (the blob resolves from history regardless), but `nex commit` refuses to reseal a *touched* file with rotted labels, so labels heal at the same cadence staleness does. Fixing a label is a content change and requires a reseal like any other edit.

### 3.3 Staleness dynamics

A ref goes stale only when the referenced package itself is edited: not when anything else in the repo moves, and not when the dep's own deps move (those are the dep's refs). Staleness accrues per-dep at that dep's release cadence and is reclaimed by two forces: any maintainer touch resolves every ref left bare, and CI merge policy may require that new seals resolve deps at HEAD (with reviewed exceptions), making refs current at merge by definition. Hot packages stay near-clean; a manifest untouched for two years shows old blobs, which is accurate (it really was built against them) and actionable (each is one deleted line away from modernized). Lag does not compound: a stale manifest's deps are current-and-sealed at HEAD, so modernizing pulls zero transitive rebuilds; the dev builds exactly one package.

### 3.4 Plan and checksum

```
plan     = H( canonical manifest content, excluding the checksum field )
checksum = H( plan ‖ output )
```

Because dependency references are blob IDs, the plan transitively covers the entire input closure (dep manifests, their deps, the environment, sources' hashes, the script), grounded at the bootstrap (§4). It is computable from a checkout alone, before any build runs, and serves as the a-priori name of a build: demand queue (plans without a recorded output), memoization key, store lookup key.

The checksum binds plan to output and lives in the file, committed. Same checksum ⇒ same closure ⇒ same build, across any two commits, any distance apart. The reference folds the dep's *blob*, and the dep's blob includes its own checksum field, so a manifest's checksum transitively vouches that its deps were sealed, not merely what their inputs were. Operational consequence: a dep must be committed with its checksum filled before dependents can pin it; within a single multi-manifest PR, `nex commit` fills checksums in topological order so in-worktree deps are hashed after being sealed locally.

Canonical encoding of manifests (byte normalization for hashing) is defined once, in one place; the checksum-field exclusion is exactly one field.

---

## 4. Bootstrap

The recursion does not ground in a blessed seed tarball; it grounds in **convergence**. The builder has a bootstrap mode that uses the host's toolchain. The first bootstrap-phase manifests are not reproducible (their output depends on the host) and correctly carry **no checksum**, since a checksum asserts reproducibility. Successive phases rebuild the toolchain with itself until outputs converge to stable, host-independent checksums; from that fixpoint upward, everything is reproducible regardless of the machine that ran the bootstrap.

Rules this imposes:

**Bootstrap-phase manifests are marked as such** and are exempt from checksum reproduction at merge. They are the only permanently unsealed manifests in the repo.

**Nothing outside the bootstrap may depend on an unsealed manifest.** This is the invariant merge-as-seal protects; the bootstrap marker is the scoped exemption, not a loophole.

**Every binary input at the leaves is named by hash.** Bootstrap manifests pin their starting binaries by sha256 in `sources:`, so even the non-reproducible phases have fully declared inputs; what varies is the output, until convergence.

**The convergence claim is the trust ground, and diversity is what makes it evidence.** "These post-fixpoint checksums are host-independent" is verified by re-running the bootstrap from a different host and arriving at the same values. What the claim proves scales with the *diversity* of the starting lineages, not merely their count. This is the diverse-double-compiling argument: a trusting-trust backdoor in one starting toolchain must, to survive undetected, propagate itself across every other lineage and reach the identical poisoned fixpoint from all of them. Convergence runs should therefore deliberately span compiler families (gcc- and clang-based hosts), compiler versions, distro ancestries, and ideally host operating systems. What convergence cannot check, at any N, is a backdoor in the *source* everyone compiles; that exposure is pinned by source hashes, bounded by review, and shared with every distro. It is the floor of the ecosystem, not a nex property.

**The full-source (hex0-style) bootstrap path is deliberately excluded.** Its sole marginal victim over N-way diverse convergence is a backdoor pre-existing in *all* plausible starting binaries while absent from source, which requires a universal, cross-implementation, self-propagating payload present across decades of independently built toolchains: a capability no fragment of which has ever been demonstrated. This scenario is treated as out of scope. The claim nex makes is accordingly precise: *no binary is trusted that N diverse lineages did not independently reproduce*, not "no binary is trusted, period."

**The cache asterisk.** Typically one downloads the bootstrap results from a zub cache rather than re-running it. Post-convergence objects verify against committed checksums as always, but a user who starts from the cache rather than their own host chain is trusting that *someone's* bootstrap honestly reached those checksums, mitigated exactly by the multiplicity of independent convergence runs. This is the single point where "everything is verifiable locally" carries a precondition, and it is stated here rather than left implicit.

---

## 5. Lifecycle: authoring and sealing

### 5.1 Merge is the seal

A change lands as: the author edits (for a routine package bump, typically the version, url, and source hash), builds locally (`nex commit` computes the plan, runs or reuses the build, writes the checksum), and opens a PR showing the human diff. CI at merge rebuilds and must reproduce the committed checksum. **Merge is the seal**, and every merge is therefore a two-builder reproducibility proof across different machines, obtained for free in the PR flow.

Consequences, accepted: outside the bootstrap, the repo never contains unbuilt manifests; CI is a merge gate, not a cache warmer, and demand-driven building applies to downstream and private repos layering on top. Editing a released manifest is simply a new blob; there is no reseal cascade because nothing is invalidated in place. Dependents' refs become *stale*, which costs nothing until each is next touched.

CI verification is a **closure walk**, not a single-node check: recompute every sealed file's checksum over content-sans-checksum plus resolved deps, recursively down to the bootstrap. A single-node check would trust a stale dep checksum field; the walk catches it.

**Trust model, named.** The maintainer builds on their own machine (there is no build bot for authors); CI reproduces at merge. This is two builds but one-and-a-half independent verifications: it genuinely cross-checks determinism and maintainer error, but against compromised CI infrastructure the maintainer's build contributes nothing, since whoever controls CI controls what merges. The trust root is therefore the repo operator's key *and* the repo operator's build infrastructure: reproducible, operator-trusted, the same posture as conventional distros, with the checksum committed where anyone can verify bytes against it. Strengthening is pure merge policy, not system design: requiring the checksum independently reproduced by X parties (additional CI runners on separately provisioned infrastructure, or signed human reproductions) is branch protection, tunable later and per-package (critical packages such as the toolchain and environment manifests warrant a higher X), with no change to any artifact or downstream mechanism. Independent third-party rebuilders checking the operator itself need zero support from nex: plans are computable from the public repo, checksums are committed, anyone can build, compare, and publish disagreement anywhere.

Three measures close the attacks the gate leaves open, all policy and ecosystem rather than mechanism:

**Witness cosigning against equivocation.** A compromised operator's remaining move is showing different signed histories to different users. The committed checksums already form an append-only transparency log; what completes it is k independent witnesses that mirror the repo and cosign each released tag as extending the head they previously saw. Equivocation then requires corrupting the operator and k witnesses, and any split view is detectable by gossip between witnesses. Cosignatures ride on tags; nothing inside nex changes.

**Builder diversity against common-mode compromise.** X reproductions are only X verifications if the builders share no inputs: merge policy for critical packages should require reproducing parties on distinct hardware, separately provisioned toolchains, and where possible distinct host systems. Hermetic builds against the pinned closure must agree regardless.

**Periodic world-reproduction audits.** A poisoned output sealed under a lying checksum by an earlier compromised merge survives per-PR verification forever, because every later build fetches it from cache and it verifies against the lying value. The only detector is periodically rebuilding the world from bootstrap on clean infrastructure and diffing every checksum: schedulable (quarterly, or continuously at low priority by volunteers), and the one audit that reaches back through history.

### 5.2 Dev iteration (single or many manifests)

Resolution order is worktree first, HEAD second. During development, new or edited manifests resolve to `git hash-object` blobs of their current content; no commits are required at any point. Blob IDs churn invisibly on every edit; plans recompute; the local store memoizes on plan, so editing a mid-stack dep rebuilds only it and its dependents.

Landing a large stack (the chromium case: dozens of manifests at once) is one `nex commit` walking the DAG bottom-up: hash each manifest, write resolved blob refs into dependents, fill checksums from the local builds, stage everything, one PR; CI seals the stack topologically at merge. Two free properties: identical manifest content in two PRs yields identical blobs and identical stored references, so parallel PRs adding the same dep merge without conflict; and a large stack may land bottom-up as several PRs, each independently sealed, purely as review-size management.

### 5.3 Updating an old manifest

```
$ nex status pkg/apps/graphics/inkscape.yaml
  deps: 11 current, 3 stale (gtk3, harfbuzz, pango), 0 label-rotted
$ nex modernize pkg/apps/graphics/inkscape.yaml    # deletes manifest: lines for stale refs
$ $EDITOR …                                        # the actual update
$ nex build pkg/apps/graphics/inkscape.yaml        # deps sealed at HEAD → store hits; one package builds
$ nex commit -m "inkscape 1.4"
```

Keeping stale refs is legal (old blobs resolve from history, and CI can rebuild against them) for a minimal-touch fix that does not absorb a migration; whether the repo *accepts* new seals against old deps is merge policy, not mechanics. If the modernized source no longer compiles, the dev doing the update is the demanding party and writes the patch, at demand time.

---

## 6. Store (zub)

Content-addressed object store, ostree-like: blobs, trees, hard-link checkout. Doctrine: untrusted cache, always optional. Resolution of any hash proceeds local repo objects → local zub store → configured remote stores, each later layer verified against the hash the earlier layer would have produced. A remote store can only ever hand you bytes you would re-derive yourself.

**Derived metadata**, computed at `zub commit`, never stored in files: per library output, an interface hash (exported versioned symbols plus dev-output header content, canonicalized) and export table; per ELF, the import table (undefined symbols with version tags); per plan, the recorded outputs and checksums with builder signatures. Multiple independent builders recording the same checksum for the same plan is standing reproducibility corroboration, and a disagreement is a localized alarm naming the exact package. For assemblies, the same records hold plan → composed tree.

**Reverse queries** are first-class: `zub refs --blob <hash>` answers "which trees still link this object", the audit primitive for advisories.

---

## 7. Claims and advisories

### 7.1 Claims

A claim is a signed observation about store objects, verifiable by recomputation: `interface-equal(a,b)`, `abidiff-clean(a,b)`, `abicompat(consumer, lib)`, `bit-identical(a,b)`, `tests-passed(tree)`, and bootstrap-convergence runs (§4). Evidence tiers, weakest to ground truth:

symbol-superset < abidiff < interface-equal < bit-identical < rebuild

Behavior changes under a stable ABI are invisible to every static tier; test claims are first-class for exactly that reason. There is no separate plan→output claim: the committed checksum *is* that record, with the merge gate (§5.1) as its provenance.

**Generation.** Claims are byproducts of work the flows already do, never a separate activity. CI at merge, holding old and new outputs for the bump-cutoff check, computes interface-equal or abidiff and publishes the result: one extra write. Test runs on a composed tree yield `tests-passed(tree)`. Bootstrap runs yield convergence claims. Import/export tables and interface hashes are store metadata computed at `zub commit` (§6), which the cheap claims merely compare.

**Hosting follows the ownership rule.** Recomputable claims (interface-equal, abidiff, and everything derivable from blobs) live store-side, indexed by subject (`zub claims <blob>`), under the untrusted-cache doctrine: any store may carry them, validity rests on the signature and on recomputability, and losing them costs minutes of CPU. Observations of *events* (bootstrap-convergence runs and release test records, whose value is accumulation over time and which no recomputation can restore) are committed into git: the central repo or a claims repo referenced from it, signed history under the trust root like everything else. The store carries both kinds only as cache and index; the store is disposable, the event history is not, and the event history was never store-hosted to begin with.

**Trust in claims is assembly policy.** An assembly's policy names accepted signers per tier, or `recompute` for tiers cheap enough to re-derive locally. Like every policy knob, this is part of what base followers inherit from a curator.

### 7.2 Advisories

An advisory is a human declaration, not an observation, so it lives in the git repo as a committed, reviewed file:

```
advisories/NEX-SA-2026-1143.yaml
```

```yaml
advisory:
  affects: blob:…                  # the vulnerable output
  fixed_by: blob:…                 # the fixing output
  fix_location: object | headers
```

Landed by the fixing maintainer in or alongside the fix PR. `stale: refuse` and `on-advisory` cadence read advisories from the repo revision the assembly resolves against, so "which advisories apply" is as pinned and replayable as everything else. Third-party or private advisories are the same mechanism layered: a downstream repo carries its own advisory files, and an assembly's advisory sources are chosen like its base. A bot watching upstream CVE feeds proposes advisory PRs; it proposes commits, it does not publish around the repo.

`fix_location: headers` bars substitution: code inlined from headers lives in consumers' binaries and is not patched by swapping a shared object. Enforcement is automatic rather than resting on the field: a header-located fix changes the interface hash, so no interface-equal claim can exist and substitution is refused by the evidence requirement itself; the field survives as routing information for humans.

---

## 8. Capsules

A capsule is a closure materialized as a hard-linked tree in the store: every file of the closure at its runtime path, blobs shared across capsules by hard link. Coherence (one resolution per library identity per process image) is enforced by the data structure: one blob per path; the conflict is unrepresentable, not merely forbidden.

Capsule identity is (manifest, resolution) = tree hash. One manifest can have several capsule variants: the tested-with variant (its own pins materialized; what `nex shell <pkg>` enters and what CI runs tests in) and substitution variants differing by individual links. Two versions of a library coexisting on one machine is structurally safe across capsule boundaries; they can meet in a process only by explicit choice (separate processes, static linking, dlmopen: all deliberate compose decisions).

Requirements and rules: store and capsules share one filesystem; everything is read-only (a write through any link edits every capsule, so immutability is a correctness requirement, not a philosophy); per-capsule metadata divergence (e.g. setuid in one but not another) forces a copy, decided at tree-write time. The composer rejects binaries carrying store-absolute paths (a resolution smuggled past the tree) and treats plugin directories as closure members populated by the tree, hence inside the solver's scope.

What capsules do not fence: shared state (/etc, /run, /var, sockets, D-Bus). Version skew crosses there as protocol and format compatibility, invisible to the solver, covered only by test claims.

**Storage characteristics.** Because binaries carry no store paths (resolution lives in trees, and the composer rejects embedded store paths), consumers are unchanged blobs across resolution changes: N program closures sharing a library cost the distinct *builds* of that library once each, plus tree metadata. The closures share everything else by hard link at zero marginal bytes, and a substitution variant costs only directory entries. This removes the growth driver that inflates Nix/Guix stores, where path embedding makes every rebuilt closure all-new bytes and content dedup reclaims only a fraction; their store sizes are an upper bound on badness here, not a prediction. The true growth variable is distinct retained builds per package: directly measurable (distinct blobs per SONAME across installed capsules is a `zub refs` aggregation, a natural health metric) and governed by garbage collection, not by the model. Capsules and deployed trees are GC roots, unreferenced blobs collect, and old boot trees are kept per rollback policy. Reproducibility feeds dedup automatically: same plan ⇒ same blob across capsules built at different times, and bit-identical outputs from different plans dedup by content addressing with no extra mechanism. Practical constraints: hyper-shared blobs (libc in every capsule) can approach filesystem hard-link ceilings on dense hosts; reflinks are the escape, and also serve the per-capsule metadata-divergence case. The honest residual overhead is inodes and directory entries per capsule.

---

## 9. Assemblies

An assembly is a manifest one level up: content that pins its inputs exactly (member manifests by blob), a build step (compose: write capsule trees, merge the bootable tree), and a checksum binding plan to output, where the output is the boot capsule's tree hash. One file, one form, sealed by merge exactly like a package. There is no separate lock artifact; changing what an assembly runs against is committing a new version of the assembly.

```yaml
assembly:
  slug: nex-systemd
  arch: x86_64
  checksum: sha256:0be371…          # machine-written: H(plan ‖ boot-capsule tree hash)

members:
- manifest: blob:9dd1c04…           # same reference shape and gestures as manifest deps:
  path: pkg/core/init/systemd.yaml  #   existing blob = hold, deleted line = resolve at commit
  use: bundle:minimal
- manifest: blob:3f2a91c…
  path: pkg/libs/system/libxcrypt.yaml
  use: output:lib

files:                              # /etc/passwd, units, symlinks: literal content, as before
  …

substitutions:                      # runs-against departures from members' built-against pins
- needs: /usr/lib/libcrypt.so.2
  manifest: blob:aa02f1…            # the replacement provider
  path: pkg/libs/system/libxcrypt.yaml
  use: output:lib
  reason: NEX-SA-2026-1143
  evidence: [claim:sha256:e3ab90…]  # verified by CI at merge; the seal covers the justification

policy:                             # standing configuration the tool applies at each commit
  float:
    "*":                    {level: patch, evidence: [interface-equal]}
    pkg/libs/system/glibc:  {level: minor, evidence: [abidiff-clean, symbol-resolve]}
  security:
    evidence: [interface-equal]     # minimum tier for advisory-driven substitution
    stale: refuse                   # composing a tree linking an advisory-tagged blob fails
  cadence: [on-advisory, weekly]

base:                               # optional: inherit a published assembly (§11)
  manifest: blob:88a2c1…            # the base assembly's blob, like any other reference
  path: assemblies/nex-desktop.yaml
  follow: [security]                # auto-adopt the publisher's security-tagged commits
```

**Sealing.** `plan = H(content sans checksum)`; composing the plan yields capsule trees and the merged boot tree; `checksum = H(plan ‖ boot tree hash)`. CI at merge recomposes and must reproduce the tree: cheap, since composing writes trees rather than running builds. The claims referenced under `substitutions:` are verified at merge, so a sealed assembly's checksum vouches for its evidence, not just its bytes. Deploy resolves the boot tree via the store's plan → tree record, verified against the committed checksum; compose is a pure function of (assembly content, store objects), reproducible forever with no resolver and no repo state.

**Policy application is a commit-time act.** Float and advisory response depend on external mutable state (what exists, what claims are published), exactly as manifest float depends on HEAD. The resolution is the same: the tool applies policy when the owner (or an automation bot opening a PR) commits, and the committed content is fully pinned. Whether a given commit was the *best* application of policy at that moment is a review question, like any human edit; reproducibility of the result is total.

**Derived, never stored:** the divergence list (every edge where a consumer's built-against blob differs from what the tree resolves at that path) is computable by walking the composed tree against member pins, and is `nex status` output, not file content. Likewise `stale: refuse` is a compose-time check against published advisories, not a recorded list.

**The solver**, run at commit: resolve members to plans; fetch outputs by checksum from any store (content-checked on arrival, value-checked against committed checksums) or demand-build misses, memoized on plan; unify per capsule, with multi-manifest unification only where closures merge into one process image (the boot capsule, cross-manifest plugins); for every runs-against ≠ built-against edge, check the consumer's import table against the candidate's export table and require the policy's evidence tier; write trees.

**Direction rule:** new library under old binary is the safe direction, verified per consumer via versioned symbols; the inversion (a binary built against a newer library handed an older one) is refused mechanically because its version tags do not resolve.

**Unsatisfiable** edges produce a report with a named culprit and priced options: rebuild X against the new dep (interface diff shown; often a relink), hold the edge, or isolate (separate process or namespace). The demanding party chooses; if source patching is needed, the demanding party does it, at demand time. Rebuilds are memoized: the second assembly demanding the combination gets a cache hit.

### 9.1 Compose modes

An assembly declares how far unification reaches:

```yaml
compose: capsules | merged
```

**capsules** (default): members live in per-manifest capsules; only the boot capsule (init, shells, daemons: closures that genuinely share a process image) is unified. Multiple builds of the same SONAME coexist structurally across capsule boundaries; divergence exists only where explicitly substituted.

**merged**: every member is unified into one flat tree: one blob per path, one build per SONAME by construction. This is the embedded/appliance mode: it trades coexistence for a single storage copy and a conventional rootfs layout. The consequence is discipline, not mechanism: the whole system becomes one unification domain, so every diamond must resolve to unify-with-evidence or rebuild. Coexistence is no longer an available answer, and divergence frequency rises accordingly. The assembly owner is the demanding party for the resulting rebuilds; those rebuilds are memoized and seed the shared cache.

The recommended embedded posture combines the two gap-closing directions from §1: releases ship **converged**. A `tested-with` policy under merged mode surfaces every diamond, resolved by rebuilding stragglers against the shipped set at release time, so everything on the device is built-against what it runs-against and the divergence report is empty or deliberate. Between releases, substitution-with-evidence is reserved for emergency field patches; the next release converges again. Where convergence is not paid for, the composed combination can still be *made* tested: run each member's test suite inside the merged tree (not its tested-with capsule) and publish `tests-passed` claims against the boot tree hash. That is the operational meaning of "tested with the libraries it will actually use," attached to the exact artifact shipped. The divergence report at release time is a release-gate artifact: one screen answering what on this image runs against something it was not built with, and why that is believed safe.

### 9.2 Export

A device need not carry zub. `zub export <tree>` emits a sealed boot tree as a plain filesystem image (squashfs, erofs, or a directory) for A/B slot updates; alternatively a device keeps a minimal store and receives delta updates with hard-link-cheap rollback. Both are export formats of the same sealed tree hash, so a firmware image is a pure function of the assembly blob: reproducible firmware with no further mechanism.

---

## 10. Flows

**Routine bump (the CVE three-liner).** The maintainer edits version, url, source hash; `nex commit` builds and seals; PR; CI reproduces; merge. Maintainer responsibility ends at merge. CI compares old vs new interface hash of the changed package: equal (the dominant case for patch releases) ⇒ publishes interface-equal, and the rebuild queue is empty by construction; no downstream build is even scheduled.

**Security substitution.** An advisory lands with `fix_location: object`. For assemblies with `on-advisory` cadence, a bot (or the owner) commits a new assembly version adding the substitution entry: evidence claim referenced, direction check per consumer, variant trees written with one link changed. CI verifies and seals at merge. Deploy is an atomic root swap; the old tree is kept for rollback. Audit: `zub refs --blob <vulnerable>` lists every tree still linking it; assemblies with `stale: refuse` fail their own next compose until updated, so pressure comes from the consumer's own policy, not from nagging humans. Convergence rebuilds are optional background work; divergences shrink as built-against catches up, and the substitution entry is removed when no longer needed: an ordinary edit in an ordinary commit.

**Toolchain/glibc bump.** Same path, never a forced world rebuild. The interface hash differs (headers changed), so no interface-equal claim; instead abidiff-clean plus a compose-time symbol-resolve check of every ELF's imports against the new exports (verifying, not trusting, the library's versioned-symbol contract). The world floats forward as runs-against ≠ built-against with evidence; rebuilding against the new headers is background convergence on each assembly owner's schedule, or never.

**Breaking bump.** The dep removed a symbol: no equal claim, abidiff reports a removal, composition is unsatisfiable for affected consumers, and the report names them with options. Typically: rebuild the one consumer (a relink if only the interface moved), memoized for everyone after.

**Compose / deploy / verify.** `nex compose <assembly>` = checkout of the boot capsule by tree hash. `nex verify --closure` walks tag → assemblies and manifests → dep blobs → checksums → plan/output records → blobs → bootstrap convergence: one chain, no dangling names. `git log` on a published assembly file is the audit history of what its followers run.

---

## 11. Published assemblies and inheritance

A curator is an assembly owner whose sealed assembly others pin: a role expected to cover most users. A user assembly references the base by blob, like any dependency: base members' capsules are inherited verbatim, bit-identical for every user of that base blob, hence perfect cache sharing. The solver runs only over the user's delta and the seam: their packages build against interfaces from the base's pinned blobs, and unification triggers only where their closures merge with base closures in one process. Advancing the base is deleting the `manifest:` line under `base:` and committing (resolve to the publisher's current), or automatic for commits matching `follow:` tags; either way the result is a new sealed user assembly against a pinned base blob, reversible by reverting the commit.

Overrides (forcing a base member elsewhere) are legal and priced: the user owns every affected closure's re-solve, and distance from the base is visible as the diff between the two assembly files. Inheritance flows downward only; a lower tier never re-solves an upper tier's interior edges. Tiers stack (distro → organization → team → user), each owning its delta. The zero-effort floor is an assembly containing nothing but a `base:` reference: full inheritance of members, policy, evidence bar, and a warm cache.

---

## 12. Responsibility ledger

| Party | Owns | Does not own |
|---|---|---|
| Package maintainer | Their manifest building and passing its tests at merge | Anything downstream; any pin-bumping treadmill |
| Nobody | Intermediate pins: they fall out of committing and solving | (nothing) |
| Assembly owner / curator | Policy, cadence, and exception decisions for systems they actually run or publish | Other people's manifests (may patch as demanding party) |
| Central repo | Being a well-known publisher with a warm cache and a merge gate | Authority: any repo plus a trust key works identically |

Genuine source breakage is fixed by the demanding party at demand time: the only assignment that does not manufacture obligations for absent volunteers.

---

## 13. Standing concessions

Header-located fixes require rebuilds; the interface-hash gate refuses the substitution automatically. Semantic breakage under a stable ABI is invisible to all static evidence; only tests catch it. Static-linking ecosystems (Rust, Go) get memoization and bump cutoff but not substitution; every dep change is a rebuild there. Shared state across capsules is outside the solver: test claims only. Closure verification is a walk, not a pointer check. Bootstrap-from-cache carries the stated trust precondition (§4). SHA-256 git repos are second-class on some forges; verify hosting before committing to it (fallback if blocked: a bespoke canonical-encoding hash carried alongside git, with the same semantics and more tooling). Cross-compilation via `environment:` is supported in principle and deliberately unelaborated.

---

## 14. Build order

1. Repo in SHA-256 format; canonical schema and encoding for manifests and assemblies; bootstrap marking; `nex commit` (resolution, plan, checksum, DAG-ordered multi-file), `nex status` / `nex modernize`.
2. CI closure-verify + merge-as-seal, with the bootstrap exemption.
3. zub: objects, trees, hard-link checkout, reverse-blob query, interface/import metadata at commit, plan → output/tree records.
4. Assembly sealing: solver, capsules and variants, substitutions, direction checks, `nex compose` / `deploy` with atomic swap and rollback.
5. Claims (interface-equal first, abidiff tier second), advisories, the substitution flow, convergence claims.
6. Published assemblies and inheritance; demand-driven build service for downstream repos.

Each step is independently useful; the system is operational for a single owner after step 4; steps 5–6 thicken as publishers and rebuilders appear.
