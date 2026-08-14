# Make graphics runtime providers explicit

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex should treat Mesa and Nvidia graphics runtime files as normal package
outputs chosen by an assembly manifest. The builder should still generate
runtime dependencies automatically from ELF files. When `nex compute-deps`
finds a known graphics ABI file such as `/usr/lib/libEGL.so.1`, it should write
a capability need such as `graphics.egl` instead of hardcoding Mesa into the
package manifest. An assembly such as `desktop-vwl-nvidia-580` must bind that
need to an exact output ref such as
`x86_64/pkg/libs/graphics/nvidia-580/580.159.04/outputs/graphics-runtime`.

After this plan, a reader can inspect the manifests and see which package
output supplies each graphics role. The builder must never choose a provider by
searching the registry, sorting candidates, or using a default hidden in Rust
code. If an assembly omits a required provider binding, the build must fail
before it creates a system ref.

This plan replaces the current Nvidia-specific materializer path matching with
a generic capability binding rule. A capability is a named runtime role such as
`graphics.egl`. The capability does not create files by itself. It points to a
normal package output that already lists its files, needs, checksum, and store
ref.

## Progress

- [x] (2026-07-03 00:00Z) Ran the worktree pre-task. The tracked worktree was
  clean; only ignored Ralph files were dirty, so there was no pre-task commit.
- [x] (2026-07-03 00:00Z) Listed `.agents/knowledge/` and read the builder,
  package manifest, system assembly, and reproducibility notes.
- [x] (2026-07-03 00:00Z) Read the current graphics manifests, materializer resolver, and system
  assembly path.
- [x] (2026-07-03 00:00Z) Added manifest schema and formatter support for
  output-provided capabilities, generated capability resolutions, and
  assembly-selected provider refs. Focused formatter tests passed.
- [x] (2026-07-03 00:00Z) Taught `nex compute-deps` to convert known graphics
  ABI paths into generated capability targets with concrete package fallbacks.
- [x] (2026-07-03 00:00Z) Added provider validation to `nex check` for invalid
  provider refs, provider outputs that do not declare a capability, and system
  package graphs with unbound capabilities.
- [x] (2026-07-03 00:00Z) Convert Mesa, Nvidia 580, Nvidia current, wlroots, and vwl manifests to
  use graphics capabilities where they currently force Mesa or rely on
  Nvidia-specific materializer code.
- [x] (2026-07-03 00:00Z) Replace Nvidia-specific capsule injection with generic provider-output
  materialization.
- [x] (2026-07-03 00:00Z) Build and inspect Mesa, Nvidia 580, and Nvidia current desktop assemblies.
- [x] (2026-07-03 00:00Z) Committed the checked implementation as
  `00db7a8 cli: bind graphics runtime providers`.

## Surprises & Discoveries

- Observation: Package-level build and check contexts do not have an assembly
  provider map, so generated capability resolution entries need a concrete
  fallback dependency name.
  Evidence: `src/cli/src/deps/mod.rs` resolves package runtime closure from
  package manifests alone, while `src/cli/src/system/build.rs` has the system
  manifest and can pass `providers:`.

- Observation: Formatter support for `providers:`, output `provides:`, and
  nested capability resolution passed focused tests.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml
  manifest::format_tests` passed with three tests.

- Observation: `compute-deps` now maps EGL, GLES, and GBM ABI paths to graphics
  capabilities, and keeps `/usr/lib/libvulkan.so.1` as a normal loader
  dependency.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml compute_deps_tests`
  passed with three tests.

- Observation: System dependency closure uses an assembly provider binding for
  a capability and fails when the capability is unbound, while package
  dependency closure can use the generated fallback.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml deps::tests`
  passed, including `system_closure_uses_bound_provider_for_capability`,
  `system_closure_rejects_unbound_capability`, and
  `package_closure_uses_capability_fallback_without_system_providers`.

- Observation: `nex check` validates provider refs before assembly builds.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml commands::check`
  passed, including tests for files refs, missing `provides`, and a valid
  provider output.

- Observation: Nvidia external-platform JSON files name stable `.so.1`
  libraries, so the Nvidia package must install matching symlinks next to the
  versioned libraries.
  Evidence: `desktop-vwl-nvidia-580` vwl capsule inspection shows
  `libnvidia-egl-gbm.so.1 -> libnvidia-egl-gbm.so.1.1.3`,
  `libnvidia-egl-wayland.so.1 -> libnvidia-egl-wayland.so.1.1.20`,
  `libnvidia-egl-xcb.so.1 -> libnvidia-egl-xcb.so.1.0.5`, and
  `libnvidia-egl-xlib.so.1 -> libnvidia-egl-xlib.so.1.0.5`.

- Observation: `write_auto_outputs_to_manifest` must preserve output
  `provides:` and `capability_files:` when generated outputs refresh.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml` passed tests named
  `generated_outputs_preserve_existing_output_provides` and
  `generated_outputs_preserve_existing_capability_files`.

- Observation: A broad output that provides several graphics capabilities can
  still need per-capability file filters.
  Evidence: Nvidia assemblies bind EGL and GLES to Nvidia but GBM to Mesa.
  Without Mesa `capability_files.graphics.gbm`, the Mesa provider output also
  copied Mesa `libEGL.so.1` and `libGLESv2.so.2` into vwl capsules. The final
  smoke inspection shows Nvidia EGL/GLES and Mesa GBM together.

## Decision Log

- Decision: Model graphics providers as package outputs with named
  capabilities, not as package search results.
  Rationale: Nex requires explicit inputs and reproducible outputs. A hidden
  solver would let the builder choose different providers when the manifest
  tree changes. A provider binding in the assembly is a visible input that the
  system checksum covers.
  Date/Author: 2026-07-03 / Carlos

- Decision: Keep provider files in package `outputs` and make capabilities
  refer to those outputs.
  Rationale: The package checksum already covers output files. A separate
  provider-file list outside `outputs` could drift from the package contents
  and bypass the normal manifest audit path.
  Date/Author: 2026-07-03 / Carlos

- Decision: Keep build-time dependencies concrete.
  Rationale: Nex manifests list compilers, headers, and build tools explicitly.
  Runtime capabilities solve only files that installed programs open at
  runtime or load through GLVND, GBM, EGL, or Vulkan.
  Date/Author: 2026-07-03 / Carlos

- Decision: Let `nex compute-deps` generate graphics capability needs from a
  checked-in ABI path map.
  Rationale: Nex currently generates runtime dependencies automatically by
  scanning ELF `DT_NEEDED` entries. Forcing humans to hand-edit every EGL,
  GLES, and GBM path would regress that rule and would miss packages such as
  Chromium, Qt, looking-glass, wl-mirror, xwayland, and wlroots. A small static
  map from known ABI file paths to capability names keeps generation
  deterministic while still leaving the assembly to choose the exact provider
  output.
  Date/Author: 2026-07-03 / Carlos

- Decision: Let child assemblies override parent `providers:` entries by
  capability name.
  Rationale: The base `desktop-vwl` assembly can bind Mesa providers, while
  `desktop-vwl-nvidia-580` and `desktop-vwl-nvidia-current` can inherit the
  package set and replace only the provider refs that select the graphics
  runtime. This mirrors existing named package and dependency inheritance.
  Date/Author: 2026-07-03 / Ralph

- Decision: Keep `graphics.gbm` bound to Mesa for Nvidia desktop assemblies
  while binding EGL and GLES to the Nvidia runtime bundle.
  Rationale: The Nvidia runtime package supplies `nvidia-drm_gbm.so` and EGL
  backend JSON, but it does not ship the generic GBM ABI file
  `/usr/lib/libgbm.so.1`. Mesa should provide the generic GBM ABI and Nvidia
  should provide the driver backend.
  Date/Author: 2026-07-03 / Ralph

- Decision: Add `capability_files` under outputs instead of splitting the Mesa
  output solely for GBM.
  Rationale: Existing package output names can remain stable, and the
  capability binding can still copy only the ABI files that a selected
  capability needs. The package checksum and manifest still cover the file
  list.
  Date/Author: 2026-07-03 / Ralph

## Outcomes & Retrospective

Implemented and checked. The builder now supports generated graphics runtime
capabilities, assembly `providers:` bindings, provider-aware dependency
closure, provider validation in `nex check`, inherited provider overrides, and
generic provider file flattening. The old Nvidia-specific materializer path
was removed.

Manifest changes make Mesa provide `graphics.egl`, `graphics.gles`, and
`graphics.gbm`; Nvidia 580 and Nvidia current provide `graphics.egl` and
`graphics.gles`; wlroots now asks for graphics capabilities with Mesa
fallbacks; desktop assemblies bind the capabilities explicitly.

Checks passed:

- `./src/cli/target/debug/nex build pkg/core/nex/nex.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs`
- `./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force`
- `./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl-nvidia-580.yaml --verbose --check --update-checksum --force`
- `./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl-nvidia-current.yaml --verbose --check --update-checksum --force`
- fresh zub checkout inspection of `desktop-vwl`, `desktop-vwl-nvidia-580`,
  and `desktop-vwl-nvidia-current` vwl capsules
- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`
- `scripts/check-builder-style.sh`
- `./src/cli/target/debug/nex check pkg/core/nex/nex.yaml pkg/libs/graphics/mesa.yaml pkg/libs/graphics/nvidia-580.yaml pkg/libs/graphics/nvidia-current.yaml pkg/libs/wayland/wlroots.yaml asm/desktop-vwl/desktop-vwl.yaml asm/desktop-vwl/desktop-vwl-nvidia-580.yaml asm/desktop-vwl/desktop-vwl-nvidia-current.yaml`
- `cargo test --manifest-path src/cli/Cargo.toml`
- `git diff --check`

## Context and Orientation

Read these files before editing:

- `PHILOSOPHY.md`
- `RUST_CODE_STYLE.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/knowledge/builder-materializer.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/kernel-and-boot.md`

Current files that matter:

- `src/cli/src/manifest/types.rs` defines package and system manifest structs.
- `src/cli/src/manifest/format.rs` formats manifests.
- `src/cli/src/commands/compute_deps.rs` scans package outputs for ELF
  `DT_NEEDED` entries and writes generated file `needs` plus the package
  `resolution` map.
- `src/cli/src/commands/check.rs` and nearby manifest check code validate
  manifests.
- `src/cli/src/materializer/resolver.rs`,
  `src/cli/src/materializer/resolver_entries.rs`,
  `src/cli/src/materializer/resolver_metadata.rs`, and
  `src/cli/src/materializer/flatten.rs` resolve runtime files and build
  capsules.
- `src/cli/src/materializer/graphics_providers.rs` contains the current
  Nvidia-specific workaround. This plan should replace that special case with
  a generic provider-output path or shrink it to a compatibility shim that no
  production assembly needs.
- `pkg/libs/graphics/mesa.yaml`,
  `pkg/libs/graphics/nvidia-580.yaml`, and
  `pkg/libs/graphics/nvidia-current.yaml` contain the provider files.
- `pkg/libs/wayland/wlroots.yaml` and
  `pkg/desktop/compositor/vwl.yaml` currently pull concrete graphics runtime
  files.
- `asm/desktop-vwl/desktop-vwl.yaml`,
  `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`, and
  `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml` choose the desktop package
  set.

The live laptop proved the concrete bug. `vwl` inherited Mesa `libEGL.so.1`
and `libGLESv2.so.2` through `wlroots`. Nvidia JSON and private libraries were
present, but Mesa's loader failed on Nvidia hardware. Replacing those generic
loader files with Nvidia's files made `vwl` start.

## Plan of Work

1. Add schema fields that make provider choices explicit.

   Package output specs should gain a field like:

   ```yaml
   outputs:
     graphics-runtime:
       provides:
       - graphics.egl
       - graphics.gles
       - graphics.gbm
       - graphics.vulkan
       files:
       - path: /usr/lib/libEGL.so.1
   ```

   System manifests should gain a field like:

   ```yaml
   providers:
     graphics.egl: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
     graphics.gles: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
     graphics.gbm: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
     graphics.vulkan: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
   ```

   The exact field names may change during implementation if another name fits
   existing schema code better. The invariant must not change: every value must
   be an exact output or bundle ref from a manifest in the assembly input set.

2. Let `nex compute-deps` generate package runtime metadata that requires a
   capability without naming a concrete package.

   The current `resolution` map points an installed path at a dependency name
   or at `self`. Extend it so a path can point at a capability:

   ```yaml
   resolution:
     /usr/lib/libEGL.so.1:
       capability: graphics.egl
       fallback: mesa
     /usr/lib/libGLESv2.so.2:
       capability: graphics.gles
       fallback: mesa
     /usr/lib/libgbm.so.1:
       capability: graphics.gbm
       fallback: mesa
   ```

   Keep the current string format working:

   ```yaml
   resolution:
     /usr/lib/libc.so.6: glibc
     /usr/lib/libvwl.so: self
   ```

   `nex compute-deps` should use a small checked-in map from ABI paths to
   capability names. It must make the same output from the same package bytes
   and manifest inputs. Start with paths proved by the desktop work:

   ```text
   /usr/lib/libEGL.so.1 -> graphics.egl
   /usr/lib/libGLESv1_CM.so.1 -> graphics.gles
   /usr/lib/libGLESv2.so.2 -> graphics.gles
   /usr/lib/libgbm.so.1 -> graphics.gbm
   ```

   Treat Vulkan carefully. `/usr/lib/libvulkan.so.1` is the Vulkan loader, not
   the GPU provider. If this plan adds a Vulkan capability, name the provider
   role separately from the loader, such as `graphics.vulkan.icd`, and add a
   test that proves `libvulkan.so.1` still resolves to the normal
   `vulkan-loader` package.

   `nex compute-deps` must not select the assembly provider ref. It may write
   the capability and the concrete build dependency that satisfied the ABI
   during the package build. That concrete dependency is a fallback for
   package-level checks and package build roots where no assembly exists. A
   system assembly must still bind the capability to an exact output or bundle
   ref instead of silently using the fallback.

   Convert existing concrete generated entries for known graphics paths during
   migration. If a package truly requires Mesa rather than the generic ABI,
   this plan must add an explicit schema escape hatch before preserving that
   concrete entry.

3. Teach manifest checks to reject unsafe provider states.

   `nex check` must fail when:

   - A package resolution entry names a capability but the package schema
     cannot parse it.
   - A system package graph needs a capability that the system manifest does
     not bind. The generated fallback must not hide this missing assembly
     input.
   - A system binds a capability to a ref that does not parse as an output or
     bundle ref.
   - The bound output or bundle does not declare `provides` for that
     capability.
   - The system binds one capability to two different refs after inheritance.
   - A package still names Mesa directly for generic EGL, GLES, or GBM paths
     when `compute-deps` would generate a capability for that path.

   The last check can start narrowly with the packages changed in this plan:
   `wlroots` and `vwl`. Do not block packages that truly require Mesa, such as
   Mesa tools or driver tests.

4. Change the resolver and materializer.

   When a file need resolves through a capability, the resolver must use the
   exact ref from the system provider map. It must verify that the selected
   output or bundle claims the capability. The materializer should flatten the
   selected provider output into the capsule just as it flattens normal runtime
   refs.

   The provider output should replace existing files in a capsule when those
   files represent the selected capability. This preserves the live fix where
   Nvidia `libEGL.so.1`, `libGLESv2.so.2`, and `libGLdispatch.so.0` replaced
   Mesa files inside the `vwl` capsule.

   Do not add a registry search. Do not choose "the first provider." Do not
   choose Mesa when the assembly omits a binding. Fail with an actionable error
   that names the package, needed capability, and assembly manifest.

5. Change the graphics manifests.

   Add a normal graphics runtime output or bundle to:

   - `pkg/libs/graphics/mesa.yaml`
   - `pkg/libs/graphics/nvidia-580.yaml`
   - `pkg/libs/graphics/nvidia-current.yaml`

   Prefer an output or bundle name such as `graphics-runtime` when the files
   should move together. Keep normal outputs such as `bin`, `lib`, `module`,
   and `misc` when they still serve other package or assembly uses.

   The provider output must include all files needed for the role, including
   files that ELF cannot see:

   - GLVND dispatch libraries such as `libEGL.so.*`, `libGLESv2.so.*`,
     `libGLdispatch.so.*`, `libOpenGL.so.*`, and `libGLX.so.*` when the
     provider owns them.
   - Vendor libraries such as `libEGL_nvidia.so.*`.
   - GBM backends such as `usr/lib/gbm/nvidia-drm_gbm.so`.
   - EGL external platform JSON files.
   - GLVND vendor JSON files.
   - Vulkan ICD and implicit layer JSON files.

6. Change the desktop and graphics users.

   Convert `wlroots` and `vwl` away from concrete Mesa runtime resolution for
   generic EGL, GLES, and GBM paths. `nex compute-deps` should produce
   capability resolution for those paths during rebuilds. Apply the same rule
   to other packages touched by this plan, such as Chromium, looking-glass,
   wl-mirror, xwayland, and Qt, only after the generator and tests prove the
   behavior.

   The base `desktop-vwl` assembly should bind graphics capabilities to Mesa.
   The Nvidia assemblies should bind graphics capabilities to the matching
   Nvidia provider package.

7. Remove the current special case once tests cover the generic path.

   `src/cli/src/materializer/graphics_providers.rs` currently knows path
   prefixes such as `/usr/lib/libEGL.so`. Remove that hardcoded provider list
   from the production path when generic capability resolution passes the
   assembly checks. If a helper file remains, it should not mention Nvidia
   package names or graphics path prefixes.

## Concrete Steps

Use the repository root as the working directory.

1. Confirm the tree and read knowledge:

   ```bash
   git status --short --untracked-files=all
   ls .agents/knowledge
   rg -n "graphics|provider|capability|runtime|reproduc" \
     .agents/knowledge PHILOSOPHY.md
   ```

2. Add schema fields and parser tests:

   ```bash
   cargo test --manifest-path src/cli/Cargo.toml manifest
   ```

3. Add formatter support and tests:

   ```bash
   cargo test --manifest-path src/cli/Cargo.toml manifest::format
   ```

4. Add provider validation tests:

   ```bash
   cargo test --manifest-path src/cli/Cargo.toml check
   ```

5. Add `compute-deps` tests:

   ```bash
   cargo test --manifest-path src/cli/Cargo.toml compute_deps
   ```

   Test that the graphics ABI map changes generated resolution entries for
   EGL, GLES, and GBM into capability entries. Test that `libvulkan.so.1` still
   resolves to `vulkan-loader` unless this plan adds and documents a separate
   Vulkan provider role.

6. Add resolver and materializer tests:

   ```bash
   cargo test --manifest-path src/cli/Cargo.toml materializer::resolver
   cargo test --manifest-path src/cli/Cargo.toml materializer::flatten
   ```

7. Convert manifests and run checks:

   ```bash
   ./src/cli/target/debug/nex check \
     pkg/libs/graphics/mesa.yaml \
     pkg/libs/graphics/nvidia-580.yaml \
     pkg/libs/graphics/nvidia-current.yaml \
     pkg/libs/wayland/wlroots.yaml \
     pkg/desktop/compositor/vwl.yaml \
     asm/desktop-vwl/desktop-vwl.yaml \
     asm/desktop-vwl/desktop-vwl-nvidia-580.yaml \
     asm/desktop-vwl/desktop-vwl-nvidia-current.yaml
   ```

8. Run the full CLI tests:

   ```bash
   cargo test --manifest-path src/cli/Cargo.toml
   scripts/check-builder-style.sh
   cargo fmt --manifest-path src/cli/Cargo.toml -- --check
   ```

9. Rebuild `nex` because CLI source changes affect the packaged CLI:

   ```bash
   ./src/cli/target/debug/nex build pkg/core/nex/nex.yaml \
     --verbose --single --check --update-checksum --force \
     --compute-deps --record-profile --generate-outputs
   ```

10. Rebuild changed graphics packages if their output metadata changes:

   ```bash
   ./src/cli/target/debug/nex build pkg/libs/graphics/mesa.yaml \
     --verbose --single --check --update-checksum --force \
     --compute-deps --record-profile --generate-outputs
   ./src/cli/target/debug/nex build pkg/libs/graphics/nvidia-580.yaml \
     --verbose --single --check --update-checksum --force \
     --compute-deps --record-profile --generate-outputs
   ./src/cli/target/debug/nex build pkg/libs/graphics/nvidia-current.yaml \
     --verbose --single --check --update-checksum --force \
     --compute-deps --record-profile --generate-outputs
   ```

   If a manifest-only provider field changes no package output bytes, the build
   may still need regenerated output or bundle refs because metadata changed.
   Do not skip the strict package check unless the diff proves only comments or
   assembly files changed.

11. Rebuild desktop assemblies:

    ```bash
    ./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml \
      --verbose --check --update-checksum --force
    ./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl-nvidia-580.yaml \
      --verbose --check --update-checksum --force
    ./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl-nvidia-current.yaml \
      --verbose --check --update-checksum --force
    ```

12. Inspect built roots with disposable checkout directories:

    ```bash
    zub --repo .nex/repo checkout --copy \
      systems/desktop-vwl/0.0.1 .nex/tmp/desktop-vwl-graphics-smoke
    zub --repo .nex/repo checkout --copy \
      systems/desktop-vwl-nvidia-580/0.0.1 \
      .nex/tmp/desktop-vwl-nvidia-580-graphics-smoke
    zub --repo .nex/repo checkout --copy \
      systems/desktop-vwl-nvidia-current/0.0.1 \
      .nex/tmp/desktop-vwl-nvidia-current-graphics-smoke
    ```

    Resolve `/usr/bin/vwl` in each root and compare the provider files inside
    its capsule with the selected provider package. Use hashes for at least:

    - `usr/lib/libEGL.so.1`
    - `usr/lib/libGLESv2.so.2`
    - `usr/lib/libGLdispatch.so.0` when the provider owns it
    - `usr/lib/gbm/nvidia-drm_gbm.so` for Nvidia roots
    - `usr/share/glvnd/egl_vendor.d/10_nvidia.json` for Nvidia roots

13. Use the cleanup script for temporary roots:

    ```bash
    bash .agents/cleanup-workdirs.sh
    ```

14. Commit exact files only after all checks pass. Do not use `git add -A`.

## Validation and Acceptance

The plan is complete only when all of these statements are true:

- `cargo test --manifest-path src/cli/Cargo.toml` passes.
- `scripts/check-builder-style.sh` passes.
- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check` passes.
- `nex check` passes for every changed package and assembly manifest.
- A `compute-deps` test proves known EGL, GLES, and GBM ABI paths generate
  capability resolution entries.
- A `compute-deps` test proves `libvulkan.so.1` remains a normal loader
  dependency unless this plan adds a separate Vulkan provider capability.
- `pkg/core/nex/nex.yaml` builds reproducibly after CLI changes.
- Changed graphics packages build reproducibly or the agent records why the
  diff did not require a package rebuild.
- `asm/desktop-vwl/desktop-vwl.yaml` builds reproducibly with Mesa providers.
- `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml` builds reproducibly with Nvidia
  580 providers.
- `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml` builds reproducibly with
  Nvidia current providers.
- A checked-out Mesa desktop root shows the `vwl` capsule uses Mesa provider
  files for generic EGL, GLES, GBM, and Vulkan roles.
- A checked-out Nvidia 580 desktop root shows the `vwl` capsule uses Nvidia
  provider files for generic EGL, GLES, GBM, and Vulkan roles.
- A checked-out Nvidia current desktop root shows the `vwl` capsule uses
  Nvidia current provider files for generic EGL, GLES, GBM, and Vulkan roles.
- A negative test proves the builder fails when an assembly needs
  `graphics.egl` but does not bind it.
- A negative test proves the builder fails when an assembly binds
  `graphics.egl` to an output that does not declare that capability.
- Production code no longer chooses Nvidia provider files by package slug or
  hardcoded graphics file prefixes.

## Idempotence and Recovery

The zub store is a cache. If a build result looks confused, remove the store
only when the user has agreed to an audit rebuild, then rebuild from manifests.
During normal development, prefer targeted rebuilds for changed packages and
assemblies.

Use `.agents/cleanup-workdirs.sh` for disposable checkout and smoke roots under
`.nex/tmp`. Add new smoke paths to that script if this plan creates stable new
temporary names.

If a package checksum mismatch occurs, distinguish these cases before updating
the manifest:

- The package output bytes changed because this plan changed the manifest or
  source input. Rebuild with `--check --update-checksum` after the package
  reproduces.
- A read-only audit build found a mismatch for an unchanged manifest. Stop and
  investigate before updating the checksum.

If the capability schema needs a different YAML spelling, update this ExecPlan
before implementing the alternate spelling. Keep the invariant that an
assembly binds every abstract runtime need to an exact output or bundle ref.

## Artifacts and Notes

The current working design uses these examples:

```yaml
outputs:
  graphics-runtime:
    provides:
    - graphics.egl
    - graphics.gles
    - graphics.gbm
    - graphics.vulkan
    files:
    - path: /usr/lib/libEGL.so.1
```

```yaml
resolution:
  /usr/lib/libEGL.so.1:
    capability: graphics.egl
    fallback: mesa
```

```yaml
providers:
  graphics.egl: x86_64/pkg/libs/graphics/nvidia-580/580.159.04/outputs/graphics-runtime
```

These examples are schema sketches, not a license to add a hidden solver. The
implementation must keep the selected provider ref visible in the assembly
manifest and covered by the system checksum.
