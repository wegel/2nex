# Make The Builder Pleasant To Read

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

The Nex builder should read like professional Rust code, not just pass
`rustfmt`. A Rust developer who opens the builder should see small modules,
clear public APIs, explicit imports, typed domain choices, tests near the
behavior they prove, and comments that explain hard choices instead of
describing obvious lines.

After this plan, a maintainer can change package builds, system builds,
materialization, output commits, and reproducibility checks without first
untangling thousand-line files. The builder must still build packages and
systems exactly as before unless a named bug fix changes behavior and a test
proves the new behavior.

This plan is not complete until the target builder area satisfies both the
mechanical rules and the readability rules from `RUST_CODE_STYLE.md`. Do not
move this plan to `done/` while leaving known builder cleanup for a later plan.

## Progress

- [x] (2026-06-29 21:12Z) Created this ExecPlan after the human asked for a
  complete builder code-quality pass that satisfies the root Rust style guide.
- [x] (2026-06-29 19:46Z) Run the Ralph worktree pre-task and record what dirty or untracked files
  were committed, ignored, or left alone.
- [x] (2026-06-29 19:46Z) Read root `AGENTS.md`, `PHILOSOPHY.md`, `RUST_CODE_STYLE.md`,
  `MANIFESTS_CODE_STYLE.md`, `.agents/TESTING.md`, `.agents/PLANS.md`, and
  relevant notes under `.agents/knowledge/`.
- [x] (2026-06-29 22:18Z) Map the builder call graph and write the current module inventory in
  this plan.
- [x] (2026-06-29 19:52Z) Add a builder style audit check that fails on the mechanical style rules
  named in this plan.
- [x] (2026-06-29 20:08Z) Refactor package build code into small modules with clear APIs.
- [x] (2026-06-29 22:18Z) Refactor system build code into small modules with clear APIs.
- [x] (2026-06-29 22:18Z) Refactor materializer and output code enough that every target file
  satisfies the style guide.
- [x] (2026-06-29 22:18Z) Replace stale docs and add missing module and public API docs.
- [x] (2026-06-29 22:18Z) Move inline tests out of target modules when the style guide requires it.
- [x] (2026-06-29 22:18Z) Run automated style, compile, unit, and behavior checks.
- [x] (2026-06-29) Get and record a qualitative review from a separate agent against the
  root Rust quality bar.
- [x] (2026-06-29) Promote verified durable knowledge before moving this plan to `done/`.

## Surprises & Discoveries

- Observation: The first inspection found several builder files far over the
  root style cap.
  Evidence: `wc -l` reported 1478 lines for `src/cli/src/build/mod.rs`, 1338
  lines for `src/cli/src/build/orchestration.rs`, 960 lines for
  `src/cli/src/system/mod.rs`, 608 lines for
  `src/cli/src/materializer/flatten.rs`, 569 lines for
  `src/cli/src/materializer/checkout.rs`, and 479 lines for
  `src/cli/src/outputs/mod.rs`.

- Observation: The first inspection found a stale materializer module doc.
  Evidence: `src/cli/src/materializer/mod.rs` says dependencies are resolved
  by ELF scanning at materialization time, while
  `resolve_runtime_deps_precomputed` and the project philosophy say runtime
  deps come from precomputed manifest metadata.

- Observation: The first inspection found glob imports in builder modules.
  Evidence: `src/cli/src/build/mod.rs`, `src/cli/src/system/mod.rs`, and
  `src/cli/src/build/orchestration.rs` use `crate::...::*` imports.

- Observation: The package build entry file now delegates concrete jobs to
  dedicated modules, and the Rust tests still pass.
  Evidence: `wc -l src/cli/src/build/*.rs` reported `package.rs` at 287
  lines, `commits.rs` at 399 lines, `package_outputs.rs` at 160 lines, and
  the other extracted package modules below the 400-line cap. `cargo test
  --manifest-path src/cli/Cargo.toml` passed 69 unit tests and 3 blob ref
  tests after the split.

- Observation: The final mechanical style audit passes for the target builder
  area.
  Evidence: `scripts/check-builder-style.sh` printed `builder style check
  passed` after the build, materializer, output, and system refactors.

- Observation: Full CLI tests still pass after the target builder refactor.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml` passed 69 unit
  tests and 3 blob ref tests after the final graph and rootfs API changes.

- Observation: Full clippy still fails outside the target builder area.
  Evidence: `cargo clippy --manifest-path src/cli/Cargo.toml --all-targets
  -- -D warnings` no longer reported target builder files after target fixes,
  but still reported warnings in `tests/blob_ref_tests.rs`, `src/main.rs`,
  `src/commands/compute_deps.rs`, `src/commands/deployments.rs`,
  `src/commands/list.rs`, `src/commands/state.rs`,
  `src/manifest/update.rs`, `src/progress/display.rs`,
  `src/zig_vendor/mod.rs`, and `src/utils.rs`.

- Observation: `pkg/core/nex/nex-ld-shim.yaml` is a small strict builder smoke
  test that exercises the real package build path.
  Evidence: `./src/cli/target/debug/nex build pkg/core/nex/nex-ld-shim.yaml
  --verbose --single --check --update-checksum --force --compute-deps
  --record-profile --generate-outputs` passed, materialized runtime/build
  commits, verified the local source file, committed outputs and bundles,
  computed runtime deps, and passed the second build checksum.

- Observation: Reusable target builder code no longer exits the process for
  checksum failures.
  Evidence: `src/cli/src/system/build.rs` now returns
  `io::ErrorKind::InvalidData` for system checksum mismatches, and `rg
  "process::exit|std::process::exit"` across the target builder area found no
  process exits. `cargo test --manifest-path src/cli/Cargo.toml` passed after
  the change.

- Observation: The first qualitative review found five target-area quality
  findings.
  Evidence: `codex exec` reviewed `HEAD~2..HEAD` and reported unresolved
  materializer deps returning success, flattening hiding missing runtime files
  and symlink target export failures, graph planning warning instead of failing
  when a buildable dependency has no manifest, sparse public docs on
  `BuildOpts`, and weak tests for those behavior paths.

- Observation: The review fixes now make those failures explicit and tested.
  Evidence: `materializer::materializer_tests::unresolved_dependencies_return_actionable_error`,
  `materializer::materializer_tests::requested_only_materialize_does_not_require_manifest_db`,
  `materializer::flatten::flatten_runtime_tests::flatten_export_error_names_commit_and_path`,
  and
  `build::orchestration::orchestration_tests::missing_dependency_manifest_stops_graph_collection`
  passed. `BuildOpts` fields now have public docs.

- Observation: The second qualitative review found three remaining target-area
  quality findings.
  Evidence: `codex exec` reported that resolving materialization could still
  succeed when the requested root commit had no manifest, package
  reproducibility checks committed second-pass outputs before comparing the
  checksum, and public docs were still missing for `BuildArgs`,
  `BuildOpts::from_args`, and `MaterializeResult::new`.

- Observation: The second review fixes are now explicit and checked.
  Evidence:
  `materializer::materializer_tests::resolving_materialize_fails_when_root_manifest_is_missing`
  passed with an empty manifest DB and empty zub repo,
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_root_manifest_is_missing`
  passed, and the strict `nex-ld-shim` build printed the checksum match before
  second-pass output refs were committed.

- Observation: The third qualitative review found one remaining target-area
  materializer finding.
  Evidence: `codex exec` reported that capsule flattening could still skip a
  declared runtime file when a resolution entry named an undeclared dependency,
  when a dependency files ref could not be derived, or when self runtime files
  existed but the root package files ref could not be derived.

- Observation: Capsule flattening now rejects those incomplete runtime closures.
  Evidence:
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_self_files_commit_cannot_be_derived`,
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_dependency_files_commit_cannot_be_derived`,
  and
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_resolution_names_undeclared_dependency`
  passed. `cargo test --manifest-path src/cli/Cargo.toml` passed 78 unit tests
  and 3 blob ref tests after the fix. Commit `1034993` contains the change.

- Observation: The fourth qualitative review found three more materializer
  closure holes.
  Evidence: `codex exec` reported that flattening still skipped transitive self
  libraries, cached provider manifests only by dependency name, and treated
  missing bundle or output metadata as an empty dependency set.

- Observation: Those fourth-review holes now have production fixes and tests.
  Evidence:
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_transitive_self_library_is_missing`,
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_bundle_metadata_is_missing`,
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_output_metadata_is_missing`,
  `materializer::resolver_metadata::resolver_metadata_tests::reports_missing_bundle_metadata`,
  `materializer::resolver_metadata::resolver_metadata_tests::reports_missing_bundle_output_metadata`,
  and
  `materializer::resolver_metadata::resolver_metadata_tests::reports_missing_output_metadata`
  passed. `cargo test --manifest-path src/cli/Cargo.toml` passed 84 unit tests
  and 3 blob ref tests. The strict `nex-ld-shim` build passed and printed
  `Build is reproducible. Checksums match.` Commit `e9e0d19` contains the
  change.

- Observation: The fifth qualitative review found two remaining runtime-closure
  holes.
  Evidence: `codex exec` reported that checksum-addressed file requests could
  omit transitive needs when a provider manifest lacks a `FileEntry`, and that
  Nex package capsules with multiple root outputs only flattened dependencies
  for the first root output.

- Observation: Those fifth-review holes now have production fixes and tests.
  Evidence:
  `materializer::resolver::resolver_tests::files_commit_reports_requested_files_missing_manifest_metadata`,
  `materializer::flatten::flatten_runtime_tests::flatten_fails_when_dependency_file_metadata_is_missing`,
  and `system::nex_db::nex_db_tests::package_root_commits_reads_all_root_outputs`
  passed. `cargo test --manifest-path src/cli/Cargo.toml` passed 87 unit tests
  and 3 blob ref tests. The strict `nex-ld-shim` build passed and printed
  `Build is reproducible. Checksums match.` Commit `ce0402b` contains the
  change.

- Observation: The sixth qualitative review found two remaining materializer
  closure holes.
  Evidence: `codex exec` reported that resolver processing still dropped
  `self` runtime files for non-files output and bundle refs, and that Nex
  checkout skipped a package directory that already existed instead of
  appending newly requested root outputs into `.nex-app-root`.

- Observation: Those sixth-review holes now have production fixes and tests.
  Evidence:
  `materializer::resolver::resolver_tests::output_commit_derives_self_files_ref`,
  `materializer::resolver::resolver_tests::files_commit_uses_current_self_files_ref`,
  `materializer::resolver::resolver_tests::output_commit_queues_derived_self_files_ref`,
  and
  `materializer::checkout::checkout_tests::write_root_commits_appends_new_roots`
  passed under
  `cargo test --manifest-path src/cli/Cargo.toml materializer -- --nocapture`.
  On this host, the temp zub repository test skipped its store-backed part
  because the host cannot map uid 0, matching existing store-backed tests.
  `cargo test --manifest-path src/cli/Cargo.toml` passed 90 unit tests and 3
  blob ref tests. `scripts/check-builder-style.sh` passed. The strict
  `nex-ld-shim` build passed and printed `Build is reproducible. Checksums
  match.`

- Observation: Full clippy still fails outside the target builder area, but
  the target warning found during the final audit was fixed.
  Evidence: `cargo clippy --manifest-path src/cli/Cargo.toml --all-targets
  -- -D warnings` first reported
  `src/materializer/resolver_entries.rs` could derive `Default`. After
  replacing the manual impl with `#[derive(Default)]`, the same clippy command
  no longer reported target builder files and still failed only in older
  non-target files such as `tests/blob_ref_tests.rs`, `src/main.rs`,
  `src/commands/compute_deps.rs`, `src/commands/deployments.rs`,
  `src/commands/list.rs`, `src/commands/state.rs`,
  `src/manifest/update.rs`, `src/progress/display.rs`,
  `src/zig_vendor/mod.rs`, and `src/utils.rs`.

- Observation: The required separate qualitative review agent is currently
  unavailable, so this plan cannot be closed yet.
  Evidence: `codex exec` returned `You've hit your usage limit. Try again at
  10:04 PM` both before and at 2026-06-29 22:04Z. `claude -p` returned
  `Not logged in - Please run /login`. `opencode run` and
  `opencode --pure run` both returned `Unexpected error: no such column:
  name`.

- Observation: The separate qualitative review agent became available and
  found four more builder-area issues.
  Evidence: `codex exec` reported that package `--check` published first-pass
  output and bundle refs before the second checksum matched, Nex symlink
  forest creation ignored user package/env overrides, file-level checkout
  rejected valid absolute symlink entries before reading symlink metadata, and
  `materialize_bundle` always requested dependency resolution without any
  manifest DB paths.

- Observation: The seventh-review findings now have production fixes and
  tests.
  Evidence: package and system builds now commit semantic outputs only after a
  successful reproducibility check. `--update-checksum --check` also defers
  manifest checksum rewrites until after the second checksum matches. The
  strict `nex-ld-shim` build log showed `Build is reproducible. Checksums
  match.` before `Verifying and committing outputs to store branches`.

- Observation: Output checksums now cover the full tree shape and Unix modes.
  Evidence: `outputs::checksum` hashes directories, regular file modes, file
  bytes, symlink modes, and symlink targets. The focused checksum tests passed,
  and the strict `nex-ld-shim` build passed with the new checksum
  `64af181e1671a5877c8a3bd5e13e859c1ed78ddfa7b3e6fa319ebef3b08a4b7e`.

- Observation: File-level materializer checkout now copies relative symlink
  target chains.
  Evidence:
  `checkout_one_file_copies_chained_relative_symlink_targets`,
  `checkout_one_file_creates_nested_relative_symlink_target_parent`, and
  `checkout_one_file_preserves_absolute_symlink_entries` passed. The strict
  `nex-ld-shim` build also exercised the real materializer path after the
  change.

- Observation: The separate review after commit `5a783b1` found one remaining
  namespace normalization blocker.
  Evidence: `codex exec` reported that `resolver_refs.rs` and `flatten.rs`
  could build `x86_64/pkg/pkg/...` refs and `pkg/pkg/...yaml` paths for
  manifests that use the supported `namespace: pkg/...` form.

- Observation: Materializer namespace handling now uses one normalized
  `pkg/...` path rule.
  Evidence: `ManifestIndex` stores and looks up normalized namespace keys,
  resolver and flattener files-ref helpers use `x86_64/{namespace_path}/...`,
  and the focused namespace tests passed. `scripts/check-builder-style.sh`,
  `cargo test --manifest-path src/cli/Cargo.toml materializer -- --nocapture`,
  `cargo test --manifest-path src/cli/Cargo.toml`, the target-area clippy
  warning filter, `nex check` for `initramfs`, `desktop-vwl`, and
  `nex-ld-shim`, and the strict `nex-ld-shim` build all passed.

- Observation: The separate review after commit `e60a45f` found one remaining
  requested-only Nex materialization blocker.
  Evidence: `codex exec` reported that `requested_only_closure` added requested
  commits as ordinary commits instead of roots, while Nex checkout only
  materializes packages whose commits are marked as roots.

- Observation: Requested-only materialization now preserves requested roots.
  Evidence: `requested_only_closure` calls `RuntimeClosure::add_root`, and
  `requested_only_closure_marks_requests_as_roots_for_nex_checkout` passed.
  `scripts/check-builder-style.sh`, `cargo test --manifest-path
  src/cli/Cargo.toml materializer -- --nocapture`, `cargo test --manifest-path
  src/cli/Cargo.toml`, the target-area clippy warning filter, `nex check` for
  `initramfs`, `desktop-vwl`, and `nex-ld-shim`, and the strict
  `nex-ld-shim` build all passed.

- Observation: The separate review after commit `1603fbc` found one remaining
  build-script environment blocker.
  Evidence: `codex exec` reported that host builds cleared the parent process
  environment in `build_script_env`, so one build could remove `PATH` and repo
  settings for later builds in the same `nex` process.

- Observation: Build script isolation now clears only the child command
  environment.
  Evidence: `build_script_env` only constructs the requested environment map,
  `spawn_build_process` calls `Command::env_clear().envs(script_env)`, and
  `host_build_env_does_not_clear_parent_process_env` passed.
  `scripts/check-builder-style.sh`, `cargo fmt --manifest-path
  src/cli/Cargo.toml -- --check`, `cargo test --manifest-path
  src/cli/Cargo.toml`, the target-area clippy warning filter, `nex check` for
  `initramfs`, `desktop-vwl`, and `nex-ld-shim`, and the strict
  `nex-ld-shim` build all passed.

- Observation: The separate review after commit `a709a8b` found three
  remaining blockers.
  Evidence: `codex exec` reported that plain `--check` builds could publish
  refs after proving a checksum that differed from the manifest, and that
  target code sliced `package.checksum` and `manifest_ref` with fixed
  `[..12]` ranges even though those values come from YAML.

- Observation: Plain `--check` no longer publishes refs for stale or missing
  manifest checksums.
  Evidence: package and system publish guards now run after reproducibility
  checks and before semantic ref publication. The package and system
  `check_without_update_rejects_stale_*` tests passed, while the strict
  `nex-ld-shim` build with `--check --update-checksum` still published after
  the second checksum matched.

- Observation: Target builder code no longer slices manifest-supplied hash
  strings with fixed byte ranges.
  Evidence: `utils::short_hash` now handles short values, resolver and graph
  display paths use it, and `short_hash_handles_short_values` passed.
  `scripts/check-builder-style.sh`, `cargo fmt --manifest-path
  src/cli/Cargo.toml -- --check`, `cargo test --manifest-path
  src/cli/Cargo.toml`, the target-area clippy warning filter, `nex check` for
  `initramfs`, `desktop-vwl`, and `nex-ld-shim`, and the strict
  `nex-ld-shim` build all passed.
  `symlink_forest_uses_user_package_and_env_overrides`,
  `checkout_one_file_preserves_absolute_symlink_entries`, and
  `bundle_helper_config_does_not_require_manifest_db` passed under
  `cargo test --manifest-path src/cli/Cargo.toml materializer -- --nocapture`.
  `scripts/check-builder-style.sh`, `cargo test --manifest-path
  src/cli/Cargo.toml`, `./src/cli/target/debug/nex check
  pkg/core/kernel/initramfs.yaml`, `./src/cli/target/debug/nex check
  asm/desktop-vwl/desktop-vwl.yaml`, and the strict `nex-ld-shim` build all
  passed after the fixes.

- Observation: The eighth qualitative review found one staged-install symlink
  bug in the target materializer area.
  Evidence: `codex exec` reported that staged system installs still created
  environment symlinks pointing at the physical scratch package directory when
  `physical_root` differed from `target_dir`.

- Observation: Staged Nex symlink forest creation now separates physical
  scanning paths from logical link targets.
  Evidence:
  `materializer::checkout::checkout_tests::symlink_forest_points_staged_links_at_logical_packages`
  passed, `scripts/check-builder-style.sh` passed after
  `src/cli/src/materializer/symlink_forest.rs` was split out, and the strict
  `nex-ld-shim` build again printed `Build is reproducible. Checksums match.`
  before output and bundle publication.

- Observation: The ninth qualitative review found two remaining target-area
  blockers.
  Evidence: `codex exec` reported that `--show-dep-paths` walked outgoing graph
  edges even though the graph stores dependency-to-dependent edges, and that
  Nex system materialization returned `Ok(())` when a requested package ref
  failed to parse.

- Observation: Both ninth-review blockers now have production fixes and
  focused tests.
  Evidence:
  `build::orchestration::orchestration_tests::dependency_paths_follow_dependency_edges_back_from_root`
  passed after dependency path collection walked incoming edges from the root.
  `system::nex::nex_tests::invalid_package_ref_stops_nex_materialization`
  passed after malformed Nex package refs became `InvalidInput` errors.
  `scripts/check-builder-style.sh`, `cargo fmt --manifest-path
  src/cli/Cargo.toml -- --check`, `cargo test --manifest-path
  src/cli/Cargo.toml`, `./src/cli/target/debug/nex check
  pkg/core/kernel/initramfs.yaml`, `./src/cli/target/debug/nex check
  asm/desktop-vwl/desktop-vwl.yaml`, and the strict `nex-ld-shim` build passed
  after these fixes.

- Observation: The tenth qualitative review found one final target-area
  blocker.
  Evidence: `codex exec` reported that `nex build --hydrate-dependencies`
  named every hydrated dependency from `commit.split('/').nth(1)`, which is
  `pkg` for normal package refs such as
  `x86_64/pkg/libs/system/glibc/2.39/bundles/dev`.

- Observation: Hydrated dependency names now come from a parsed package ref or
  from the original direct dependency name.
  Evidence:
  `build::orchestration::hydrate::hydrate_tests::hydrated_dependency_names_use_package_slug_not_pkg_segment`,
  `build::orchestration::hydrate::hydrate_tests::hydrated_dependency_names_preserve_direct_names`,
  and
  `build::orchestration::hydrate::hydrate_tests::formatted_hydrated_dependencies_use_meaningful_names`
  passed. `scripts/check-builder-style.sh`, `cargo fmt --manifest-path
  src/cli/Cargo.toml -- --check`, `cargo test --manifest-path
  src/cli/Cargo.toml`, `./src/cli/target/debug/nex check
  pkg/core/kernel/initramfs.yaml`, `./src/cli/target/debug/nex check
  asm/desktop-vwl/desktop-vwl.yaml`, and the strict `nex-ld-shim` build passed
  after the fix.

- Observation: The eleventh qualitative review found only target-area clippy
  cleanup in the hydration fix.
  Evidence: `codex exec` reported `needless_question_mark` in
  `src/cli/src/build/orchestration/hydrate.rs` and a cloned-ref warning in
  `src/cli/src/build/orchestration/hydrate_tests.rs`. After the cleanup,
  `cargo clippy --manifest-path src/cli/Cargo.toml --all-targets -- -D warnings`
  failed only in known non-target files, while `scripts/check-builder-style.sh`,
  `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`, `cargo test
  --manifest-path src/cli/Cargo.toml`, the two manifest checks, and the strict
  `nex-ld-shim` build passed.

- Observation: The twelfth qualitative review found two remaining output-area
  blockers.
  Evidence: `codex exec` reported that output checksums ignored symlinks even
  though package commits and generated outputs include symlinks, and that
  generated-output categorization could panic on walk errors or non-UTF paths.

- Observation: Output checksums now include symlink targets, and generated
  output categorization now reports errors instead of panicking.
  Evidence:
  `outputs::checksum::checksum_tests::output_checksum_changes_when_symlink_target_changes`,
  `outputs::checksum::checksum_tests::output_checksum_distinguishes_file_from_symlink`,
  and
  `outputs::categories::categories_tests::generated_outputs_reject_non_utf8_paths`
  passed. `scripts/check-builder-style.sh`, `cargo fmt --manifest-path
  src/cli/Cargo.toml -- --check`, `cargo test --manifest-path
  src/cli/Cargo.toml`, target-filtered clippy with `TARGET_WARNINGS=0`, the
  manifest checks, and the strict `nex-ld-shim` build passed. The strict build
  updated the `nex-ld-shim` package checksum to
  `7e0c932312db5ccdb23f3f0727ba8ad667c77c16882ea30bcc6edd3cce973ab9` because
  the checksum algorithm now includes symlink metadata.

- Observation: The thirteenth qualitative review found three remaining
  target-area blockers.
  Evidence: `codex exec` reported that capsule flattening exported only one
  relative symlink hop and did not reject `..` escapes, output checksums
  silently ignored special files, and target-area code still used bare
  `.unwrap()` despite the root Rust style guide.

- Observation: The thirteenth-review blockers now have production fixes and
  focused tests.
  Evidence:
  `src/cli/src/materializer/relative_symlink.rs` now shares the safe relative
  symlink target rule between checkout and flattening,
  `src/cli/src/materializer/flatten_export.rs` exports relative symlink target
  chains from files commits, `src/cli/src/outputs/checksum.rs` rejects
  unsupported output file types with `InvalidData`, and `rg -n
  "\.unwrap\(\)" src/cli/src/build src/cli/src/system
  src/cli/src/materializer src/cli/src/outputs
  src/cli/src/commands/build.rs` returned no matches. The focused tests
  `materializer::flatten::flatten_export_tests::*` and
  `outputs::checksum::checksum_tests::output_checksum_rejects_fifo_entries`
  passed; on this host the store-backed flatten tests skipped the zub commit
  part because `commit_tree` cannot map uid 0.

## Decision Log

- Decision: Treat this as a builder-area quality pass, not a narrow
  file-splitting task.
  Rationale: The human asked not to discover later that cleanup remains. A
  professional result needs correct module boundaries, names, docs, tests,
  error paths, and behavior checks together.
  Date/Author: 2026-06-29 / Carlos

- Decision: Add or update an automated style audit for the target area before
  the refactor finishes.
  Rationale: `rustfmt` cannot catch file size, function size, missing module
  docs, glob imports, or inline tests in large modules. A check makes the final
  claim repeatable.
  Date/Author: 2026-06-29 / Carlos

- Decision: Use a separate qualitative review agent before plan completion.
  Rationale: A script can count lines and catch imports, but it cannot judge
  whether a module reads like a clear Rust chapter, whether names carry the
  right domain facts, or whether a helper split improves understanding.
  Date/Author: 2026-06-29 / human and Carlos

- Decision: Keep behavior changes out of this plan unless a refactor exposes a
  builder bug and a focused test proves the fix.
  Rationale: The main goal is readability and maintainability. Behavior drift
  without a test would make package and assembly reproducibility harder to
  trust.
  Date/Author: 2026-06-29 / Carlos

- Decision: Restore the `nex-ld-shim` timing profile after the strict build.
  Rationale: The strict build rewrote only `build.profile`, which records
  local timing rather than a durable builder behavior change. The command still
  proves the changed builder path, but a timing-only manifest diff does not
  belong in the builder refactor commit.
  Date/Author: 2026-06-29 / Carlos

- Decision: Return an error for system checksum mismatch instead of exiting
  from `system/build.rs`.
  Rationale: The command layer may choose how to present failures, but builder
  logic should be reusable and testable.
  Date/Author: 2026-06-29 / Carlos

- Decision: Make unresolved runtime dependencies and missing declared runtime
  files hard materializer errors.
  Rationale: A root or Nex capsule that lacks declared runtime files is not a
  successful build artifact. The caller needs the missing path, commit, and
  reason so the manifest can be fixed.
  Date/Author: 2026-06-29 / Carlos

- Decision: Make graph planning fail when a dependency needs a local build but
  no manifest exists.
  Rationale: The graph builder has the dependency ref and manifest search
  context. Returning an error there gives a better fix path than a later store
  checkout failure.
  Date/Author: 2026-06-29 / Carlos

- Decision: Compare the package second-build checksum before committing
  second-pass outputs and bundles.
  Rationale: A failed reproducibility check must not publish nondeterministic
  output refs under the current manifest hash.
  Date/Author: 2026-06-29 / Carlos

- Decision: Treat a missing root manifest as a materializer error when runtime
  dependency resolution is enabled.
  Rationale: Without the root manifest, the materializer cannot prove the
  runtime closure named by package metadata.
  Date/Author: 2026-06-29 / Carlos

- Decision: Make capsule flattening fail if declared runtime files cannot map
  to a concrete provider files commit.
  Rationale: A capsule that silently drops a declared runtime file is not a
  valid artifact. The flattener must give the package author a path, dependency
  name, and manifest context instead of returning a short success list.
  Date/Author: 2026-06-29 / Carlos

- Decision: Put materializer flattening errors in a small helper module.
  Rationale: `flatten.rs` was already near the style guide file-size cap, and
  named error helpers keep the main flattening path readable while preserving
  actionable messages.
  Date/Author: 2026-06-29 / Carlos

- Decision: Remove the flattener dependency-manifest cache instead of widening
  its key.
  Rationale: The cache sat on a correctness boundary and keyed by names that are
  local to one manifest. Looking up the provider manifest directly keeps the
  code simpler and avoids cross-manifest aliasing.
  Date/Author: 2026-06-29 / Carlos

- Decision: Keep resolver bundle/output metadata checks in
  `resolver_metadata.rs`.
  Rationale: `resolver.rs` was close to the 400-line cap. A small helper module
  makes the missing-metadata rule explicit without crowding the resolver loop.
  Date/Author: 2026-06-29 / Carlos

- Decision: Move resolver file-entry selection to `resolver_entries.rs`.
  Rationale: The missing-file-metadata fix pushed `resolver.rs` over the style
  guide cap. A dedicated module keeps checksum-file entry selection small and
  testable.
  Date/Author: 2026-06-29 / Carlos

- Decision: Store and process every root output commit in `.nex-app-root`.
  Rationale: A package capsule can be made from more than one root output. The
  flattener must read every root output's manifest metadata before it can prove
  the capsule contains the full runtime closure.
  Date/Author: 2026-06-29 / Carlos

- Decision: Make symlink forest relative path calculation lexical.
  Rationale: During staged system installs, the final logical `/nex/pkg` and
  `/nex/env` paths may not exist yet. The materializer still needs to write
  symlinks that are correct after the staged root moves into place.
  Date/Author: 2026-06-29 / Carlos

- Decision: Keep dependency graph edge direction as dependency-to-dependent and
  make path display traverse that model correctly.
  Rationale: The build order logic already relies on this graph shape.
  Reversing edges would risk working scheduling code, while incoming-edge path
  traversal fixes only the display behavior.
  Date/Author: 2026-06-29 / Carlos

- Decision: Treat malformed Nex package refs as hard system materialization
  errors.
  Rationale: A system manifest explicitly requests each package commit. A
  malformed ref should stop the build instead of silently omitting a package
  from the root filesystem.
  Date/Author: 2026-06-29 / Carlos

- Decision: Use parsed package refs for hydrated dependency names.
  Rationale: Package refs already encode the slug location, while fixed
  `split('/')` indexes confuse the `pkg` namespace marker with the package
  name. Preserving direct dependency names keeps hand-written resolution names
  stable.
  Date/Author: 2026-06-29 / Carlos

- Decision: Include symlink metadata in package output checksums.
  Rationale: A build artifact can change by changing only a symlink target.
  Reproducibility checks must catch that change just like regular file content
  changes.
  Date/Author: 2026-06-29 / Carlos

- Decision: Return errors from generated-output categorization.
  Rationale: The package build flow already returns `io::Result`, so invalid
  output paths should name the bad path and stop the build instead of panicking.
  Date/Author: 2026-06-29 / Carlos

- Decision: Share relative symlink safety rules between file-level checkout
  and capsule flattening.
  Rationale: Both code paths copy files from store commits into a partial
  package tree. They must reject path escapes and follow safe relative chains
  the same way.
  Date/Author: 2026-06-29 / Carlos

- Decision: Reject unsupported special files in output checksum walks.
  Rationale: Silently ignoring a FIFO, socket, or device node lets a package
  change the output tree without changing its checksum.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

EP005 completed the builder readability and quality pass. The target builder
area is split into small modules, passes `scripts/check-builder-style.sh`, has
no bare `.unwrap()` calls, and has focused tests for the behavior bugs found by
the review loop.

The final independent `codex exec` review returned `no blockers` after checking
the root Rust style guide, the three named final blockers, the target style
script, full CLI tests, target clippy output, and the main package, system,
materializer, and output checksum paths.

The work also fixed real behavior: materializer closure failures now stop
builds, package and system `--check` builds no longer publish stale refs,
capsule flattening follows safe relative symlink chains, output checksums cover
full tree shape and reject unsupported file types, and target code follows the
root Rust style better.

Durable notes from this plan were promoted into
`.agents/knowledge/builder-materializer.md`, `.agents/knowledge/cli-testing.md`,
and `.agents/knowledge/reproducibility.md`.

## Context and Orientation

Read these files first:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `RUST_CODE_STYLE.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/PLANS.md`

List `.agents/knowledge/` before work starts. Read at least these knowledge
notes because they touch the builder, tests, or reproducibility:

- `.agents/knowledge/cli-testing.md`
- `.agents/knowledge/reproducibility.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/agent-workflow.md`

Target builder area:

- `src/cli/src/build/`
- `src/cli/src/build/mod.rs`
- `src/cli/src/system/mod.rs`
- `src/cli/src/materializer/`
- `src/cli/src/outputs/mod.rs`
- `src/cli/src/commands/build.rs`
- Any new modules or tests that replace code from those files.

Use `semeja search` first when looking for code by purpose or symbol. Use
`rg` when the work needs exhaustive literal matches.

The root style guide is the authority for this plan. In particular, the target
area must satisfy these rules:

- Every module has a one-sentence `//!` module doc.
- Every public type, function, method, and public field has useful `///` docs.
- No target file has more than 400 lines including tests.
- Most target files sit near 150 to 300 lines when the concern allows that.
- No target function has more than 60 lines.
- Most target functions sit near 15 to 30 lines when the concern allows that.
- No target module uses glob imports.
- Imports are grouped as `std`, external crates, then internal modules, and
  each group is sorted.
- Public types come before public APIs, and private helpers come later.
- Tests live in separate test files unless the module is under 100 lines.
- Boolean functions read like questions.
- Repeated patterns that appear three or more times become helpers.
- Internal builder code uses specific error types where callers need to react
  to the error. Top-level command code may convert those errors for CLI output.
- Inline comments explain non-obvious domain choices. They do not narrate
  obvious code.

Professional readability also means:

- A reader can describe what a module owns after reading its module doc and
  public type names.
- A reader can understand a public function's flow without scrolling through a
  long block.
- Names say the domain fact directly, such as `BuildRoot`, `OutputChecksum`,
  `ReproducibilityCheck`, or `SystemRootCommit`, rather than passing many raw
  strings and booleans.
- The builder separates package manifest loading, build root preparation,
  source staging, build script execution, output splitting, store commits,
  runtime dependency metadata, and reproducibility checks.
- The system builder separates dependency resolution, package materialization,
  overlay application, system script execution, checksum handling, and rootfs
  commit.
- The materializer docs and code agree that runtime dependencies use
  precomputed manifest metadata, not install-time ELF scanning.

## Plan of Work

Start with a map, not edits. Inventory every public function in the target
area, the files that call it, and the behavior it owns. Record the map in
`Artifacts and Notes` or link to a temporary scratch note.

Add an automated style audit that checks the target builder area. The audit can
be a shell script, Rust test, or small Rust utility, but it must run from the
repo root and fail on these facts:

- A target Rust file has more than 400 lines.
- A target function has more than 60 lines.
- A target file has no module doc, unless it is a test file.
- A target file uses a glob import.
- A large target module keeps inline `#[cfg(test)] mod tests`.

The audit may warn rather than fail on import sorting and public doc gaps if
the implementation cost is high, but a separate qualitative review agent must
still inspect those items before this plan completes.

Refactor in small, buildable commits. Prefer modules with one concrete job:

- `build/env.rs` for loading build environments and expanding templates.
- `build/rootfs.rs` for build root setup and dependency layering.
- `build/inputs.rs` for fetching and staging declared sources.
- `build/script.rs` for running build scripts under the selected environment.
- `build/package.rs` for package build flow.
- `build/reproducibility.rs` for two-pass rebuild checks.
- `build/checksum.rs` or `outputs/checksum.rs` for output checksums if that
  ownership fits better after inspection.
- `system/build.rs` or `system/manifest.rs` for the system build flow.
- `system/materialize.rs`, `system/overlays.rs`, and `system/commit.rs` for
  system root filesystem work if those names fit the existing code.
- `outputs/bundles.rs`, `outputs/files.rs`, and `outputs/categories.rs` if
  output code remains too large.

Use the actual call graph and tests to decide the final module names. Do not
force the names above if the code points to better names.

As code moves, keep public APIs narrow. Replace repeated long argument lists
with focused structs only when the struct has a clear domain name and each
field belongs together. Avoid creating a generic "context" bag.

Add tests while extracting behavior. At minimum, keep or add tests for:

- Manifest kind dispatch in `nex build`.
- Build directory naming for package and system builds.
- Build environment template expansion.
- Reproducibility check control flow, including the second clean build root.
- Output checksum calculation stability.
- Output categorization behavior.
- Runtime dependency closure behavior for precomputed deps.
- System root metadata creation.
- Any bug fixed during the refactor.

When a function cannot be tested without a real zub store, package build root,
or namespace, move the pure part into a helper and test that helper. Keep one
integration or smoke check for the real command path.

## Concrete Steps

Run these commands from the repository root.

1. Orientation:

   ```bash
   sed -n '1,240p' AGENTS.md
   sed -n '1,260p' PHILOSOPHY.md
   sed -n '1,260p' RUST_CODE_STYLE.md
   sed -n '1,220p' MANIFESTS_CODE_STYLE.md
   sed -n '1,220p' .agents/TESTING.md
   sed -n '1,240p' .agents/PLANS.md
   find .agents/knowledge -maxdepth 1 -type f -printf '%f\n' | sort
   ```

2. Worktree pre-task:

   ```bash
   git status --short --untracked-files=all
   ```

   Classify dirty and untracked files before changing code. Commit coherent
   checked work when it already makes sense. Do not commit ignored files, local
   settings, scratch files, half-finished work, or unrelated human notes.

3. Builder map:

   ```bash
   semeja search "package build flow and reproducibility check" src/cli/src
   semeja search "system build materialize packages overlays commit rootfs" src/cli/src
   semeja search "runtime dependency materializer precomputed deps" src/cli/src
   rg -n "^(pub |fn |impl |struct |enum |mod |use .*\\*)|#[cfg\\(test\\)]" \
     src/cli/src/build src/cli/src/system.rs src/cli/src/system \
     src/cli/src/materializer src/cli/src/outputs src/cli/src/commands/build.rs
   wc -l src/cli/src/build/*.rs src/cli/src/system/*.rs \
     src/cli/src/materializer/*.rs src/cli/src/outputs/*.rs \
     src/cli/src/commands/build.rs 2>/dev/null || true
   ```

   Adjust paths if the repo uses files rather than directories for a module.

4. Add the style audit. Name the command in this plan after adding it. A
   preferred shape is:

   ```bash
   scripts/check-builder-style.sh
   ```

   The script must be deterministic and must not write build outputs.

5. Refactor and test in slices. After each slice, run checks that match the
   changed files. Typical Rust checks are:

   ```bash
   cargo fmt --manifest-path src/cli/Cargo.toml -- --check
   cargo test --manifest-path src/cli/Cargo.toml
   cargo clippy --manifest-path src/cli/Cargo.toml --all-targets -- -D warnings
   ```

   If `cargo clippy` reports unrelated pre-existing warnings outside the target
   area, record the exact warnings in `Surprises & Discoveries`. Fix warnings
   in the target area before completing this plan.

6. Run behavior checks that prove the builder still works. Use small manifests
   or existing tests when possible. At minimum:

   ```bash
   cargo test --manifest-path src/cli/Cargo.toml
   cargo build --manifest-path src/cli/Cargo.toml
   ./src/cli/target/debug/nex check pkg/core/kernel/initramfs.yaml
   ./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
   ```

   Also run one strict package build that exercises source staging, build root
   setup, output commits, checksum handling, generated runtime deps, and the
   second reproducibility build. Prefer a small package that already has a
   stable checksum:

   ```bash
   ./src/cli/target/debug/nex build <small-package-manifest> \
     --verbose --single --check --update-checksum --force --compute-deps \
     --record-profile --generate-outputs
   ```

   Record why the chosen manifest proves the builder path. Do not use a large
   desktop application unless no smaller package covers the same path.

7. Run the final style gates:

   ```bash
   scripts/check-builder-style.sh
   cargo fmt --manifest-path src/cli/Cargo.toml -- --check
   cargo test --manifest-path src/cli/Cargo.toml
   cargo clippy --manifest-path src/cli/Cargo.toml --all-targets -- -D warnings
   ```

8. Final target inventory. Inspect every target file and record the result in
   `Artifacts and Notes`:

   ```bash
   find src/cli/src/build src/cli/src/materializer src/cli/src/outputs \
     -name '*.rs' -print | sort
   ```

   Include `src/cli/src/system/mod.rs` and `src/cli/src/commands/build.rs` or
   their replacement paths. For each target file, record its line count, its
   concern, and whether it has any accepted style exception. A completed plan
   should have no target-area exception unless this plan explains why the
   exception is better than strict compliance.

9. Qualitative review agent. Ask a separate agent to review the final builder
   code against `RUST_CODE_STYLE.md`, `PHILOSOPHY.md`, and this ExecPlan. The
   reviewer must focus on readability and design quality, not only mechanical
   rule checks.

   The review prompt must ask the reviewer to inspect the target builder area
   and answer these questions with file and line references:

   - Does each module own one concrete concern?
   - Does each public function read as a short outline?
   - Do names expose real builder concepts rather than generic plumbing?
   - Do error types and error messages help callers and users fix problems?
   - Do tests prove behavior at the right level?
   - Do comments explain hard choices and avoid line-by-line narration?
   - Would a professional Rust developer find the code pleasant to read?

   Record the reviewer findings in `Artifacts and Notes`. Fix every reviewer
   finding in the target area, rerun the relevant checks, and ask for another
   qualitative review. The plan can finish only after the reviewer reports no
   remaining target-area quality findings.

   If the execution environment cannot provide a separate qualitative review
   agent, stop under the blocking protocol. Do not self-certify the qualitative
   review gate.

## Validation and Acceptance

This plan is complete only when all of these are true:

- The target builder area passes the added style audit.
- No target Rust file exceeds 400 lines.
- No target function exceeds 60 lines.
- No target module uses glob imports.
- Every target module has a clear one-sentence module doc.
- Every public API in the target area has useful docs.
- Inline tests remain only in target modules under 100 lines.
- The stale materializer docs are corrected.
- The builder no longer has thousand-line catch-all modules.
- Package build flow and system build flow read as short outlines that call
  well-named helpers.
- The code no longer exits the process from inside reusable builder logic.
  Command handlers may decide process exit behavior.
- Target-area errors carry enough structure or context that a caller can tell
  build-script failure, checksum mismatch, missing manifest data, store
  failure, and reproducibility failure apart.
- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check` passes.
- `cargo test --manifest-path src/cli/Cargo.toml` passes.
- `cargo clippy --manifest-path src/cli/Cargo.toml --all-targets -- -D warnings`
  passes, or this plan records unrelated pre-existing warnings outside the
  target area and proves the target area has none.
- A strict package build passes and proves the real builder path still works.
- Any changed system build behavior has a system manifest check or assembly
  build proof.
- The final inventory in `Artifacts and Notes` names every target file and its
  concern.
- A separate qualitative review agent has reviewed the target area and reported
  no remaining quality findings.

Do not complete this plan with a note such as "future cleanup remains" for the
target builder area. If the target area still needs cleanup, keep working.

## Idempotence and Recovery

The refactor should preserve behavior. If a slice fails tests, use Git and the
test output to isolate that slice rather than editing blindly. Do not use
destructive Git commands unless the human explicitly asks.

The style audit must be safe to rerun at any time. It must not depend on build
artifacts, local store contents, or network access.

Build work directories under `.nex/tmp/` are disposable. Prefer normal builder
commands that recreate them. If manual cleanup becomes necessary, use the repo
cleanup script pattern from `.agents/cleanup-workdirs.sh` or add a small
targeted cleanup script before deleting large extracted trees.

If a strict package build fails because the chosen package is flaky or too
large for the sandbox, pick a smaller package that still exercises the same
builder path. Record the failed command, the reason it did not prove the goal,
and the replacement command.

If no separate qualitative review agent is available, this plan is blocked.
Ralph must state that the missing reviewer blocks completion, state that a
qualitative review agent is needed, print `@@BLOCKED@@`, and stop.

## Artifacts and Notes

Initial style assessment:

- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check` passed on
  2026-06-29.
- Target files that exceeded the style cap during the first inspection:
  - `src/cli/src/build/mod.rs`: 1478 lines.
  - `src/cli/src/build/orchestration.rs`: 1338 lines.
  - `src/cli/src/system/mod.rs`: 960 lines.
  - `src/cli/src/materializer/checkout.rs`: 569 lines.
  - `src/cli/src/materializer/flatten.rs`: 608 lines.
  - `src/cli/src/outputs/mod.rs`: 479 lines.
- `src/cli/src/materializer/mod.rs` has stale docs about ELF scanning.
- `src/cli/src/build/mod.rs`, `src/cli/src/system/mod.rs`, and
  `src/cli/src/build/orchestration.rs` use glob imports.

Future agents must append check transcripts, file inventories, and final
review notes here as the work proceeds.

Worktree pre-task, 2026-06-29 19:46Z:

- `git status --short --untracked-files=all` showed only local notes and
  settings as untracked: `.claude/settings.local.json`, `CURRENT_TODO.md`,
  `PROMPT.md`, `SONIQ_YOCTO_PARITY_PLAN.md`,
  `chromium-manifests-todo.md`, and
  `src/bootloader/.claude/settings.local.json`.
- `git diff --stat` printed no tracked dirty files.
- `git status --short --ignored --untracked-files=all` showed
  `.agents/execplans/005-builder-code-quality.md` and
  `.agents/execplans/006-graphical-qemu-smoke-tests.md` as ignored through
  `.gitignore:20:/.agents/`.
- No pre-task commit was made. The visible untracked files are local notes or
  local settings, and the active ExecPlans are ignored. Repo rules say not to
  commit ignored files.

Orientation, 2026-06-29 19:46Z:

- Read `AGENTS.md`, `PHILOSOPHY.md`, `RUST_CODE_STYLE.md`,
  `MANIFESTS_CODE_STYLE.md`, `.agents/TESTING.md`, `.agents/PLANS.md`,
  `.agents/knowledge/cli-testing.md`,
  `.agents/knowledge/reproducibility.md`,
  `.agents/knowledge/package-manifests.md`,
  `.agents/knowledge/system-assemblies.md`, and
  `.agents/knowledge/agent-workflow.md`.
- Listed knowledge files: `agent-workflow.md`, `cli-testing.md`,
  `kernel-and-boot.md`, `ostreefy-parity.md`, `package-manifests.md`,
  `reproducibility.md`, and `system-assemblies.md`.

Builder map and audit baseline, 2026-06-29 19:52Z:

- Added `scripts/check-builder-style.sh`. It checks target builder files for
  the mechanical rules that a script can judge: file length over 400 lines,
  function length over 60 lines, missing module docs, glob imports, and inline
  tests in modules with at least 100 lines.
- `bash -n scripts/check-builder-style.sh` passed.
- `scripts/check-builder-style.sh` failed as expected on the current builder
  tree. Major failures include:
  - `src/cli/src/build/mod.rs`: 1478 lines, missing module doc, glob imports,
    inline tests, and long functions including `run_build_script_with_env`
    and `build_package_manifest_with_dir`.
  - `src/cli/src/build/orchestration.rs`: 1338 lines, glob imports, inline
    tests, and long dependency graph functions.
  - `src/cli/src/system/mod.rs`: 960 lines, missing module doc, glob imports,
    and long functions including `build_system_manifest_with_dir`,
    `materialize_nex_structure`, and `apply_overlays`.
  - `src/cli/src/materializer/checkout.rs`,
    `src/cli/src/materializer/flatten.rs`, and
    `src/cli/src/materializer/resolver.rs`: file length and function length
    failures.
  - `src/cli/src/outputs/mod.rs`: 479 lines, missing module doc, glob import,
    inline tests, and long `fetch_and_verify_input`.
- `semeja search` confirmed the main package build path enters through
  `commands/build.rs`, then `build::build_single`, then
  `build_package_manifest_with_dir`; system builds enter
  `system::build_system_manifest_with_dir`; runtime dependency materialization
  uses `materializer::resolver::resolve_runtime_deps_precomputed`.

Final mechanical target inventory, 2026-06-29 22:18Z:

- `scripts/check-builder-style.sh`: 146 lines, target style audit.
- `src/cli/src/build/mod.rs`: 29 lines, build facade and re-exports.
- `src/cli/src/build/commits.rs`: 399 lines, output and bundle commits.
- `src/cli/src/build/dependencies.rs`: 73 lines, build dependency refs.
- `src/cli/src/build/env.rs`: 75 lines, build environment loading.
- `src/cli/src/build/inputs.rs`: 80 lines, source staging.
- `src/cli/src/build/package.rs`: 287 lines, package build flow.
- `src/cli/src/build/package_outputs.rs`: 160 lines, package output checks.
- `src/cli/src/build/reproducibility.rs`: 95 lines, second package build.
- `src/cli/src/build/rootfs.rs`: 300 lines, rootfs setup and layering.
- `src/cli/src/build/script.rs`: 283 lines, build script execution.
- `src/cli/src/build/status.rs`: 76 lines, manifest hash and stale checks.
- `src/cli/src/build/orchestration.rs`: 242 lines, orchestration facade.
- `src/cli/src/build/orchestration/*.rs`: each file is 336 lines or less and
  owns graph, checksums, hydrate, links, manifest lookup, planner, or trace.
- `src/cli/src/materializer/*.rs`: each file is 389 lines or less and owns
  checkout, resolver, flattening, type, or test concerns.
- `src/cli/src/outputs/*.rs`: each file is 255 lines or less and owns bundle
  commits, source inputs, categories, checksums, or tests.
- `src/cli/src/system/*.rs`: each file is 284 lines or less and owns system
  build flow, commits, dependency conversion, env vars, materialization,
  overlays, Nex DB, Nex links, or Nex shim installation.
- `src/cli/src/commands/build.rs`: 157 lines, CLI build options and dispatch.

Check transcript, 2026-06-29 22:18Z:

- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`: passed.
- `scripts/check-builder-style.sh`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml`: passed 69 unit tests and
  3 blob ref tests.
- `cargo build --manifest-path src/cli/Cargo.toml`: passed.
- `./src/cli/target/debug/nex check pkg/core/kernel/initramfs.yaml`: passed.
- `./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml`: passed.
- `./src/cli/target/debug/nex build pkg/core/nex/nex-ld-shim.yaml --verbose
  --single --check --update-checksum --force --compute-deps --record-profile
  --generate-outputs`: passed and reported `Build is reproducible. Checksums
  match.`
- `cargo clippy --manifest-path src/cli/Cargo.toml --all-targets --
  -D warnings`: failed only outside the target area after target warnings were
  fixed. Remaining files are listed under `Surprises & Discoveries`.

Follow-up commit check transcript, 2026-06-29 22:25Z:

- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`: passed.
- `scripts/check-builder-style.sh`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml system -- --nocapture`:
  passed 5 system-related tests.
- `cargo test --manifest-path src/cli/Cargo.toml`: passed 69 unit tests and
  3 blob ref tests.

Qualitative review fix check transcript, 2026-06-29 22:52Z:

- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`: passed.
- `scripts/check-builder-style.sh`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml materializer --
  --nocapture`: passed 10 materializer tests.
- `cargo test --manifest-path src/cli/Cargo.toml build::orchestration --
  --nocapture`: passed 2 orchestration tests. The stale dependency rebuild
  test printed the existing host user-namespace skip message but still passed.
- `cargo test --manifest-path src/cli/Cargo.toml`: passed 73 unit tests and
  3 blob ref tests.
- `./src/cli/target/debug/nex build pkg/core/nex/nex-ld-shim.yaml --verbose
  --single --check --update-checksum --force --compute-deps --record-profile
  --generate-outputs`: passed and reported `Build is reproducible. Checksums
  match.` The timing-only profile rewrite was restored.
- `./src/cli/target/debug/nex check pkg/core/kernel/initramfs.yaml`: passed.
- `./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml`: passed.

Second qualitative review fix check transcript, 2026-06-29 23:10Z:

- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`: passed.
- `scripts/check-builder-style.sh`: passed.
- `cargo test --manifest-path src/cli/Cargo.toml materializer --
  --nocapture`: passed 12 materializer tests.
- `cargo test --manifest-path src/cli/Cargo.toml`: passed 75 unit tests and
  3 blob ref tests.
- `./src/cli/target/debug/nex build pkg/core/nex/nex-ld-shim.yaml --verbose
  --single --check --update-checksum --force --compute-deps --record-profile
  --generate-outputs`: passed and reported `Build is reproducible. Checksums
  match.` The log showed the checksum comparison before second-pass output refs
  were committed. The timing-only profile rewrite was restored.
- `./src/cli/target/debug/nex check pkg/core/kernel/initramfs.yaml`: passed.
- `./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml`: passed.

Thirteenth qualitative review fix check transcript, 2026-06-29:

- `cargo fmt --manifest-path src/cli/Cargo.toml -- --check`: passed.
- `scripts/check-builder-style.sh`: passed.
- `rg -n "\.unwrap\(\)" src/cli/src/build src/cli/src/system
  src/cli/src/materializer src/cli/src/outputs
  src/cli/src/commands/build.rs`: returned no matches.
- `cargo test --manifest-path src/cli/Cargo.toml materializer::flatten --
  --nocapture`: passed 15 focused flatten tests. The two store-backed export
  tests skipped their zub commit step on this host because `commit_tree`
  reported `uid 0 not mapped in namespace`.
- `cargo test --manifest-path src/cli/Cargo.toml outputs::checksum --
  --nocapture`: passed 5 checksum tests.
- `cargo test --manifest-path src/cli/Cargo.toml`: passed 119 unit tests and
  3 blob ref tests.
- Target-filtered `cargo clippy --manifest-path src/cli/Cargo.toml
  --all-targets --message-format=json -- -W warnings`: passed with
  `TARGET_WARNINGS=0`.
- `cargo build --manifest-path src/cli/Cargo.toml`: passed.
- `./src/cli/target/debug/nex check pkg/core/kernel/initramfs.yaml`: passed.
- `./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml`: passed.
- `./src/cli/target/debug/nex build pkg/core/nex/nex-ld-shim.yaml --verbose
  --single --check --update-checksum --force --compute-deps --record-profile
  --generate-outputs`: passed and reported `Build is reproducible. Checksums
  match.` The timing-only profile rewrite was restored, and
  `./src/cli/target/debug/nex check pkg/core/nex/nex-ld-shim.yaml` passed
  after the restore.
