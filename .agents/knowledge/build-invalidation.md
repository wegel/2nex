# Build Invalidation

## Staleness does not propagate to dependents

Nex marks a dependency stale by its own manifest hash and never by the state of
what it depends on. `src/cli/src/build/orchestration/graph.rs:173` resolves a
floating dependency ref, computes that dependency's manifest hash, and calls
`build_exists_for_manifest(repo, ref, manifest_hash)`. A missing build for that
hash means stale, and `graph.rs:303` prints `manifest changed, rebuilding`.

The cache key is per node. It is the package's own manifest hash plus its ref.
It is not a hash of the dependency closure. A package therefore stays cached
when a package it depends on changes.

Observed on 2026-08-19. `pkg/libs/graphics/librsvg.yaml` changed and librsvg
rebuilt to a new output, then:

    nex build pkg/libs/graphics/gtk4.yaml --dry-run --verbose
      [floating] x86_64/pkg/libs/graphics/librsvg/2.61.3/bundles/dev skipping
      Package gtk4 already built, skipping

Fourteen packages depend on librsvg: chromium, gnome-desktop,
gnome-text-editor, gtk3, gtk4, gtkmm, gtkmm3, gtksourceview5, ironbar,
libspelling, loupe, nautilus, pavucontrol, remmina. None of them rebuilt, and
no ref style changes that. Pinning selects which dependency manifest resolves.
It does not decide whether the dependent is stale.

## Why the assemblies still shipped the new library

Shared libraries reach a root at assembly materialization time, not at package
build time. `src/cli/src/materializer/checkout.rs:245` walks the root commit map
and flattens each package's runtime libraries into that package's capsule,
reading the store as it goes. A built root has no librsvg capsule of its own.
`/usr/lib/librsvg-2.so.2` in `desktop-vwl` resolves into a consumer capsule such
as `nex/pkg/apps/editor/gnome-text-editor/47.4/91af9c37/usr/lib/`.

Verified: the flattened copy in the built root and the store output at
`x86_64/pkg/libs/graphics/librsvg/2.61.3/outputs/lib` share the same sha256
prefix `6befafaa9981135d`. Rebuilding a library and its assemblies is enough to
ship the new library, which is why the gap above stayed invisible.

## Open question, do not change without approval

The human called this behavior incorrect and counter-intuitive on 2026-08-19,
from the point of view of developing Nex and its packages. Revisit it as a
design question, not as a defect to patch on sight. The human will approve any
modification or design change when the work reaches that point.

Points to weigh when it does:

- A closure-aware cache key would rebuild dependents correctly and would also
  rebuild large packages such as chromium on any leaf change.
- The current per-node key makes a package's recorded checksum a claim about
  its own manifest only. It is not a claim that the package still reproduces
  against its current dependencies.
- A middle path is a check mode that reports which dependents a change
  invalidates without rebuilding them.
- Any change here interacts with `--check`, which already bypasses the
  staleness check, and with pinned `manifest_ref` values.
