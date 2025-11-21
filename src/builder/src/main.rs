use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};

use clap::Parser;
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use std::fmt;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;

pub mod build;
pub mod deps;
pub mod manifest;
pub mod ostree;
pub mod outputs;
pub mod runtime;
pub mod system;

mod utils;

use build::*;
use deps::*;
use manifest::*;
use ostree::*;
use outputs::*;
use runtime::scanner::{RuntimeScanResult, RuntimeScanner};

#[derive(Parser)]
#[clap(version = "1.0", author = "Your Name")]
struct Cli {
    // positional args
    #[clap(value_name = "REPO")]
    repo_path: Option<String>,
    #[clap(value_name = "MANIFEST")]
    manifest_file: Option<String>,

    // graph builder flags (default mode)
    #[clap(long, default_value = "./manifests", help = "Base directory for searching manifests")]
    manifest_dir: String,

    #[clap(long, help = "Run the build script on the host's filesystem (for bootstrapping)")]
    bootstrap: bool,

    #[clap(long, help = "Skip runtime dependency scanning")]
    skip_runtime_deps: bool,

    #[clap(long, help = "Include per-reference explanations in runtime dependency output")]
    runtime_deps_verbose: bool,

    #[clap(long, help = "Treat missing files during runtime dependency scanning as warnings")]
    allow_missing_runtime_files: bool,

    #[clap(long, help = "Rewrite outputs.*.requires based on the runtime dependency scanner")]
    update_outputs_requires: bool,

    #[clap(long, help = "Update outputs.*.requires using existing OSTree outputs without rebuilding")]
    update_outputs_requires_only: bool,

    #[clap(long, help = "Validate build reproducibility")]
    validate_reproducibility: bool,

    #[clap(long, help = "Update the manifest checksum when build outputs differ from what is recorded")]
    update_checksum: bool,

    #[clap(long, help = "Rewrite OSTree output/bundle metadata without rebuilding (package manifests only)")]
    refresh_ostree_metadata: bool,

    #[clap(long, help = "Build only the specified manifest without dependencies")]
    single: bool,

    #[clap(long, help = "Show what would be built without actually building (dry run)")]
    dry_run: bool,

    #[clap(short = 'j', long, help = "Maximum number of parallel build jobs (default: number of CPUs)")]
    jobs: Option<usize>,

    #[clap(long, help = "Show dependency paths for all packages")]
    show_dep_paths: bool,

    #[clap(long, help = "Force rebuild even if package is already built")]
    force: bool,

    #[clap(long, help = "Add checksums to manifests missing them (from OSTree or by building)")]
    add_checksums: bool,

    #[clap(long, help = "Trace which packages pull in a specific dependency (e.g. 'bootstrap/phase1')")]
    trace_dependency: Option<String>,

    #[clap(long, help = "Specify build directory (default: ./build_rootfs_{slug}_{flavor})")]
    build_dir: Option<String>,

    #[clap(long, help = "Expand dependencies to include all transitive deps, ordered by depth")]
    hydrate_dependencies: bool,

    #[clap(long, help = "Include transitive runtime deps (requires) from manifest dependencies")]
    transitive_requires: bool,
}


pub struct Opts {
    repo_path: String,
    manifest_file: String,
    validate_reproducibility: bool,
    update_checksum: bool,
    bootstrap: bool,
    skip_runtime_deps: bool,
    runtime_deps_verbose: bool,
    allow_missing_runtime_files: bool,
    update_outputs_requires: bool,
    update_outputs_requires_only: bool,
    refresh_ostree_metadata: bool,
    force: bool,
    build_dir: Option<String>,
    transitive_requires: bool,
}

fn main() -> io::Result<()> {
    let cli: Cli = Cli::parse();

    let repo_path = cli.repo_path.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "REPO argument required")
    })?;
    let manifest_file = cli.manifest_file.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "MANIFEST argument required")
    })?;

    // configure rayon thread pool if jobs specified
    if let Some(num_jobs) = cli.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_jobs)
            .build_global()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to configure thread pool: {}", e)))?;
    }

    let manifest_path = Path::new(&manifest_file);
    let manifest_dirs = vec![PathBuf::from(&cli.manifest_dir)];

    let opts = Opts {
        repo_path: repo_path.clone(),
        manifest_file: manifest_file.clone(),
        validate_reproducibility: cli.validate_reproducibility,
        update_checksum: cli.update_checksum,
        bootstrap: cli.bootstrap,
        skip_runtime_deps: cli.skip_runtime_deps,
        runtime_deps_verbose: cli.runtime_deps_verbose,
        allow_missing_runtime_files: cli.allow_missing_runtime_files,
        update_outputs_requires: cli.update_outputs_requires,
        update_outputs_requires_only: cli.update_outputs_requires_only,
        refresh_ostree_metadata: cli.refresh_ostree_metadata,
        force: cli.force,
        build_dir: cli.build_dir,
        transitive_requires: cli.transitive_requires,
    };

    if cli.hydrate_dependencies {
        // hydrate mode: expand dependencies to include all transitive deps
        hydrate_dependencies(&repo_path, &manifest_file)
    } else if cli.single {
        // single mode: build only the specified manifest without dependencies
        build_single(&opts)
    } else {
        // default: build with full dependency resolution
        build_with_dependencies(&repo_path, manifest_path, &manifest_dirs, &opts, cli.dry_run, cli.add_checksums, cli.show_dep_paths, cli.force, cli.trace_dependency.as_deref())
    }
}

fn hydrate_dependencies(repo_path: &str, manifest_file: &str) -> io::Result<()> {
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

    // resolve all transitive dependencies (already in topological order, deepest first)
    let all_commits = resolve_dependency_closure(dependencies, repo_path)?;

    // convert commits to Dependency entries
    let hydrated_deps: Vec<Dependency> = all_commits
        .into_iter()
        .map(|commit| {
            // extract name from commit path like x86_64/zlib/1.3.1/sys/libs/bundles/dev
            let name = commit
                .split('/')
                .nth(1)
                .map(|s| s.to_string());
            Dependency { commit, name }
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
            if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('-') {
                dep_end = Some(i);
                break;
            }
        }
    }

    let dep_start = dep_start.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "No dependencies section found in manifest")
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

    println!("Hydrated {} dependencies (was {})", hydrated_deps.len(), dependencies.len());

    Ok(())
}

fn build_package_manifest(opts: &Opts, manifest: &mut Manifest) -> io::Result<()> {
    build_package_manifest_with_dir(opts, manifest, "./build_rootfs")
}

fn build_single(opts: &Opts) -> io::Result<()> {
    // load the manifest
    let manifest_data = load_manifest(&opts.manifest_file)?;

    // validate flags for refresh_ostree_metadata
    if opts.refresh_ostree_metadata {
        if matches!(manifest_data, ManifestData::System(_)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--refresh-ostree-metadata only applies to package manifests",
            ));
        }
        if opts.update_outputs_requires || opts.update_outputs_requires_only || opts.validate_reproducibility {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--refresh-ostree-metadata cannot be combined with build/update flags",
            ));
        }
    }

    match manifest_data {
        ManifestData::Package(mut manifest) => {
            if opts.refresh_ostree_metadata {
                refresh_package_metadata(&opts.repo_path, &manifest, Path::new(&opts.manifest_file))
            } else {
                println!("Building package: {}", manifest.package.slug);
                let build_dir = opts.build_dir.clone().unwrap_or_else(|| {
                    format!("./build_rootfs_{}_{}",
                        manifest.package.slug.replace("/", "_"),
                        manifest.package.flavor.replace("/", "_"))
                });
                build_package_manifest_with_dir(opts, &mut manifest, &build_dir)
            }
        }
        ManifestData::System(manifest) => {
            println!("Building system: {}", manifest.system.slug);
            let build_dir = opts.build_dir.clone().unwrap_or_else(|| {
                format!("./build_rootfs_{}_system",
                    manifest.system.slug.replace("/", "_"))
            });
            system::build_system_manifest_with_dir(opts, &manifest, &build_dir)
        }
    }
}

fn package_already_built(manifest: &Manifest, repo_path: &str) -> io::Result<bool> {
    use std::process::Command;

    // check if at least one output exists in ostree
    for category in manifest.outputs.keys() {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, category
        );

        let output = Command::new("ostree")
            .arg("refs")
            .arg("--repo")
            .arg(repo_path)
            .arg(&branch_name)
            .output()?;

        if output.status.success() && !output.stdout.is_empty() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn build_package_manifest_with_dir(opts: &Opts, manifest: &mut Manifest, base_dir: &str) -> io::Result<()> {
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    let dependency_commits = if opts.transitive_requires {
        resolve_dependency_closure(&manifest.dependencies, &opts.repo_path)?
    } else {
        // just use direct commits from dependencies, no transitive resolution
        manifest.dependencies.iter().map(|d| d.commit.clone()).collect()
    };
    let wants_update_outputs = opts.update_outputs_requires || opts.update_outputs_requires_only;
    let runtime_scanner = RuntimeScanner::new(&opts.repo_path, &dependency_commits)
        .with_allow_missing_files(opts.allow_missing_runtime_files);

    // check if package is already built and we can skip rebuilding
    let can_skip_rebuild = !opts.force && opts.update_outputs_requires && package_already_built(manifest, &opts.repo_path)?;

    if opts.update_outputs_requires_only || can_skip_rebuild {
        if can_skip_rebuild {
            println!("Package already built, updating requires without rebuilding");
        }
        stage_existing_outputs(manifest, base_dir, &opts.repo_path)?;
        let runtime_result = runtime_scanner.scan(
            &manifest.package.name,
            &manifest.package.version,
            base_dir,
            manifest,
            opts.runtime_deps_verbose,
        )?;
        update_manifest_outputs(&opts.manifest_file, &runtime_result)?;
        apply_runtime_requires(manifest, &runtime_result);
        println!(
            "Updated outputs.requires for {} without rebuilding",
            opts.manifest_file
        );
        return Ok(());
    }

    setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
    let input_env_vars = handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;

    let package_name = &manifest.package.name;
    let package_version = &manifest.package.version;
    let package_flavor = &manifest.package.flavor;

    println!(
        "Building {} {} for flavor {}",
        package_name, package_version, package_flavor
    );

    let mut env_vars = HashMap::new();
    env_vars.extend(input_env_vars);
    let build_script = manifest.build.script.clone();

    run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;

    let need_runtime_scan = !opts.skip_runtime_deps || wants_update_outputs;
    let runtime_analysis: Option<RuntimeScanResult> = if need_runtime_scan {
        Some(runtime_scanner.scan(
            &manifest.package.name,
            &manifest.package.version,
            base_dir,
            manifest,
            opts.runtime_deps_verbose,
        )?)
    } else {
        None
    };

    if wants_update_outputs {
        if let Some(result) = runtime_analysis.as_ref() {
            update_manifest_outputs(&opts.manifest_file, result)?;
            apply_runtime_requires(manifest, result);
        } else {
            println!("Skipping output requires update because runtime scanning was disabled.");
        }
    }

    let runtime_suggestions: Option<&RuntimeScanResult> = if opts.skip_runtime_deps {
        None
    } else {
        runtime_analysis.as_ref()
    };

    verify_and_commit_outputs(
        manifest,
        base_dir,
        &opts.repo_path,
        runtime_suggestions,
        opts.runtime_deps_verbose,
        Path::new(&opts.manifest_file),
    )?;

    create_and_commit_bundles(manifest, base_dir, &opts.repo_path, Path::new(&opts.manifest_file))?;

    println!("Build, packaging, and commit to OSTree completed for all outputs.");

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
                } else {
                    eprintln!(
                        "Checksum mismatch. Expected: {}, Calculated: {}",
                        expected_checksum, checksum
                    );
                    process::exit(-2);
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
            }
        }
    }

    if opts.validate_reproducibility {
        println!("Validating build reproducibility by building the package a second time.");
        fs::remove_dir_all(base_dir)?;
        setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
        handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;
        run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;
        verify_and_commit_outputs(
            manifest,
            base_dir,
            &opts.repo_path,
            runtime_suggestions,
            opts.runtime_deps_verbose,
            Path::new(&opts.manifest_file),
        )?;
        create_and_commit_bundles(manifest, base_dir, &opts.repo_path, Path::new(&opts.manifest_file))?;

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

fn refresh_package_metadata(repo_path: &str, manifest: &Manifest, manifest_path: &Path) -> io::Result<()> {
    println!(
        "Refreshing OSTree metadata for {}/{} ({})",
        manifest.package.slug, manifest.package.version, manifest.package.flavor
    );
    let manifest_hash = compute_manifest_hash(manifest_path)?;
    refresh_output_branches(repo_path, manifest, &manifest_hash)?;
    refresh_bundle_branches(repo_path, manifest, &manifest_hash)?;
    println!("Finished refreshing metadata for {}", manifest.package.slug);
    Ok(())
}

fn refresh_output_branches(repo_path: &str, manifest: &Manifest, manifest_hash: &str) -> io::Result<()> {
    for (category, spec) in &manifest.outputs {
        if category == "discard" {
            continue;
        }
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, category
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = output_branch_metadata(manifest, spec, manifest_hash)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}

fn refresh_bundle_branches(repo_path: &str, manifest: &Manifest, manifest_hash: &str) -> io::Result<()> {
    for (bundle_name, bundle) in &manifest.bundles {
        let branch_name = format!(
            "x86_64/{}/{}/{}/bundles/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, bundle_name
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = bundle_branch_metadata(manifest, bundle, manifest_hash)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}


// parse commit ref to extract slug, version, and flavor
// format: x86_64/{slug}/{version}/{flavor}/outputs/{output} or .../bundles/{bundle}

// find manifest file for a given commit reference
fn find_manifest_for_commit(
    commit: &str,
    manifest_dirs: &[PathBuf],
) -> io::Result<PathBuf> {
    // parse commit: x86_64/{slug}/{version}/{flavor}/...
    // flavor can be multi-part like "bootstrap/phase3"
    let parts: Vec<&str> = commit.split('/').collect();
    if parts.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid commit reference format: {}", commit),
        ));
    }

    let slug = parts[1];
    let _version = parts[2];
    // flavor is everything from parts[3] onwards until "outputs" or "bundles"
    let mut flavor_parts = vec![];
    for &part in &parts[3..] {
        if part == "outputs" || part == "bundles" {
            break;
        }
        flavor_parts.push(part);
    }
    let flavor = flavor_parts.join("/");

    // search in manifest directories
    for base_dir in manifest_dirs {
        let flavor_path = base_dir.join(&flavor);

        // try direct path: {flavor}/{slug}.yaml
        let direct_path = flavor_path.join(format!("{}.yaml", slug));
        if direct_path.exists() {
            return Ok(direct_path);
        }

        // try with -slug suffix: {flavor}/*-{slug}.yaml
        if flavor_path.exists() && flavor_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&flavor_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                            let filename = path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("");
                            if filename.ends_with(&format!("-{}", slug)) {
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Could not find manifest for commit {} (slug: {}, flavor: {})", commit, slug, flavor),
    ))
}

// compute SHA256 hash of manifest file
fn compute_manifest_hash(manifest_path: &Path) -> io::Result<String> {
    let contents = fs::read(manifest_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    Ok(format!("{:x}", hasher.finalize()))
}

// check if all outputs of a manifest are already built in OSTree with current manifest hash
fn check_if_built(repo_path: &str, manifest: &Manifest, manifest_path: &Path) -> io::Result<bool> {
    let arch = "x86_64"; // TODO: make configurable
    let slug = &manifest.package.slug;
    let version = &manifest.package.version;
    let flavor = &manifest.package.flavor;

    // compute current manifest hash
    let current_hash = compute_manifest_hash(manifest_path)?;

    // check all outputs
    for (output_name, _spec) in &manifest.outputs {
        let branch = format!("{}/{}/{}/{}/outputs/{}", arch, slug, version, flavor, output_name);

        // check if branch exists
        if ensure_branch_exists(repo_path, &branch).is_err() {
            return Ok(false);
        }

        // check if manifest hash matches
        match get_branch_metadata(repo_path, &branch, "nex.manifest.hash") {
            Ok(stored_hash) => {
                if stored_hash != current_hash {
                    println!("  Manifest {} has changed (hash mismatch), rebuilding", manifest_path.display());
                    return Ok(false);
                }
            }
            Err(_) => {
                // old commit without manifest hash metadata - rebuild to add it
                println!("  No manifest hash found in commit, rebuilding to add metadata");
                return Ok(false);
            }
        }
    }

    Ok(true)
}

// recursively collect dependencies and build graph
fn collect_dependencies_recursive(
    manifest_path: &Path,
    repo_path: &str,
    manifest_dirs: &[PathBuf],
    graph: &mut DiGraph<PathBuf, ()>,
    manifest_map: &mut HashMap<PathBuf, NodeIndex>,
    force: bool,
    ostree_cache: &mut HashMap<String, bool>,
) -> io::Result<NodeIndex> {
    // check if already processed
    if let Some(&node) = manifest_map.get(manifest_path) {
        return Ok(node);
    }

    // load manifest
    let manifest_data = load_manifest(manifest_path.to_str().unwrap())?;

    let (is_system, slug, dependencies) = match manifest_data {
        ManifestData::Package(ref m) => {
            (false, m.package.slug.clone(), m.dependencies.clone())
        }
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
            if check_if_built(repo_path, manifest, manifest_path)? {
                println!("Package {} already built, skipping", manifest.package.slug);
                let node = graph.add_node(PathBuf::new()); // empty path = skip
                manifest_map.insert(manifest_path.to_path_buf(), node);
                return Ok(node);
            }
        }
    }

    // add this manifest to graph
    let node = graph.add_node(manifest_path.to_path_buf());
    manifest_map.insert(manifest_path.to_path_buf(), node);

    println!("Processing dependencies for {}", slug);

    // process dependencies
    for dep in &dependencies {
        // check if dependency is already in OSTree (with caching)
        let in_ostree = ostree_cache.entry(dep.commit.clone()).or_insert_with(|| {
            ensure_branch_exists(repo_path, &dep.commit).is_ok()
        });

        if *in_ostree {
            println!("  Dependency {} already in OSTree, skipping", dep.commit);
            continue;
        }

        // find manifest for this dependency
        match find_manifest_for_commit(&dep.commit, manifest_dirs) {
            Ok(dep_manifest_path) => {
                println!("  Found dependency manifest: {}", dep_manifest_path.display());

                // recurse
                let dep_node = collect_dependencies_recursive(
                    &dep_manifest_path,
                    repo_path,
                    manifest_dirs,
                    graph,
                    manifest_map,
                    force,
                    ostree_cache,
                )?;

                // add edge: dep must be built before current
                // edge direction: dep_node -> node (dep comes before dependent)
                // only add edge if dep_node is a real node (not empty path marker)
                if graph[dep_node] != PathBuf::new() {
                    graph.add_edge(dep_node, node, ());
                }
            }
            Err(e) => {
                eprintln!("  Warning: Could not find manifest for dependency {}: {}", dep.commit, e);
                // continue anyway - might be a bootstrap dependency that's already built
            }
        }
    }

    Ok(node)
}

// show how packages would be built in parallel waves
fn show_parallel_execution_plan(
    graph: &DiGraph<PathBuf, ()>,
    build_order: &[NodeIndex],
    compact: bool,
) {
    let mut dependencies: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for &node in build_order {
        let deps: Vec<NodeIndex> = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .filter(|&dep| graph[dep] != PathBuf::new())
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

        let wave_names: Vec<String> = wave.iter()
            .map(|&node_idx| {
                let path = &graph[node_idx];
                let filename = path.file_name()
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
            println!("\n  Wave {}: {} package(s) in parallel", wave_num, wave.len());
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
    graph: &DiGraph<PathBuf, ()>,
    build_order: &[NodeIndex],
    opts: &Opts,
) -> io::Result<()> {
    let total = build_order.len();
    let completed = Arc::new(Mutex::new(HashSet::new()));
    let build_counter = Arc::new(Mutex::new(0usize));

    // create a map of node -> dependencies for quick lookup
    let mut dependencies: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for &node in build_order {
        let deps: Vec<NodeIndex> = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .filter(|&dep| graph[dep] != PathBuf::new())
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

        // bootstrap packages must build sequentially (they share ./build_rootfs directory)
        // if this wave contains bootstrap packages, only build one at a time
        let has_bootstrap = wave.iter().any(|&node_idx| {
            let path = &graph[node_idx];
            if let Ok(manifest_data) = load_manifest(path.to_str().unwrap()) {
                matches!(manifest_data, ManifestData::Package(m) if m.package.bootstrap)
            } else {
                false
            }
        });

        if has_bootstrap {
            // find the first bootstrap package and build only that one
            let bootstrap_idx = wave.iter().position(|&node_idx| {
                let path = &graph[node_idx];
                if let Ok(manifest_data) = load_manifest(path.to_str().unwrap()) {
                    matches!(manifest_data, ManifestData::Package(m) if m.package.bootstrap)
                } else {
                    false
                }
            }).unwrap();
            wave = vec![wave[bootstrap_idx]];
        }

        println!("Building wave of {} package(s) in parallel...", wave.len());

        // build all packages in this wave in parallel
        let results: Vec<Result<String, String>> = wave
            .par_iter()
            .map(|&node_idx| {
                let path = &graph[node_idx];
                let build_num = {
                    let mut counter = build_counter.lock().unwrap();
                    *counter += 1;
                    *counter
                };

                // load manifest
                let manifest_data = match load_manifest(path.to_str().unwrap()) {
                    Ok(data) => data,
                    Err(e) => return Err(format!("Failed to load {}: {}", path.display(), e)),
                };

                let result = match manifest_data {
                    ManifestData::Package(mut manifest) => {
                        // bootstrap packages must use fixed directory name so GCC's hardcoded sysroot path remains valid
                        let build_dir = if manifest.package.bootstrap {
                            "./build_rootfs".to_string()
                        } else {
                            format!("./build_rootfs_{}_{}",
                                manifest.package.slug.replace("/", "_"),
                                manifest.package.flavor.replace("/", "_"))
                        };

                        // create opts for this build
                        let build_opts = Opts {
                            repo_path: opts.repo_path.clone(),
                            manifest_file: path.to_str().unwrap().to_string(),
                            validate_reproducibility: opts.validate_reproducibility,
                            update_checksum: opts.update_checksum,
                            bootstrap: manifest.package.bootstrap,
                            skip_runtime_deps: opts.skip_runtime_deps,
                            runtime_deps_verbose: opts.runtime_deps_verbose,
                            allow_missing_runtime_files: opts.allow_missing_runtime_files,
                            update_outputs_requires: opts.update_outputs_requires,
                            update_outputs_requires_only: opts.update_outputs_requires_only,
                            refresh_ostree_metadata: opts.refresh_ostree_metadata,
                            force: opts.force,
                            build_dir: None,
                            transitive_requires: true,
                        };

                        println!("[{}/{}] Building: {}", build_num, total, manifest.package.slug);

                        // build the package
                        let slug = manifest.package.slug.clone();
                        if let Err(e) = build_package_manifest_with_dir(&build_opts, &mut manifest, &build_dir) {
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
                        let build_dir = format!("./build_rootfs_{}_{}",
                            system_manifest.system.slug.replace("/", "_"),
                            "system");

                        let build_opts = Opts {
                            repo_path: opts.repo_path.clone(),
                            manifest_file: path.to_str().unwrap().to_string(),
                            validate_reproducibility: opts.validate_reproducibility,
                            update_checksum: opts.update_checksum,
                            bootstrap: opts.bootstrap,
                            skip_runtime_deps: opts.skip_runtime_deps,
                            runtime_deps_verbose: opts.runtime_deps_verbose,
                            allow_missing_runtime_files: opts.allow_missing_runtime_files,
                            update_outputs_requires: opts.update_outputs_requires,
                            update_outputs_requires_only: opts.update_outputs_requires_only,
                            refresh_ostree_metadata: opts.refresh_ostree_metadata,
                            force: opts.force,
                            build_dir: None,
                            transitive_requires: true,
                        };

                        println!("[{}/{}] Building system: {}", build_num, total, system_manifest.system.slug);

                        // build the system using the legacy builder since it handles all the dependency resolution
                        let slug = system_manifest.system.slug.clone();
                        if let Err(e) = system::build_system_manifest_with_dir(&build_opts, &system_manifest, &build_dir) {
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
    graph: &DiGraph<PathBuf, ()>,
    manifest_map: &HashMap<PathBuf, NodeIndex>,
    root_path: &Path,
) {
    use petgraph::visit::Dfs;

    let root_node = manifest_map.get(root_path).expect("Root manifest should be in map");

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
                    let p = &graph[n];
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
    graph: &DiGraph<PathBuf, ()>,
    repo_path: &str,
    opts: &Opts,
) -> io::Result<()> {
    use crate::ostree::read_checksum_from_commit;
    use crate::manifest::update::update_manifest_checksum_field;
    use crate::manifest::ManifestKind;

    for &node_idx in build_order {
        let manifest_path = &graph[node_idx];
        let manifest_data = load_manifest(manifest_path.to_str().unwrap())?;

        match manifest_data {
            ManifestData::Package(manifest) => {
                // skip if already has checksum
                if manifest.package.checksum.is_some() {
                    continue;
                }

                println!("Processing: {}", manifest.package.slug);

                // try to get checksum from OSTree (check first bundle)
                if let Some((bundle_name, _)) = manifest.bundles.iter().next() {
                    let commit_ref = format!(
                        "x86_64/{}/{}/{}/bundles/{}",
                        manifest.package.slug,
                        manifest.package.version,
                        manifest.package.flavor,
                        bundle_name
                    );

                    match read_checksum_from_commit(repo_path, &commit_ref) {
                        Ok(checksum) => {
                            println!("  Found checksum in OSTree: {}", checksum);
                            update_manifest_checksum_field(
                                manifest_path.to_str().unwrap(),
                                ManifestKind::Package,
                                &checksum,
                            )?;
                            continue;
                        }
                        Err(_) => {
                            println!("  Not found in OSTree, building to get checksum...");
                        }
                    }
                }

                // build to get checksum
                let mut manifest_copy = manifest.clone();
                let build_opts = Opts {
                    repo_path: repo_path.to_string(),
                    manifest_file: manifest_path.to_str().unwrap().to_string(),
                    validate_reproducibility: false,
                    update_checksum: true, // enable checksum updating
                    bootstrap: manifest.package.bootstrap,
                    skip_runtime_deps: opts.skip_runtime_deps,
                    runtime_deps_verbose: opts.runtime_deps_verbose,
                    allow_missing_runtime_files: opts.allow_missing_runtime_files,
                    update_outputs_requires: opts.update_outputs_requires,
                    update_outputs_requires_only: false,
                    refresh_ostree_metadata: false,
                    force: false,
                    build_dir: None,
                    transitive_requires: true,
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

/// Trace and display dependency chains that include a specific pattern
fn trace_dependency_chains(
    repo_path: &str,
    manifest_path: &Path,
    _manifest_dirs: &[PathBuf],
    pattern: &str,
) -> io::Result<()> {
    println!("Tracing dependencies matching pattern: '{}'", pattern);
    println!("Starting from: {}\n", manifest_path.display());

    // load the root manifest
    let manifest_data = load_manifest(manifest_path.to_str().unwrap())?;
    let (root_slug, root_deps) = match manifest_data {
        ManifestData::Package(ref m) => {
            (m.package.slug.clone(), m.dependencies.clone())
        }
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
        if trace_commit_recursive(repo_path, &dep.commit, pattern, &mut chain)? {
            found_matches = true;
        }
    }

    if !found_matches {
        println!("No dependencies matching pattern '{}' found.", pattern);
    }

    Ok(())
}

/// Recursively trace a commit and its dependencies for a pattern
fn trace_commit_recursive(
    repo_path: &str,
    commit: &str,
    pattern: &str,
    chain: &mut Vec<String>,
) -> io::Result<bool> {
    // check if this commit matches the pattern
    let matches_pattern = commit.contains(pattern);

    // extract package name from commit for display
    let pkg_name = if let Some((slug, version, flavor)) = parse_commit_ref(commit) {
        format!("{}/{}/{}", slug, version, flavor)
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

    // fetch runtime dependencies from OSTree
    let requires = match fetch_requires_from_repo(repo_path, commit) {
        Ok(reqs) => reqs,
        Err(_) => {
            // commit might not exist in repo yet, skip it
            chain.pop();
            return Ok(false);
        }
    };

    // recursively check each dependency
    let mut found_in_subtree = false;
    for req in &requires {
        if trace_commit_recursive(repo_path, req, pattern, chain)? {
            found_in_subtree = true;
        }
    }

    chain.pop();
    Ok(found_in_subtree)
}

fn build_with_dependencies(
    repo_path: &str,
    manifest_path: &Path,
    manifest_dirs: &[PathBuf],
    opts: &Opts,
    dry_run: bool,
    add_checksums: bool,
    show_dep_paths: bool,
    force: bool,
    trace_dependency: Option<&str>,
) -> io::Result<()> {
    if dry_run {
        println!("DRY RUN: Analyzing dependency graph for {}", manifest_path.display());
    } else {
        println!("Building dependency graph for {}", manifest_path.display());
    }

    let mut graph = DiGraph::new();
    let mut manifest_map = HashMap::new();
    let mut ostree_cache = HashMap::new();

    // collect all dependencies recursively
    // when add_checksums or update_outputs_requires is true, treat it like force to include already-built packages
    collect_dependencies_recursive(
        manifest_path,
        repo_path,
        manifest_dirs,
        &mut graph,
        &mut manifest_map,
        force || add_checksums || opts.update_outputs_requires,
        &mut ostree_cache,
    )?;

    // if tracing dependencies, show all packages that pull in the traced pattern
    if let Some(pattern) = trace_dependency {
        trace_dependency_chains(repo_path, manifest_path, manifest_dirs, pattern)?;
        return Ok(());
    }

    // filter out empty path markers (already-built packages)
    let valid_nodes: Vec<NodeIndex> = graph
        .node_indices()
        .filter(|&idx| graph[idx] != PathBuf::new())
        .collect();

    if valid_nodes.is_empty() {
        println!("All packages already built!");
        return Ok(());
    }

    println!("\nDependency graph has {} packages to build", valid_nodes.len());

    // show dependency paths if requested
    if show_dep_paths {
        println!("\nDependency paths:");
        show_dependency_paths(&graph, &manifest_map, manifest_path);
    }

    // topological sort to get build order
    let build_order = toposort(&graph, None).map_err(|cycle| {
        let node_path = &graph[cycle.node_id()];
        io::Error::new(
            io::ErrorKind::Other,
            format!("Circular dependency detected at node {:?}: {}", cycle.node_id(), node_path.display()),
        )
    })?;

    // filter build order to only include valid nodes
    let build_order: Vec<NodeIndex> = build_order
        .into_iter()
        .filter(|&idx| graph[idx] != PathBuf::new())
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
        let path = &graph[node_idx];
        println!("  {}. {}", i + 1, path.display());
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
        write!(f, "{}/{}", self.flavor, self.slug)
    }
}












/// Verifies and commits outputs to OSTree branches based on the manifest.
///
/// This function takes the manifest, base directory, and repository path as input.
/// It verifies and commits the outputs specified in the manifest to the corresponding
/// OSTree branches. The function categorizes the output files, checks their existence,
/// moves them to the appropriate output directories, and commits them to the OSTree
/// repository.






















#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use tempfile::NamedTempFile;

    fn base_manifest() -> &'static str {
        r#"
package:
  schema: 1
  name: sample
  slug: sample
  flavor: bootstrap/phase0
  version: "1.0"
dependencies: []
sources: []
build:
  script: "true"
outputs: {}
"#
    }

    #[test]
    fn parses_detailed_bundle_with_metadata() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    includes:\n      - bin\n      - lib\n    requires:\n      - x86_64/foo/1.0/outputs/lib\n    suggests:\n      - x86_64/bar/2.0/bundles/dev\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
        assert_eq!(
            bundle.requires,
            vec!["x86_64/foo/1.0/outputs/lib".to_string()]
        );
        assert_eq!(
            bundle.suggests,
            vec!["x86_64/bar/2.0/bundles/dev".to_string()]
        );
    }

    #[test]
    fn parses_detailed_output_with_metadata() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen(
                "outputs: {}",
                "outputs:\n  bin:\n    files:\n      - /usr/bin/foo\n    requires:\n      - x86_64/libfoo/1.0/outputs/lib\n    suggests:\n      - x86_64/foo-doc/1.0/outputs/doc",
                1,
            )
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files, vec!["/usr/bin/foo".to_string()]);
        assert_eq!(
            output.requires,
            vec!["x86_64/libfoo/1.0/outputs/lib".to_string()]
        );
        assert_eq!(
            output.suggests,
            vec!["x86_64/foo-doc/1.0/outputs/doc".to_string()]
        );
    }

    #[test]
    fn parses_legacy_output_format() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen("outputs: {}", "outputs:\n  bin:\n    - /usr/bin/foo", 1)
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files, vec!["/usr/bin/foo".to_string()]);
        assert!(output.requires.is_empty());
        assert!(output.suggests.is_empty());
    }

    #[test]
    fn parses_legacy_bundle_format() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    - bin\n    - lib\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
        assert!(bundle.requires.is_empty());
        assert!(bundle.suggests.is_empty());
    }

    #[test]
    fn closure_resolution_collects_all_commits() {
        let deps = vec![
            Dependency {
                commit: "pkg/A".into(),
                name: None,
            },
            Dependency {
                commit: "pkg/D".into(),
                name: None,
            },
        ];
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        graph.insert("pkg/A", vec!["pkg/B", "pkg/C"]);
        graph.insert("pkg/B", vec!["pkg/C"]);
        graph.insert("pkg/C", vec!["pkg/D"]);
        graph.insert("pkg/D", vec![]);

        let result = resolve_dependency_closure_with_fetch(&deps, |commit| {
            Ok(graph
                .get(commit)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|s| s.to_string())
                .collect())
        })
        .unwrap();
        let expected: HashSet<String> = ["pkg/A", "pkg/B", "pkg/C", "pkg/D"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let actual: HashSet<String> = result.into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn closure_resolution_errors_on_cycle() {
        let deps = vec![Dependency {
            commit: "pkg/A".into(),
            name: None,
        }];
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        graph.insert("pkg/A", vec!["pkg/B"]);
        graph.insert("pkg/B", vec!["pkg/A"]);

        let err = resolve_dependency_closure_with_fetch(&deps, |commit| {
            Ok(graph
                .get(commit)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|s| s.to_string())
                .collect())
        })
        .expect_err("expected cycle to be detected");
        assert!(
            err.to_string().contains("pkg/A -> pkg/B -> pkg/A"),
            "unexpected error message: {}",
            err
        );
    }

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
