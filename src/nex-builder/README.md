# Small Nex graph builder

This crate builds one package or every package needed by one system assembly.
It keeps package execution, graph planning, and assembly work in separate
modules so later features do not need to enter the package build loop.

Before any build script runs, the binary:

1. Loads every reachable package and base assembly.
2. Maps each named Zub package ref to
   `catalog/pkg/<namespace>/<slug>.yaml`.
3. Checks that the manifest has the requested version, files tree, output, or
   bundle.
4. Loads every build environment.
5. Checks raw Zub hashes and rejects missing manifests or dependency cycles.

The scheduler then runs ready nodes in a stable order. `--jobs` limits the
number of scripts that run together. `--cpus` sets the total CPU budget, and
each running script receives its share through `{num_cpus}`. The default
`--jobs 1` uses the same scheduler as a parallel run.

Each worker owns a clean root and a named log under
`<build-dir>/logs/<graph-index>-<node>.log`. The normal terminal view draws one
bar for the graph and one for each running node. A node with a `build.profile`
shows a wall-clock ETA and adjusts it as output crosses recorded checkpoints.
A node without a profile shows elapsed time and output bytes instead. When one
worker fails, the scheduler starts no more nodes and waits for scripts that
already run. Two workers may safely share the source cache.

A package worker checks out build dependencies, verifies and stages sources,
runs the package script in Linux namespaces, checks the output tree, commits
its raw files, selects output trees without copying blobs, builds bundles, and
publishes all refs. An assembly worker checks out
build tools at its root, checks out its base and packages under `target`,
writes declared files without following a symlink outside `target`, runs its
script, and publishes `systems/<slug>/<version>`.

A dependency normally checks out its complete ref. To take a few paths from a
larger tree, keep the Zub ref exact and list absolute paths separately:

```yaml
- commit: x86_64/pkg/libs/system/glibc/2.39/files
  paths:
  - /usr/bin/ldd
  - /usr/bin/getent
```

With `--check`, each stale node builds twice from clean roots and must produce
the same tree checksum. Its progress bar covers both builds. The scheduler
reuses a node only when every expected ref exists and a Zub commit carries the
manifest's tree checksum. It never uses a timestamp or Zub commit hash as the
result identity.

`--verbose` prints prefixed build-script lines while keeping the bars.
`--no-progress` prints stable status lines for logs and CI. `--quiet` suppresses
status and the final summary while the per-node log files remain available.
`--record-profile` rebuilds every node and replaces each successfully built
manifest's timing profile. It writes the same ten weighted checkpoints used by
the previous builder and records only the first build when `--check` is also
present.

The `nex-source` crate downloads plain URLs, reads local files, and creates
deterministic Git bundles. It also rebuilds Cargo vendor trees from
`Cargo.lock`, Go vendor trees from `go.sum` and `go.mod`, and recursive Zig
package caches from `build.zig.zon`. Every handler writes through a locked
SHA-256 cache entry, so concurrent workers share downloads safely.

Remote stores, cross-machine scheduling, and shared timing databases remain
outside this foundation.

Build an assembly with:

```text
cargo run --manifest-path src/nex-builder/Cargo.toml -- \
  catalog/assemblies/base/flat-minimal.yaml --repo .nex/repo --jobs 4 --cpus 16
```

Run the checks with:

```text
cargo test --manifest-path src/nex-builder/Cargo.toml --workspace
cargo clippy --manifest-path src/nex-builder/Cargo.toml \
  --workspace --all-targets -- -D warnings
```

The normal tests do not use the network. To rebuild three real vendor archives
and compare them with retained manifest hashes, set
`NEX_SOURCE_NETWORK_TEST` to an empty cache directory and run:

```text
cargo test --manifest-path src/nex-builder/Cargo.toml \
  -p nex-source --test real_archives
```
