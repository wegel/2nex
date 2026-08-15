# Builder And Materializer

## Quality Gate

Use `scripts/check-builder-style.sh` when work touches the builder target
area. The script checks root style rules that code can measure: file length,
function length, module docs, glob imports, and inline tests in large modules.

The target area is:

```bash
src/cli/src/build
src/cli/src/system
src/cli/src/materializer
src/cli/src/outputs
src/cli/src/commands/build.rs
```

Full clippy can still fail outside that area. For EP005-style checks, filter
clippy diagnostics to those target paths and require `TARGET_WARNINGS=0`.

Evidence: during EP005, `scripts/check-builder-style.sh` passed, full clippy
still failed in non-target files, and the target-filtered clippy pass reported
`TARGET_WARNINGS=0`.

## Package And System Publishing

Package and system `--check` builds must not publish output, bundle, semantic,
or system refs when the manifest checksum is missing or stale unless
`--update-checksum` is also set.

The builder should run the reproducibility check first, then decide whether it
can publish. This lets `--check --update-checksum` refresh a checksum while
plain `--check` refuses to publish refs from unrecorded output.

Evidence: EP005 added package and system publish guards and tests named
`check_without_update_rejects_stale_*_checksum` and
`check_with_update_allows_stale_*_checksum`.

## Build Script Isolation

Host-mode build script setup must not mutate the parent process environment.
Build script environment variables should be passed to `Command` with
`env_clear()` and `envs(...)`.

Evidence: `build::script::script_tests::host_build_env_does_not_clear_parent_process_env`
passed after the build launcher stopped clearing the process-global
environment.

## Materializer Runtime Closure

Resolving materialization must return hard errors for unresolved runtime
requirements. Do not let a root request succeed when a manifest is missing,
a dependency name is undeclared, an output or bundle lacks metadata, or a
declared runtime file cannot be found in the provider manifest.

Requested-only materialization still marks requested refs as roots, even when
dependency resolution is disabled.

Evidence: EP005 added tests for missing root manifests, undeclared dependency
names, missing output and bundle metadata, missing dependency file metadata,
and requested-only root marking.

Checksum-addressed files refs should use the current request as the self files
ref. Non-files output and bundle refs derive their self files ref from the
provider manifest checksum.

Evidence: resolver tests named `files_commit_uses_current_self_files_ref`,
`output_commit_derives_self_files_ref`, and
`output_commit_queues_derived_self_files_ref` passed during EP005.

## Capability Providers

`nex compute-deps` maps known graphics ABI paths to capability requirements
instead of pinning those paths to a concrete graphics package. Current mappings
are `/usr/lib/libEGL.so.1` to `graphics.egl`,
`/usr/lib/libGLESv1_CM.so.1` and `/usr/lib/libGLESv2.so.2` to
`graphics.gles`, and `/usr/lib/libgbm.so.1` to `graphics.gbm`.

Capability requirements still carry a concrete fallback. Package-level checks
and builds use the fallback because a single package manifest does not know
which provider an assembly will choose. System materialization uses the
assembly `providers:` map.

Provider bindings must name exact output or bundle refs whose selected outputs
declare the requested capability in `provides:`. `nex check` rejects a system
provider that points at a ref whose output metadata does not provide the
capability.

Use output `capability_files:` when a broad output provides several
capabilities but one assembly wants only one of them. Without a file filter,
binding Mesa as the `graphics.gbm` provider also copied Mesa EGL and GLES files
into Nvidia capsules. The Nvidia desktop assemblies bind `graphics.egl` and
`graphics.gles` to the Nvidia runtime bundle, then bind `graphics.gbm` to Mesa
with a Mesa file filter.

Generated output refreshes preserve `provides:` and `capability_files:`.

Evidence: EP008 added `ResolutionTarget`, system `providers:`, output
`provides:`, output `capability_files:`, provider-aware dependency closure,
and capability provider tests. `cargo test --manifest-path src/cli/Cargo.toml`
passed with 141 unit tests and 3 blob ref tests. Checked-out
`desktop-vwl`, `desktop-vwl-nvidia-580`, and `desktop-vwl-nvidia-current`
systems showed Mesa libraries in the base `vwl` capsule, Nvidia EGL and GLES
libraries in Nvidia capsules, and Mesa GBM in Nvidia capsules.

## Capsule Flattening

Capsule flattening must include transitive `self` runtime libraries. A runtime
file that is provided by the same package can itself name additional `needs`,
and the flattener must walk those needs before declaring the capsule complete.

Evidence:
`materializer::flatten::flatten_runtime_tests::flatten_fails_when_transitive_self_library_is_missing`
passed after the flattener walked transitive self libraries.

Capsule flattening and file-level checkout must use the same relative symlink
safety rule. Resolve relative symlink targets lexically under the package or
checkout root, reject `..` escapes with `InvalidData`, and cap relative symlink
chains at 40 hops.

Evidence: `src/cli/src/materializer/relative_symlink.rs` provides the shared
helper. `checkout_store.rs` uses it for file-level checkout, and
`flatten_export.rs` uses it while exporting chained targets from files commits.
Tests named `flatten_library_exports_chained_relative_symlink_targets` and
`flatten_library_rejects_relative_symlink_target_that_escapes_package` passed.

## Nex-Mode Materialization

`.nex-app-root` can contain more than one root output for the same package
capsule. Nex checkout can revisit an existing package directory and must append
new root outputs instead of skipping the package directory entirely.

Evidence: `materializer::checkout::checkout_tests::write_root_commits_appends_new_roots`
and `system::nex_db::nex_db_tests::package_root_commits_reads_all_root_outputs`
passed during EP005.

Staged Nex-mode symlink forests must separate the physical package directory
from the logical target path. The code should scan files under the scratch
physical root, but public symlink targets should point at the final logical
`/nex/pkg` path.

Evidence:
`materializer::checkout::checkout_tests::symlink_forest_points_staged_links_at_logical_packages`
passed after the split.

System Nex materialization must reject malformed package refs. A bad ref in a
system manifest should become an `InvalidInput` error, not a skipped package.

Evidence: `system::nex::nex_tests::invalid_package_ref_stops_nex_materialization`
passed during EP005.

## Output Checksums And Generated Outputs

Output checksums describe the full output tree shape. They include directories,
regular file modes and bytes, symlink modes, and symlink targets. Unsupported
special file types such as FIFOs are rejected with `InvalidData` instead of
being ignored.

Evidence:
`outputs::checksum::checksum_tests::output_checksum_changes_when_symlink_target_changes`,
`output_checksum_distinguishes_file_from_symlink`, and
`output_checksum_rejects_fifo_entries` passed during EP005.

Generated-output categorization should return errors instead of panicking when
the output tree walk fails or a path is not valid UTF-8.

Evidence:
`outputs::categories::categories_tests::generated_outputs_reject_non_utf8_paths`
passed during EP005.

## Package Names And Namespaces

Materializer code should normalize package namespace paths to one `pkg/...`
prefix. Avoid deriving paths that become `pkg/pkg/...`.

Evidence: EP005 added namespace normalization in the materializer resolver and
flattener after a review found doubled `pkg` paths.

`nex build --hydrate-dependencies` should name hydrated dependencies from the
parsed package slug or keep the original direct dependency name. Do not use a
raw segment such as `commit.split('/').nth(1)`, which returns `pkg` for normal
package refs.

Evidence:
`build::orchestration::hydrate::hydrate_tests::hydrated_dependency_names_use_package_slug_not_pkg_segment`,
`hydrated_dependency_names_preserve_direct_names`, and
`formatted_hydrated_dependencies_use_meaningful_names` passed during EP005.

Resolve a semantic package ref from the `package.slug` declared in YAML, not
from a filename or filename suffix. Filenames can differ from slugs, and one
valid package filename can end in another package's slug. For example,
`2nex-utilities.yaml` declares `nex-utilities`, while `at-spi2-atk.yaml` must
not match `atk`. Reject two manifests that declare the same slug.

Evidence: EP012's GTK dependency refresh first reported both `atk.yaml` and
`at-spi2-atk.yaml` for the `atk` ref. The declared-slug-only lookup passed all
190 CLI tests, including both filename cases and duplicate declarations.

## Display Hashes

Use `utils::short_hash` for display prefixes of untrusted or variable-length
hash strings. Do not slice with `[..12]`, which can panic on short strings.

Evidence: EP005 replaced short display slices in builder/materializer code and
added `utils::tests::short_hash_handles_short_values`.
