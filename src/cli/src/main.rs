use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use std::fmt;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

pub mod build;
pub mod cargo_vendor;
pub mod commands;
pub mod deps;
pub mod go_vendor;
pub mod manifest;
pub mod zig_vendor;
pub mod materializer;
pub mod outputs;
pub mod progress;
pub mod refs;
pub mod repo;
pub mod store;
pub mod system;

mod utils;

use build::*;
use deps::*;
use manifest::*;
use outputs::*;
use store::*;
#[derive(Parser)]
#[clap(
    name = "nex",
    version = "1.0",
    about = "2nex package builder and manager"
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build packages from manifests
    Build(commands::build::BuildArgs),

    /// Check manifest files for issues
    Check(commands::check::CheckArgs),

    /// List packages in the repository
    List(commands::list::ListArgs),

    /// Show information about a package
    Info(commands::info::InfoArgs),

    /// Search for packages
    Search(commands::search::SearchArgs),

    /// Pin dependencies to their current git blob SHAs
    Link(commands::link::LinkArgs),

    /// Enter staging mode (create overlay for transactional changes)
    Stage(commands::stage::StageArgs),

    /// Discard staged changes and exit staging mode
    Discard(commands::discard::DiscardArgs),

    /// Install a package (requires staging mode or --commit)
    Install(commands::install::InstallArgs),

    /// Remove a package (requires staging mode)
    Remove(commands::remove::RemoveArgs),

    /// Switch the active version of a package
    Switch(commands::switch::SwitchArgs),

    /// Commit staged changes
    Commit(commands::commit::CommitArgs),

    /// Show current and previous deployments
    Status(commands::status::StatusArgs),

    /// Rollback to a previous version
    Rollback(commands::rollback::RollbackArgs),

    /// Resolve and show runtime dependencies for a package
    Resolve(commands::resolve::ResolveArgs),

    /// Compute and store runtime dependencies in manifest
    ComputeDeps(commands::compute_deps::ComputeDepsArgs),

    /// Format manifest files
    Format(commands::format::FormatArgs),

    /// Generate ref completions for shell auto-completion
    Complete(commands::complete::CompleteArgs),

    /// Remote helper for SSH transport (internal use)
    #[clap(name = "zub-remote")]
    ZubRemote {
        /// Repository path
        path: PathBuf,
    },
}

// re-export BuildOpts for internal use
pub use commands::build::BuildOpts;

fn main() -> io::Result<()> {
    let cli: Cli = Cli::parse();

    match cli.command {
        Command::Build(args) => run_build(&args),
        Command::Check(args) => commands::check::run(&args),
        Command::List(args) => commands::list::run(&args),
        Command::Info(args) => commands::info::run(&args),
        Command::Search(args) => commands::search::run(&args),
        Command::Link(args) => link_manifest_dependencies(&args.manifest),
        Command::Stage(args) => commands::stage::run(&args),
        Command::Discard(args) => commands::discard::run(&args),
        Command::Install(args) => commands::install::run(&args),
        Command::Remove(args) => commands::remove::run(&args),
        Command::Switch(args) => commands::switch::run(&args),
        Command::Commit(args) => commands::commit::run(&args),
        Command::Status(args) => commands::status::run(&args),
        Command::Rollback(args) => commands::rollback::run(&args),
        Command::Resolve(args) => commands::resolve::run(&args),
        Command::ComputeDeps(args) => commands::compute_deps::run(&args),
        Command::Format(args) => commands::format::run(&args),
        Command::Complete(args) => commands::complete::run(&args),
        Command::ZubRemote { path } => run_zub_remote(&path),
    }
}

fn run_build(args: &commands::build::BuildArgs) -> io::Result<()> {
    // determine repo path based on context
    let repo_path = if let Some(ref r) = args.repo {
        // explicit repo path provided
        repo::resolve_repo_path(Some(r))?
    } else if args.system || Path::new(".nex/repo").exists() {
        // explicit --system flag or build-time context (local .nex/repo)
        repo::resolve_repo_path(None)?
    } else {
        // user context: use user's repo
        let ctx = repo::detect_context(false)?;
        if !ctx.repo_path.exists() {
            repo::ensure_user_dirs(&ctx)?;
            eprintln!(
                "Created user environment at {}",
                ctx.repo_path.parent().unwrap_or(&ctx.repo_path).display()
            );
        }
        ctx.repo_path.to_string_lossy().to_string()
    };

    // configure rayon thread pool if jobs specified
    if let Some(num_jobs) = args.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_jobs)
            .build_global()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to configure thread pool: {}", e),
                )
            })?;
    }

    let manifest_path = Path::new(&args.manifest);
    let manifest_dirs = vec![PathBuf::from(&args.manifest_dir)];
    let opts = BuildOpts::from_args(args, repo_path.clone());

    if args.hydrate_dependencies {
        // hydrate mode: expand dependencies to include all transitive deps
        hydrate_dependencies(&repo_path, &args.manifest)
    } else if args.single {
        // single mode: build only the specified manifest without dependencies
        build::build_single(&opts)
    } else {
        // default: build with full dependency resolution
        build_with_dependencies(
            &repo_path,
            manifest_path,
            &manifest_dirs,
            &opts,
            args.dry_run,
            args.add_checksums,
            args.show_dep_paths,
            args.force,
            args.trace_dependency.as_deref(),
        )
    }
}

fn hydrate_dependencies(_repo_path: &str, manifest_file: &str) -> io::Result<()> {
    // load manifest to get dependencies
    let manifest_data = load_manifest(manifest_file)?;

    let dependencies = match &manifest_data {
        ManifestData::Package(m) => &m.dependencies,
        ManifestData::System(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--hydrate-dependencies only applies to package manifests",
            ));
        }
    };

    // load manifest index for dependency resolution
    let manifest_index = manifest::ManifestIndex::load("pkg")?;

    // resolve all transitive dependencies (already in topological order, deepest first)
    let all_commits = resolve_dependency_closure(dependencies, &manifest_index)?;

    // convert commits to Dependency entries
    let hydrated_deps: Vec<Dependency> = all_commits
        .into_iter()
        .map(|commit| {
            // extract name from commit path like x86_64/zlib/1.3.1/sys/libs/bundles/dev
            let name = commit.split('/').nth(1).map(|s| s.to_string());
            Dependency {
                commit,
                name,
                manifest_ref: None,
            }
        })
        .collect();

    // read the original file
    let content = fs::read_to_string(manifest_file)?;

    // find the dependencies section and replace only that
    let lines: Vec<&str> = content.lines().collect();
    let mut dep_start = None;
    let mut dep_end = None;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("dependencies:") {
            dep_start = Some(i);
        } else if dep_start.is_some() && dep_end.is_none() {
            // check if this is a new top-level key (no indentation)
            if !line.is_empty()
                && !line.starts_with(' ')
                && !line.starts_with('\t')
                && !line.starts_with('-')
            {
                dep_end = Some(i);
                break;
            }
        }
    }

    let dep_start = dep_start.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No dependencies section found in manifest",
        )
    })?;
    let dep_end = dep_end.unwrap_or(lines.len());

    // build the new dependencies section
    let mut new_dep_section = vec!["dependencies:".to_string()];
    for dep in &hydrated_deps {
        if let Some(name) = &dep.name {
            new_dep_section.push(format!("  - name: {}", name));
            new_dep_section.push(format!("    commit: {}", dep.commit));
        } else {
            new_dep_section.push(format!("  - commit: {}", dep.commit));
        }
    }

    // reconstruct the file
    let mut result: Vec<String> = lines[..dep_start].iter().map(|s| s.to_string()).collect();
    result.extend(new_dep_section);
    result.extend(lines[dep_end..].iter().map(|s| s.to_string()));

    // write back
    let output = result.join("\n");
    // preserve trailing newline if original had one
    let output = if content.ends_with('\n') {
        format!("{}\n", output)
    } else {
        output
    };

    fs::write(manifest_file, output)?;

    println!(
        "Hydrated {} dependencies (was {})",
        hydrated_deps.len(),
        dependencies.len()
    );

    Ok(())
}

fn build_package_manifest(opts: &BuildOpts, manifest: &mut Manifest) -> io::Result<()> {
    build_package_manifest_with_dir(opts, manifest, ".nex/tmp/build_rootfs")
}

fn build_package_manifest_with_dir(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    base_dir: &str,
) -> io::Result<()> {
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    // resolve dependencies to specific commit IDs when they have manifest_ref
    let dependency_commits = resolve_dependency_commits(&manifest.dependencies, &opts.repo_path)?;

    setup_composite_rootfs(
        base_dir,
        &opts.repo_path,
        &opts.fallback_repos,
        &dependency_commits,
    )?;
    let input_env_vars = handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;

    let package_name = &manifest.package.name;
    let package_version = &manifest.package.version;
    let package_namespace = &manifest.package.namespace;

    println!(
        "Building {} {} in namespace {}",
        package_name, package_version, package_namespace
    );

    let mut env_vars = HashMap::new();
    env_vars.extend(input_env_vars);
    let build_script = manifest.build.script.clone();

    run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;

    // if generate_outputs is enabled, write auto-detected outputs to manifest
    if opts.generate_outputs {
        let out_dir = Path::new(base_dir).join("2nex/out");
        let categorized = categorize_files(&out_dir);
        crate::manifest::update::write_auto_outputs_to_manifest(&opts.manifest_file, &categorized)?;

        // reload the manifest to pick up the new outputs
        let reloaded = load_manifest(&opts.manifest_file)?;
        if let ManifestData::Package(reloaded_manifest) = reloaded {
            *manifest = reloaded_manifest;
        }
    }

    verify_and_commit_outputs(
        manifest,
        base_dir,
        &opts.repo_path,
        Path::new(&opts.manifest_file),
    )?;

    create_and_commit_bundles(
        manifest,
        base_dir,
        &opts.repo_path,
        Path::new(&opts.manifest_file),
    )?;

    println!("Build, packaging, and commit completed for all outputs.");

    let output_dir = Path::new(base_dir).join("2nex/out");
    let checksum = calculate_output_checksum(&output_dir)?;
    println!("Build output checksum: {}", checksum);

    match manifest.package.checksum.as_ref() {
        Some(expected_checksum) => {
            if checksum != *expected_checksum {
                if opts.update_checksum {
                    println!(
                        "Checksum mismatch (expected {}, calculated {}). Updating manifest.",
                        expected_checksum, checksum
                    );
                    update_manifest_checksum_field(
                        &opts.manifest_file,
                        ManifestKind::Package,
                        &checksum,
                    )?;
                    manifest.package.checksum = Some(checksum.clone());
                    // refresh store metadata with new manifest hash
                    refresh_package_metadata(
                        &opts.repo_path,
                        manifest,
                        Path::new(&opts.manifest_file),
                    )?;
                } else if !opts.check {
                    eprintln!(
                        "Checksum mismatch. Expected: {}, Calculated: {}",
                        expected_checksum, checksum
                    );
                    process::exit(-2);
                } else {
                    println!(
                        "Note: checksum differs from manifest (expected {}, got {}). Proceeding with reproducibility check.",
                        expected_checksum, checksum
                    );
                }
            } else {
                println!("Checksum verified successfully.");
            }
        }
        None => {
            if opts.update_checksum {
                println!(
                    "Manifest {} does not record a checksum. Storing {}.",
                    opts.manifest_file, checksum
                );
                update_manifest_checksum_field(
                    &opts.manifest_file,
                    ManifestKind::Package,
                    &checksum,
                )?;
                manifest.package.checksum = Some(checksum.clone());
                // refresh store metadata with new manifest hash
                refresh_package_metadata(
                    &opts.repo_path,
                    manifest,
                    Path::new(&opts.manifest_file),
                )?;
            }
        }
    }

    // create {hash}/files commit (union of all outputs) for dependency resolution
    create_files_commit_for_package(manifest, &opts.repo_path, Path::new(&opts.manifest_file))?;

    // compute runtime dependencies (opt-in, modifies manifest)
    if opts.compute_deps {
        commands::compute_deps::compute_deps_for_manifest(
            manifest,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
            opts.runtime_deps_verbose,
            false, // never dry_run during build
        )?;
        // refresh store metadata since compute_deps modified the manifest
        refresh_package_metadata(&opts.repo_path, manifest, Path::new(&opts.manifest_file))?;
    }

    if opts.check {
        println!("Validating build reproducibility by building the package a second time.");
        fs::remove_dir_all(base_dir)?;
        setup_composite_rootfs(
            base_dir,
            &opts.repo_path,
            &opts.fallback_repos,
            &dependency_commits,
        )?;
        handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;
        run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;
        verify_and_commit_outputs(
            manifest,
            base_dir,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
        )?;
        create_and_commit_bundles(
            manifest,
            base_dir,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
        )?;

        let second_checksum = calculate_output_checksum(&output_dir)?;
        println!("Second build output checksum: {}", second_checksum);

        if checksum == second_checksum {
            println!("Build is reproducible. Checksums match.");
        } else {
            println!("Build is not reproducible. Checksums do not match.");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Build is not reproducible.",
            ));
        }
    }

    append_checksum_file(&manifest.package, &checksum, &Path::new("checksums.txt"))?;

    Ok(())
}

fn refresh_package_metadata(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
) -> io::Result<()> {
    println!(
        "Refreshing store metadata for {}/{} ({})",
        manifest.package.slug, manifest.package.version, manifest.package.namespace
    );
    let manifest_hash = compute_manifest_hash(manifest_path)?;
    refresh_output_branches(repo_path, manifest, &manifest_hash)?;
    refresh_bundle_branches(repo_path, manifest, &manifest_hash)?;
    println!("Finished refreshing metadata for {}", manifest.package.slug);
    Ok(())
}

/// create {hash}/files commit as in-store union of all outputs.
fn create_files_commit_for_package(
    manifest: &Manifest,
    repo_path: &str,
    manifest_path: &Path,
) -> io::Result<()> {
    // determine address hash: checksum if stable, else manifest git blob SHA
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);

    let address_hash = if has_stable_checksum {
        manifest.package.checksum.clone().unwrap()
    } else {
        utils::hash_file_content(manifest_path)?
    };

    let files_ref = format!("{}/files", address_hash);

    // build output refs
    let output_refs: Vec<String> = manifest
        .outputs
        .keys()
        .filter(|k| *k != "discard")
        .map(|name| {
            format!(
                "x86_64/{}/{}/{}/outputs/{}",
                manifest.package.namespace_path(),
                manifest.package.slug,
                manifest.package.version,
                name
            )
        })
        .collect();

    if output_refs.is_empty() {
        return Ok(());
    }

    println!("Creating files commit: {}", files_ref);

    let repo = zub::Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let ref_strs: Vec<&str> = output_refs.iter().map(|s| s.as_str()).collect();
    zub::ops::union_trees(
        &repo,
        &ref_strs,
        &files_ref,
        zub::ops::UnionOptions {
            on_conflict: zub::ops::ConflictResolution::Last,
            ..Default::default()
        },
    )
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // attach metadata
    let metadata = vec![
        ("nex.address_hash".to_string(), address_hash.clone()),
        (
            "nex.package".to_string(),
            format!(
                "{}/{}/{}",
                manifest.package.namespace_path(),
                manifest.package.slug,
                manifest.package.version
            ),
        ),
    ];
    rewrite_branch_metadata(repo_path, &files_ref, &metadata)?;

    // create semantic files ref (x86_64/pkg/{namespace}/{slug}/{version}/files)
    let semantic_files_ref = format!(
        "x86_64/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version
    );

    // resolve the commit hash from the checksum-based ref
    let commit_hash = zub::resolve_ref(&repo, &files_ref)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // write the semantic ref pointing to the same commit
    zub::write_ref(&repo, &semantic_files_ref, &commit_hash)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // attach manifest hash to semantic ref for staleness checks
    let manifest_hash = compute_manifest_hash(manifest_path)?;
    rewrite_branch_metadata(
        repo_path,
        &semantic_files_ref,
        &[
            ("nex.manifest.hash".to_string(), manifest_hash),
            ("nex.address_hash".to_string(), address_hash),
        ],
    )?;

    println!("Created semantic ref: {}", semantic_files_ref);

    Ok(())
}

fn refresh_output_branches(
    repo_path: &str,
    manifest: &Manifest,
    manifest_hash: &str,
) -> io::Result<()> {
    for (category, spec) in &manifest.outputs {
        if category == "discard" {
            continue;
        }
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            category
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = output_branch_metadata(manifest, spec, manifest_hash)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}

fn refresh_bundle_branches(
    repo_path: &str,
    manifest: &Manifest,
    manifest_hash: &str,
) -> io::Result<()> {
    for (bundle_name, bundle) in &manifest.bundles {
        let branch_name = format!(
            "x86_64/{}/{}/{}/bundles/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            bundle_name
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = bundle_branch_metadata(manifest, bundle, manifest_hash)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}

// find manifest file for a given commit reference
fn find_manifest_for_commit(commit: &str, manifest_dirs: &[PathBuf]) -> io::Result<PathBuf> {
    use crate::refs::PackageRef;

    let pkg_ref = PackageRef::parse(commit).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid commit reference format: {} ({})", commit, e),
        )
    })?;

    let slug = &pkg_ref.slug;
    let namespace = &pkg_ref.namespace;

    // search in manifest directories
    // note: PackageRef.namespace does NOT include "pkg/" prefix, so we need to try both
    // base_dir/pkg/{namespace} and base_dir/{namespace}
    for base_dir in manifest_dirs {
        let mut namespace_paths = vec![
            base_dir.join("pkg").join(namespace), // try with pkg/ prefix first
            base_dir.join(namespace),             // then without
        ];
        // also handle case where base_dir already ends with "pkg"
        if base_dir.ends_with("pkg") {
            namespace_paths.push(base_dir.join(namespace));
        }

        for namespace_path in namespace_paths {
            // try direct path: {namespace}/{slug}.yaml
            let direct_path = namespace_path.join(format!("{}.yaml", slug));
            if direct_path.exists() {
                return Ok(direct_path);
            }

            // try with -slug suffix: {namespace}/*-{slug}.yaml
            if namespace_path.exists() && namespace_path.is_dir() {
                if let Ok(entries) = fs::read_dir(&namespace_path) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                                let filename =
                                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                if filename.ends_with(&format!("-{}", slug)) {
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "Could not find manifest for commit {} (slug: {}, namespace: {})",
            commit, slug, namespace
        ),
    ))
}

// compute SHA256 hash of manifest file
fn compute_manifest_hash(manifest_path: &Path) -> io::Result<String> {
    let contents = fs::read(manifest_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Resolve dependencies to specific commit IDs when they have manifest_ref.
/// Returns a list of (commit_ref_or_id, original_branch) pairs.
fn resolve_dependency_commits(
    dependencies: &[Dependency],
    repo_path: &str,
) -> io::Result<Vec<String>> {
    let git_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut resolved = Vec::new();

    for dep in dependencies {
        if let Some(ref blob_sha) = dep.manifest_ref {
            // fetch blob content and compute its hash
            match crate::utils::fetch_git_blob(&git_root, blob_sha) {
                Ok(content) => {
                    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

                    // search store history for matching build
                    match find_commit_by_manifest_hash(repo_path, &dep.commit, &content_hash) {
                        Ok(Some(commit_id)) => {
                            // use the specific commit ID instead of branch name
                            resolved.push(commit_id);
                            continue;
                        }
                        Ok(None) => {
                            // fall back to branch name
                            eprintln!(
                                "Warning: no matching commit found for {} with manifest_ref {}",
                                dep.commit, blob_sha
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: failed to search history for {}: {}",
                                dep.commit, e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to fetch blob {} for {}: {}",
                        blob_sha, dep.commit, e
                    );
                }
            }
        }
        // no manifest_ref or resolution failed - use branch name
        resolved.push(dep.commit.clone());
    }

    Ok(resolved)
}

// check if all outputs of a manifest are already built in the store with current manifest hash
// returns Some(commit_id) if found, None if not built or hash mismatch
fn check_if_built(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
) -> io::Result<Option<String>> {
    let arch = "x86_64"; // TODO: make configurable
    let slug = &manifest.package.slug;
    let version = &manifest.package.version;
    let namespace = manifest.package.namespace_path();

    // compute current manifest hash
    let current_hash = compute_manifest_hash(manifest_path)?;

    // check all outputs - we'll use the first output to find the commit
    let mut found_commit: Option<String> = None;

    for (output_name, _spec) in &manifest.outputs {
        let branch = format!(
            "{}/{}/{}/{}/outputs/{}",
            arch, namespace, slug, version, output_name
        );

        // check if branch exists
        if ensure_branch_exists(repo_path, &branch).is_err() {
            return Ok(None);
        }

        // try to find commit by manifest hash using history search
        match find_commit_by_manifest_hash(repo_path, &branch, &current_hash)? {
            Some(commit_id) => {
                if found_commit.is_none() {
                    found_commit = Some(commit_id);
                }
            }
            None => {
                // no commit found with matching hash
                println!(
                    "  Manifest {} has changed or no matching commit in history, rebuilding",
                    manifest_path.display()
                );
                return Ok(None);
            }
        }
    }

    Ok(found_commit)
}

// recursively collect dependencies and build graph
fn collect_dependencies_recursive(
    manifest_source: &ManifestSource,
    repo_path: &str,
    manifest_dirs: &[PathBuf],
    graph: &mut DiGraph<ManifestSource, ()>,
    manifest_map: &mut HashMap<PathBuf, NodeIndex>,
    force: bool,
    ref_cache: &mut HashMap<String, bool>,
) -> io::Result<NodeIndex> {
    let manifest_path = manifest_source.path();

    // check if already processed
    if let Some(&node) = manifest_map.get(manifest_path) {
        return Ok(node);
    }

    // load manifest from source (path or blob)
    let manifest_data = load_manifest_from_source(manifest_source)?;

    let (is_system, slug, dependencies) = match manifest_data {
        ManifestData::Package(ref m) => (false, m.package.slug.clone(), m.dependencies.clone()),
        ManifestData::System(ref s) => {
            // for system manifests, combine dependencies and packages into one list
            let mut all_deps = s.dependencies.clone();
            all_deps.extend(system::dependencies_from_system_packages(&s.packages));
            (true, s.system.slug.clone(), all_deps)
        }
    };

    // for package manifests, check if already built (unless force is true)
    if !is_system && !force {
        if let ManifestData::Package(ref manifest) = manifest_data {
            if check_if_built(repo_path, manifest, manifest_path)?.is_some() {
                println!("Package {} already built, skipping", manifest.package.slug);
                let node = graph.add_node(ManifestSource::Path(PathBuf::new())); // empty = skip
                manifest_map.insert(manifest_path.to_path_buf(), node);
                return Ok(node);
            }
        }
    }

    // add this manifest to graph
    let node = graph.add_node(manifest_source.clone());
    manifest_map.insert(manifest_path.to_path_buf(), node);

    println!("Processing dependencies for {}", slug);

    // find git repo root for blob fetching
    let git_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // process dependencies
    for dep in &dependencies {
        // if manifest_ref is set, use content-addressable resolution
        if let Some(ref blob_sha) = dep.manifest_ref {
            // fetch blob content and compute its hash
            match crate::utils::fetch_git_blob(&git_root, blob_sha) {
                Ok(content) => {
                    use sha2::{Digest, Sha256};
                    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

                    // search store history for matching build
                    match find_commit_by_manifest_hash(repo_path, &dep.commit, &content_hash) {
                        Ok(Some(commit_id)) => {
                            println!("  [{}] {} skipping", &commit_id[..12], dep.commit);
                            continue;
                        }
                        Ok(None) => {
                            println!(
                                "  [needs build] {} (pinned to {})",
                                dep.commit,
                                &blob_sha[..12]
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "  Warning: failed to search history for {}: {}",
                                dep.commit, e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "  Warning: failed to fetch blob {} for {}: {}",
                        blob_sha, dep.commit, e
                    );
                }
            }
        } else {
            // no manifest_ref - floating mode: check if branch exists AND manifest hash matches
            let branch_exists = ref_cache
                .entry(dep.commit.clone())
                .or_insert_with(|| ensure_branch_exists(repo_path, &dep.commit).is_ok());

            if *branch_exists {
                // branch exists, now check if manifest hash matches
                match find_manifest_for_commit(&dep.commit, manifest_dirs) {
                    Ok(dep_manifest_path) => {
                        let manifest_hash = compute_manifest_hash(&dep_manifest_path)?;
                        match find_commit_by_manifest_hash(repo_path, &dep.commit, &manifest_hash) {
                            Ok(Some(_)) => {
                                println!("  [floating] {} skipping", dep.commit);
                                continue;
                            }
                            Ok(None) => {
                                println!(
                                    "  [floating/stale] {} manifest changed, rebuilding",
                                    dep.commit
                                );
                                // fall through to rebuild
                            }
                            Err(e) => {
                                eprintln!(
                                    "  Warning: failed to check manifest hash for {}: {}",
                                    dep.commit, e
                                );
                                // fall through to rebuild to be safe
                            }
                        }
                    }
                    Err(_) => {
                        // can't find manifest, assume it's a pre-built dep and skip
                        println!("  [floating] {} skipping (no manifest found)", dep.commit);
                        continue;
                    }
                }
            }
        }

        // find manifest for this dependency
        match find_manifest_for_commit(&dep.commit, manifest_dirs) {
            Ok(dep_manifest_path) => {
                // check if already processed before printing
                if manifest_map.contains_key(&dep_manifest_path) {
                    // already processed, just get the node for edge creation
                    let dep_node = manifest_map[&dep_manifest_path];
                    if !graph[dep_node].is_empty() {
                        graph.add_edge(dep_node, node, ());
                    }
                    continue;
                }

                println!(
                    "  Found dependency manifest: {}",
                    dep_manifest_path.display()
                );

                // create ManifestSource based on whether manifest_ref is present
                let dep_source = if let Some(ref blob_sha) = dep.manifest_ref {
                    ManifestSource::Blob {
                        sha: blob_sha.clone(),
                        path: dep_manifest_path,
                    }
                } else {
                    ManifestSource::Path(dep_manifest_path)
                };

                // recurse
                let dep_node = collect_dependencies_recursive(
                    &dep_source,
                    repo_path,
                    manifest_dirs,
                    graph,
                    manifest_map,
                    force,
                    ref_cache,
                )?;

                // add edge: dep must be built before current
                // edge direction: dep_node -> node (dep comes before dependent)
                // only add edge if dep_node is a real node (not empty marker)
                if !graph[dep_node].is_empty() {
                    graph.add_edge(dep_node, node, ());
                }
            }
            Err(e) => {
                eprintln!(
                    "  Warning: Could not find manifest for dependency {}: {}",
                    dep.commit, e
                );
                // continue anyway - might be a bootstrap dependency that's already built
            }
        }
    }

    Ok(node)
}

// show how packages would be built in parallel waves
fn show_parallel_execution_plan(
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
    compact: bool,
) {
    let mut dependencies: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for &node in build_order {
        let deps: Vec<NodeIndex> = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .filter(|&dep| !graph[dep].is_empty())
            .collect();
        dependencies.insert(node, deps);
    }

    let mut remaining: Vec<NodeIndex> = build_order.to_vec();
    let mut completed = HashSet::new();
    let mut wave_num = 0;

    while !remaining.is_empty() {
        wave_num += 1;

        // find all packages that can be built in this wave
        let wave: Vec<NodeIndex> = remaining
            .iter()
            .filter(|&&node| {
                let deps = &dependencies[&node];
                deps.iter().all(|dep| completed.contains(dep))
            })
            .cloned()
            .collect();

        if wave.is_empty() {
            println!("\nERROR: Cannot make progress - circular dependency detected");
            break;
        }

        let wave_names: Vec<String> = wave
            .iter()
            .map(|&node_idx| {
                let path = graph[node_idx].path();
                let filename = path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("unknown");
                filename.trim_end_matches(".yaml").to_string()
            })
            .collect();

        if compact {
            println!(
                "  Wave {} ({}): {}",
                wave_num,
                wave.len(),
                wave_names.join(", ")
            );
        } else {
            println!(
                "\n  Wave {}: {} package(s) in parallel",
                wave_num,
                wave.len()
            );
            for name in &wave_names {
                println!("    - {}", name);
            }
        }

        // mark as completed
        for &node in &wave {
            completed.insert(node);
        }

        // remove from remaining
        remaining.retain(|node| !completed.contains(node));
    }
}

// build a manifest and all its missing dependencies in parallel
fn build_packages_parallel(
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
    opts: &BuildOpts,
) -> io::Result<()> {
    // ensure tmp directory exists
    fs::create_dir_all(".nex/tmp")?;

    // create shared MultiProgress for all parallel builds
    let multi_progress = Arc::new(indicatif::MultiProgress::new());

    let total = build_order.len();
    let completed = Arc::new(Mutex::new(HashSet::new()));
    let build_counter = Arc::new(Mutex::new(0usize));

    // create a map of node -> dependencies for quick lookup
    let mut dependencies: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for &node in build_order {
        let deps: Vec<NodeIndex> = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .filter(|&dep| !graph[dep].is_empty())
            .collect();
        dependencies.insert(node, deps);
    }

    // build in waves: at each step, build all packages whose deps are complete
    let mut remaining: Vec<NodeIndex> = build_order.to_vec();

    while !remaining.is_empty() {
        // find all packages that can be built in this wave
        let mut wave: Vec<NodeIndex> = remaining
            .iter()
            .filter(|&&node| {
                let deps = &dependencies[&node];
                let completed_set = completed.lock().unwrap();
                deps.iter().all(|dep| completed_set.contains(dep))
            })
            .cloned()
            .collect();

        if wave.is_empty() {
            // no progress can be made - shouldn't happen with valid toposort
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot make progress: all remaining packages have unmet dependencies",
            ));
        }

        // bootstrap packages must build sequentially (they share .nex/tmp/build_rootfs directory)
        // if this wave contains bootstrap packages, only build one at a time
        let has_bootstrap = wave.iter().any(|&node_idx| {
            let source = &graph[node_idx];
            if let Ok(manifest_data) = load_manifest_from_source(source) {
                matches!(manifest_data, ManifestData::Package(m) if m.package.bootstrap)
            } else {
                false
            }
        });

        if has_bootstrap {
            // find the first bootstrap package and build only that one
            let bootstrap_idx = wave
                .iter()
                .position(|&node_idx| {
                    let source = &graph[node_idx];
                    if let Ok(manifest_data) = load_manifest_from_source(source) {
                        matches!(manifest_data, ManifestData::Package(m) if m.package.bootstrap)
                    } else {
                        false
                    }
                })
                .unwrap();
            wave = vec![wave[bootstrap_idx]];
        }

        println!("Building wave of {} package(s) in parallel...", wave.len());

        // build all packages in this wave in parallel
        let results: Vec<Result<String, String>> = wave
            .par_iter()
            .map(|&node_idx| {
                let source = &graph[node_idx];
                let path = source.path();
                let build_num = {
                    let mut counter = build_counter.lock().unwrap();
                    *counter += 1;
                    *counter
                };

                // load manifest from source (path or blob)
                let manifest_data = match load_manifest_from_source(source) {
                    Ok(data) => data,
                    Err(e) => return Err(format!("Failed to load {}: {}", path.display(), e)),
                };

                let result = match manifest_data {
                    ManifestData::Package(mut manifest) => {
                        // bootstrap packages must use fixed directory name so GCC's hardcoded sysroot path remains valid
                        let build_dir = if manifest.package.bootstrap {
                            ".nex/tmp/build_rootfs".to_string()
                        } else {
                            format!(
                                ".nex/tmp/build_rootfs_{}_{}",
                                manifest.package.slug.replace("/", "_"),
                                manifest.package.namespace.replace("/", "_")
                            )
                        };

                        // create opts for this build
                        let build_opts = BuildOpts {
                            repo_path: opts.repo_path.clone(),
                            manifest_file: path.to_str().unwrap().to_string(),
                            check: opts.check,
                            update_checksum: opts.update_checksum,
                            bootstrap: manifest.package.bootstrap,
                            compute_deps: opts.compute_deps,
                            runtime_deps_verbose: opts.runtime_deps_verbose,
                            refresh_metadata: opts.refresh_metadata,
                            force: opts.force,
                            build_dir: None,
                            generate_outputs: opts.generate_outputs,
                            fallback_repos: opts.fallback_repos.clone(),
                            verbose: opts.verbose,
                            record_profile: opts.record_profile,
                            no_progress: opts.no_progress,
                            multi_progress: Some(multi_progress.clone()),
                        };

                        println!(
                            "[{}/{}] Building: {}",
                            build_num, total, manifest.package.slug
                        );

                        // build the package
                        let slug = manifest.package.slug.clone();
                        if let Err(e) = crate::build::build_package_manifest_with_dir(
                            &build_opts,
                            &mut manifest,
                            &build_dir,
                        ) {
                            return Err(format!("Failed to build {}: {}", slug, e));
                        }

                        // clean up build directory after each package
                        let build_path = Path::new(&build_dir);
                        if build_path.exists() {
                            if let Err(e) = fs::remove_dir_all(build_path) {
                                eprintln!("Warning: failed to clean up {}: {}", build_dir, e);
                            }
                        }

                        Ok(slug)
                    }
                    ManifestData::System(system_manifest) => {
                        let build_dir = format!(
                            ".nex/tmp/build_rootfs_{}_{}",
                            system_manifest.system.slug.replace("/", "_"),
                            "system"
                        );

                        let build_opts = BuildOpts {
                            repo_path: opts.repo_path.clone(),
                            manifest_file: path.to_str().unwrap().to_string(),
                            check: opts.check,
                            update_checksum: opts.update_checksum,
                            bootstrap: opts.bootstrap,
                            compute_deps: opts.compute_deps,
                            runtime_deps_verbose: opts.runtime_deps_verbose,
                            refresh_metadata: opts.refresh_metadata,
                            force: opts.force,
                            build_dir: None,
                            generate_outputs: opts.generate_outputs,
                            fallback_repos: opts.fallback_repos.clone(),
                            verbose: opts.verbose,
                            record_profile: opts.record_profile,
                            no_progress: opts.no_progress,
                            multi_progress: Some(multi_progress.clone()),
                        };

                        println!(
                            "[{}/{}] Building system: {}",
                            build_num, total, system_manifest.system.slug
                        );

                        // build the system using the legacy builder since it handles all the dependency resolution
                        let slug = system_manifest.system.slug.clone();
                        if let Err(e) = system::build_system_manifest_with_dir(
                            &build_opts,
                            &system_manifest,
                            &build_dir,
                        ) {
                            return Err(format!("Failed to build system {}: {}", slug, e));
                        }

                        // clean up build directory
                        let build_path = Path::new(&build_dir);
                        if build_path.exists() {
                            if let Err(e) = fs::remove_dir_all(build_path) {
                                eprintln!("Warning: failed to clean up {}: {}", build_dir, e);
                            }
                        }

                        Ok(slug)
                    }
                };

                result
            })
            .collect();

        // check results and mark completed
        for (i, result) in results.iter().enumerate() {
            match result {
                Ok(slug) => {
                    println!("✓ Successfully built {}", slug);
                    completed.lock().unwrap().insert(wave[i]);
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, e.clone()));
                }
            }
        }

        // remove completed packages from remaining
        remaining.retain(|node| !completed.lock().unwrap().contains(node));
    }

    Ok(())
}

fn show_dependency_paths(
    graph: &DiGraph<ManifestSource, ()>,
    manifest_map: &HashMap<PathBuf, NodeIndex>,
    root_path: &Path,
) {
    use petgraph::visit::Dfs;

    let root_node = manifest_map
        .get(root_path)
        .expect("Root manifest should be in map");

    // for each node in the graph, show the path from root to that node
    for (path, &node) in manifest_map.iter() {
        if path == root_path || *path == PathBuf::new() {
            continue; // skip root and empty markers
        }

        // find a path from root to this node using DFS
        let mut dfs = Dfs::new(graph, *root_node);
        let mut parent_map: HashMap<NodeIndex, Option<NodeIndex>> = HashMap::new();
        parent_map.insert(*root_node, None);

        while let Some(current) = dfs.next(graph) {
            if current == node {
                // reconstruct path
                let mut path_nodes = vec![current];
                let mut cur = current;
                while let Some(Some(parent)) = parent_map.get(&cur) {
                    path_nodes.push(*parent);
                    cur = *parent;
                }
                path_nodes.reverse();

                // print path
                print!("  ");
                for (i, &n) in path_nodes.iter().enumerate() {
                    let p = graph[n].path();
                    if i > 0 {
                        print!(" → ");
                    }
                    if let Some(name) = p.file_stem().and_then(|s| s.to_str()) {
                        print!("{}", name);
                    } else {
                        print!("{}", p.display());
                    }
                }
                println!();
                break;
            }

            // record parents
            for neighbor in graph.neighbors(current) {
                parent_map.entry(neighbor).or_insert(Some(current));
            }
        }
    }
}

fn add_missing_checksums_to_manifests(
    build_order: &[NodeIndex],
    graph: &DiGraph<ManifestSource, ()>,
    repo_path: &str,
    opts: &BuildOpts,
) -> io::Result<()> {
    use crate::manifest::update::update_manifest_checksum_field;
    use crate::manifest::ManifestKind;
    use crate::store::get_branch_metadata;

    for &node_idx in build_order {
        let source = &graph[node_idx];
        let manifest_path = source.path();
        let manifest_data = load_manifest_from_source(source)?;

        match manifest_data {
            ManifestData::Package(manifest) => {
                // skip if already has checksum
                if manifest.package.checksum.is_some() {
                    continue;
                }

                println!("Processing: {}", manifest.package.slug);

                // try to get checksum from store (check first bundle)
                if let Some((bundle_name, _)) = manifest.bundles.iter().next() {
                    let commit_ref = format!(
                        "x86_64/{}/{}/{}/bundles/{}",
                        manifest.package.namespace_path(),
                        manifest.package.slug,
                        manifest.package.version,
                        bundle_name
                    );

                    match get_branch_metadata(repo_path, &commit_ref, "nex.output.checksum") {
                        Ok(checksum) => {
                            println!("  Found checksum in store: {}", checksum);
                            update_manifest_checksum_field(
                                manifest_path.to_str().unwrap(),
                                ManifestKind::Package,
                                &checksum,
                            )?;
                            continue;
                        }
                        Err(_) => {
                            println!("  Not found in store, building to get checksum...");
                        }
                    }
                }

                // build to get checksum
                let mut manifest_copy = manifest.clone();
                let build_opts = BuildOpts {
                    repo_path: repo_path.to_string(),
                    manifest_file: manifest_path.to_str().unwrap().to_string(),
                    check: false,
                    update_checksum: true, // enable checksum updating
                    bootstrap: manifest.package.bootstrap,
                    compute_deps: opts.compute_deps,
                    runtime_deps_verbose: opts.runtime_deps_verbose,
                    refresh_metadata: false,
                    force: false,
                    build_dir: None,
                    generate_outputs: false, // don't auto-generate outputs when adding checksums
                    fallback_repos: opts.fallback_repos.clone(),
                    verbose: opts.verbose,
                    record_profile: opts.record_profile,
                    no_progress: opts.no_progress,
                    multi_progress: None,
                };

                build_package_manifest(&build_opts, &mut manifest_copy)?;
                println!("  Built and checksummed");
            }
            ManifestData::System(_) => {
                // system manifests don't need checksums for this purpose
                continue;
            }
        }
    }

    Ok(())
}

/// Trace and display dependency chains that include a specific pattern.
/// Uses manifest-based dependency resolution.
fn trace_dependency_chains(
    _repo_path: &str,
    manifest_path: &Path,
    _manifest_dirs: &[PathBuf],
    pattern: &str,
) -> io::Result<()> {
    println!("Tracing dependencies matching pattern: '{}'", pattern);
    println!("Starting from: {}\n", manifest_path.display());

    // load manifest index for dependency resolution
    let manifest_index = manifest::ManifestIndex::load("pkg")?;

    // load the root manifest
    let manifest_data = load_manifest(manifest_path.to_str().unwrap())?;
    let (root_slug, root_deps) = match manifest_data {
        ManifestData::Package(ref m) => (m.package.slug.clone(), m.dependencies.clone()),
        ManifestData::System(ref s) => {
            let mut all_deps = s.dependencies.clone();
            all_deps.extend(system::dependencies_from_system_packages(&s.packages));
            (s.system.slug.clone(), all_deps)
        }
    };

    // recursively trace all dependencies
    let mut found_matches = false;
    for dep in &root_deps {
        let mut chain = vec![root_slug.clone()];
        if trace_commit_recursive(&dep.commit, pattern, &mut chain, &manifest_index)? {
            found_matches = true;
        }
    }

    if !found_matches {
        println!("No dependencies matching pattern '{}' found.", pattern);
    }

    Ok(())
}

/// Recursively trace a commit and its dependencies for a pattern.
/// Uses manifest-based dependency resolution via needs/resolution.
fn trace_commit_recursive(
    commit: &str,
    pattern: &str,
    chain: &mut Vec<String>,
    manifest_index: &manifest::ManifestIndex,
) -> io::Result<bool> {
    // check if this commit matches the pattern
    let matches_pattern = commit.contains(pattern);

    // extract package name from commit for display
    let pkg_name = if let Some((slug, version, namespace)) = parse_commit_ref(commit) {
        format!("{}/{}/{}", namespace, slug, version)
    } else {
        commit.to_string()
    };

    chain.push(pkg_name.clone());

    // if this commit matches the pattern, print the chain
    if matches_pattern {
        println!("Found match: {}", commit);
        println!("  Chain: {}", chain.join(" → "));
        println!();
        chain.pop();
        return Ok(true);
    }

    // get runtime dependencies from manifest (using same logic as deps/mod.rs)
    let deps = get_manifest_deps(commit, manifest_index);

    // recursively check each dependency
    let mut found_in_subtree = false;
    for dep_commit in &deps {
        if trace_commit_recursive(dep_commit, pattern, chain, manifest_index)? {
            found_in_subtree = true;
        }
    }

    chain.pop();
    Ok(found_in_subtree)
}

/// Get runtime dependencies for a commit from its manifest.
fn get_manifest_deps(commit: &str, manifest_index: &manifest::ManifestIndex) -> Vec<String> {
    // parse commit to get namespace/slug
    let parts: Vec<&str> = commit.split('/').collect();
    let pkg_idx = match parts.iter().position(|&p| p == "pkg") {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    let end_idx = match parts.iter().position(|&p| p == "outputs" || p == "bundles") {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    if end_idx <= pkg_idx + 2 {
        return Vec::new();
    }

    let slug_idx = end_idx - 2;
    let namespace = parts[pkg_idx + 1..slug_idx].join("/");
    let slug = parts[slug_idx];

    let manifest = match manifest_index.get_manifest(&namespace, slug) {
        Some(m) => m,
        None => return Vec::new(),
    };

    // collect unique dep commits from resolution map
    let mut deps = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for dep_name in manifest.resolution.values() {
        if dep_name == "self" {
            continue;
        }
        if let Some(dep) = manifest
            .dependencies
            .iter()
            .find(|d| d.name.as_deref() == Some(dep_name))
        {
            if seen.insert(dep.commit.clone()) {
                deps.push(dep.commit.clone());
            }
        }
    }

    deps
}

fn build_with_dependencies(
    repo_path: &str,
    manifest_path: &Path,
    manifest_dirs: &[PathBuf],
    opts: &BuildOpts,
    dry_run: bool,
    add_checksums: bool,
    show_dep_paths: bool,
    force: bool,
    trace_dependency: Option<&str>,
) -> io::Result<()> {
    if dry_run {
        println!(
            "DRY RUN: Analyzing dependency graph for {}",
            manifest_path.display()
        );
    } else {
        println!("Building dependency graph for {}", manifest_path.display());
    }

    let mut graph = DiGraph::new();
    let mut manifest_map = HashMap::new();
    let mut ref_cache = HashMap::new();

    // collect all dependencies recursively
    // when add_checksums is true, treat it like force to include already-built packages
    let root_source = ManifestSource::Path(manifest_path.to_path_buf());
    collect_dependencies_recursive(
        &root_source,
        repo_path,
        manifest_dirs,
        &mut graph,
        &mut manifest_map,
        force || add_checksums,
        &mut ref_cache,
    )?;

    // if tracing dependencies, show all packages that pull in the traced pattern
    if let Some(pattern) = trace_dependency {
        trace_dependency_chains(repo_path, manifest_path, manifest_dirs, pattern)?;
        return Ok(());
    }

    // filter out empty markers (already-built packages)
    let valid_nodes: Vec<NodeIndex> = graph
        .node_indices()
        .filter(|&idx| !graph[idx].is_empty())
        .collect();

    if valid_nodes.is_empty() {
        println!("All packages already built!");
        return Ok(());
    }

    println!(
        "\nDependency graph has {} packages to build",
        valid_nodes.len()
    );

    // show dependency paths if requested
    if show_dep_paths {
        println!("\nDependency paths:");
        show_dependency_paths(&graph, &manifest_map, manifest_path);
    }

    // topological sort to get build order
    let build_order = toposort(&graph, None).map_err(|cycle| {
        let node_source = &graph[cycle.node_id()];
        io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Circular dependency detected at node {:?}: {}",
                cycle.node_id(),
                node_source.path().display()
            ),
        )
    })?;

    // filter build order to only include valid nodes
    let build_order: Vec<NodeIndex> = build_order
        .into_iter()
        .filter(|&idx| !graph[idx].is_empty())
        .collect();

    // if add_checksums mode, process manifests to add missing checksums
    if add_checksums {
        println!("\nAdding missing checksums...");
        add_missing_checksums_to_manifests(&build_order, &graph, repo_path, opts)?;
        println!("Checksums updated!");
        return Ok(());
    }

    // if dry run, show parallel execution plan
    if dry_run {
        println!("\nDRY RUN: Parallel execution plan:");
        show_parallel_execution_plan(&graph, &build_order, false);
        println!("\nDRY RUN: Would build {} packages", build_order.len());
        return Ok(());
    }

    println!("\nBuild order:");
    for (i, &node_idx) in build_order.iter().enumerate() {
        let source = &graph[node_idx];
        println!("  {}. {}", i + 1, source.path().display());
    }

    println!("\nParallel execution plan:");
    show_parallel_execution_plan(&graph, &build_order, true);

    // build in parallel waves
    println!("\nStarting parallel builds...\n");
    build_packages_parallel(&graph, &build_order, opts)?;

    println!("==================================================");
    println!("All packages built successfully!");
    println!("==================================================");

    Ok(())
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.namespace, self.slug)
    }
}

/// run the zub remote helper protocol (server side of SSH transport)
fn run_zub_remote(repo_path: &Path) -> io::Result<()> {
    let repo = zub::Repo::open(repo_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    zub::transport::serve_remote(&repo)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

/// Pin all dependencies in a manifest to their current git blob SHAs.
fn link_manifest_dependencies(manifest_file: &str) -> io::Result<()> {
    use crate::utils::hash_file_content;

    // load manifest
    let manifest_data = load_manifest(manifest_file)?;

    let (is_system, dependencies, packages) = match &manifest_data {
        ManifestData::Package(m) => (false, m.dependencies.clone(), Vec::new()),
        ManifestData::System(s) => (true, s.dependencies.clone(), s.packages.clone()),
    };

    if is_system {
        println!("Linking system manifest: {}", manifest_file);
    } else {
        println!("Linking package manifest: {}", manifest_file);
    }

    // use current directory as manifest search path
    let manifest_dirs = vec![PathBuf::from(".")];

    // read original file content
    let original_content = fs::read_to_string(manifest_file)?;

    // process dependencies
    let mut updated_content = original_content.clone();
    let mut linked_count = 0;

    for dep in &dependencies {
        // find the manifest file using existing resolver
        match find_manifest_for_commit(&dep.commit, &manifest_dirs) {
            Ok(manifest_path) => {
                // calculate git blob SHA
                let blob_sha = hash_file_content(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), blob_sha);

                // update the content to add manifest_ref
                // find the dependency entry and add manifest_ref after commit
                let commit_pattern = format!("commit: {}", dep.commit);
                if let Some(pos) = updated_content.find(&commit_pattern) {
                    // check if manifest_ref already exists for this entry
                    let after_commit = pos + commit_pattern.len();
                    let next_section = updated_content[after_commit..]
                        .find("\n  - ")
                        .or_else(|| updated_content[after_commit..].find("\nsources:"))
                        .or_else(|| updated_content[after_commit..].find("\npackages:"))
                        .unwrap_or(updated_content.len() - after_commit);

                    let entry_section = &updated_content[pos..after_commit + next_section];
                    if !entry_section.contains("manifest_ref:") {
                        // insert manifest_ref after commit line
                        let insert_pos = after_commit;
                        let insertion = format!("\n    manifest_ref: {}", blob_sha);
                        updated_content.insert_str(insert_pos, &insertion);
                        linked_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("  Warning: {}: {}", dep.commit, e);
            }
        }
    }

    // process packages (for system manifests)
    for pkg in &packages {
        match find_manifest_for_commit(&pkg.commit, &manifest_dirs) {
            Ok(manifest_path) => {
                let blob_sha = hash_file_content(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), blob_sha);
                // for packages, we'd need to add manifest_ref support to SystemPackage
            }
            Err(e) => {
                eprintln!("  Warning: {}: {}", pkg.commit, e);
            }
        }
    }

    // write updated content
    if linked_count > 0 {
        fs::write(manifest_file, updated_content)?;
        println!(
            "\nLinked {} dependencies in {}",
            linked_count, manifest_file
        );
    } else {
        println!("\nNo dependencies to link or all already linked");
    }

    Ok(())
}

/// Verifies and commits outputs to store branches based on the manifest.
///
/// This function takes the manifest, base directory, and repository path as input.
/// It verifies and commits the outputs specified in the manifest to the corresponding
/// store branches. The function categorizes the output files, checks their existence,
/// moves them to the appropriate output directories, and commits them to the
/// repository.

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn base_manifest() -> &'static str {
        r#"
package:
  schema: 1
  name: sample
  slug: sample
  namespace: bootstrap/phase0
  version: "1.0"
dependencies: []
sources: []
build:
  script: "true"
outputs: {}
"#
    }

    #[test]
    fn parses_bundle_with_includes() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    includes:\n      - bin\n      - lib\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
    }

    #[test]
    fn parses_output_with_files_and_needs() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen(
                "outputs: {}",
                "outputs:\n  bin:\n    files:\n      - path: /usr/bin/foo\n        needs:\n          - /usr/lib/libc.so.6",
                1,
            )
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].path, "/usr/bin/foo");
        assert_eq!(
            output.files[0].needs,
            vec!["/usr/lib/libc.so.6".to_string()]
        );
    }

    #[test]
    fn parses_simple_bundle_format() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    - bin\n    - lib\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
    }

    // note: old closure_resolution tests removed - now uses manifest-based resolution
    // which requires actual manifest files, not mock fetch functions

    #[test]
    fn detects_system_manifest_kind() {
        let yaml = r#"
schema: 1
kind: system
system:
  name: Demo
  slug: demo
  version: "1.0"
packages:
  - commit: x86_64/foo/1.0/base/bundles/dev
build:
  script: ":"
sources: []
dependencies: []
"#;
        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(detect_manifest_kind(&value), ManifestKind::System);
        let sys: SystemManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(sys.system.slug, "demo");
        assert_eq!(sys.system.version, "1.0");
    }

    #[test]
    fn load_manifest_parses_system_kind() {
        let yaml = r#"
schema: 1
kind: system
system:
  name: Demo
  slug: demo
  version: "1.0"
packages:
  - commit: x86_64/foo/1.0/base/bundles/dev
build:
  script: ":"
sources: []
dependencies: []
"#;
        let mut file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file, yaml.as_bytes()).unwrap();
        match load_manifest(file.path().to_str().unwrap()).unwrap() {
            ManifestData::System(sys) => {
                assert_eq!(sys.system.slug, "demo");
                assert_eq!(sys.packages.len(), 1);
            }
            _ => panic!("Expected system manifest"),
        }
    }
}
