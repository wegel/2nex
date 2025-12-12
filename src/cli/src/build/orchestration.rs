//! Build orchestration: dependency graph construction and parallel execution

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use rayon::prelude::*;

use crate::build::{check_if_built, compute_manifest_hash};
use crate::commands::build::BuildOpts;
use crate::deps::*;
use crate::manifest::types::{Dependency, Manifest, ManifestSource};
use crate::manifest::*;
use crate::store::*;
use crate::system;

/// Get the build directory for a package (unique per package for parallel builds)
pub fn get_build_dir_for_package(manifest: &Manifest) -> String {
    format!(
        ".nex/tmp/build_rootfs_{}_{}",
        manifest.package.slug.replace("/", "_"),
        manifest.package.namespace.replace("/", "_")
    )
}

/// Find manifest file for a given commit reference
pub fn find_manifest_for_commit(commit: &str, manifest_dirs: &[PathBuf]) -> io::Result<PathBuf> {
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
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
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
        format!(
            "Could not find manifest for commit {} (slug: {}, namespace: {})",
            commit, slug, namespace
        ),
    ))
}

/// Hydrate dependencies in a manifest file by resolving transitive dependencies
pub fn hydrate_dependencies(_repo_path: &str, manifest_file: &str) -> io::Result<()> {
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

    let manifest_index = ManifestIndex::load("pkg")?;
    let all_commits = resolve_dependency_closure(dependencies, &manifest_index)?;

    let hydrated_deps: Vec<Dependency> = all_commits
        .into_iter()
        .map(|commit| {
            let name = commit.split('/').nth(1).map(|s| s.to_string());
            Dependency {
                commit,
                name,
                manifest_ref: None,
            }
        })
        .collect();

    let content = fs::read_to_string(manifest_file)?;
    let lines: Vec<&str> = content.lines().collect();
    let mut dep_start = None;
    let mut dep_end = None;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("dependencies:") {
            dep_start = Some(i);
        } else if dep_start.is_some()
            && dep_end.is_none()
            && !line.is_empty()
            && !line.starts_with(' ')
            && !line.starts_with('\t')
            && !line.starts_with('-')
        {
            dep_end = Some(i);
            break;
        }
    }

    let dep_start = dep_start.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No dependencies section found in manifest",
        )
    })?;
    let dep_end = dep_end.unwrap_or(lines.len());

    let mut new_dep_section = vec!["dependencies:".to_string()];
    for dep in &hydrated_deps {
        if let Some(name) = &dep.name {
            new_dep_section.push(format!("  - name: {}", name));
            new_dep_section.push(format!("    commit: {}", dep.commit));
        } else {
            new_dep_section.push(format!("  - commit: {}", dep.commit));
        }
    }

    let mut result: Vec<String> = lines[..dep_start].iter().map(|s| s.to_string()).collect();
    result.extend(new_dep_section);
    result.extend(lines[dep_end..].iter().map(|s| s.to_string()));

    let output = result.join("\n");
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

/// Recursively collect dependencies and build graph
pub fn collect_dependencies_recursive(
    manifest_source: &ManifestSource,
    repo_path: &str,
    manifest_dirs: &[PathBuf],
    graph: &mut DiGraph<ManifestSource, ()>,
    manifest_map: &mut HashMap<PathBuf, NodeIndex>,
    force: bool,
    ref_cache: &mut HashMap<String, bool>,
    store: Option<&Store>,
) -> io::Result<NodeIndex> {
    let manifest_path = manifest_source.path();

    if let Some(&node) = manifest_map.get(manifest_path) {
        return Ok(node);
    }

    let manifest_data = load_manifest_from_source(manifest_source)?;

    let (is_system, slug, dependencies) = match manifest_data {
        ManifestData::Package(ref m) => (false, m.package.slug.clone(), m.dependencies.clone()),
        ManifestData::System(ref s) => {
            let mut all_deps = s.dependencies.clone();
            all_deps.extend(system::dependencies_from_system_packages(&s.packages));
            (true, s.system.slug.clone(), all_deps)
        }
    };

    if !is_system && !force {
        if let ManifestData::Package(ref manifest) = manifest_data {
            if check_if_built(repo_path, manifest, manifest_path)?.is_some() {
                println!("Package {} already built, skipping", manifest.package.slug);
                let node = graph.add_node(ManifestSource::Skip);
                manifest_map.insert(manifest_path.to_path_buf(), node);
                return Ok(node);
            }
        }
    }

    let node = graph.add_node(manifest_source.clone());
    manifest_map.insert(manifest_path.to_path_buf(), node);

    println!("Processing dependencies for {}", slug);

    let git_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for dep in &dependencies {
        if let Some(ref blob_sha) = dep.manifest_ref {
            // ensure ref exists locally (try remote pull if needed)
            let ref_available = ref_cache.entry(dep.commit.clone()).or_insert_with(|| {
                if ensure_branch_exists(repo_path, &dep.commit).is_ok() {
                    return true;
                }
                if let Some(s) = store {
                    if s.pull_from_remote(&dep.commit).unwrap_or(false) {
                        return true;
                    }
                }
                false
            });

            if !*ref_available {
                // ref not available locally or remotely, needs build
                println!(
                    "  [needs build] {} (pinned to {}, not in store)",
                    dep.commit,
                    &blob_sha[..12]
                );
            } else {
                match crate::utils::fetch_git_blob(&git_root, blob_sha) {
                    Ok(content) => {
                        use sha2::{Digest, Sha256};
                        let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

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
            }
        } else {
            // for refs with internal paths (e.g., files/usr/bin/ldd), check the base ref
            use crate::refs::PackageRef;
            let ref_to_check = if let Ok(pkg_ref) = PackageRef::parse(&dep.commit) {
                if pkg_ref.has_internal_path() {
                    pkg_ref.commit_ref()
                } else {
                    dep.commit.clone()
                }
            } else {
                dep.commit.clone()
            };

            let branch_exists = ref_cache.entry(ref_to_check.clone()).or_insert_with(|| {
                if ensure_branch_exists(repo_path, &ref_to_check).is_ok() {
                    return true;
                }
                // try pulling from remote if not found locally
                if let Some(s) = store {
                    if s.pull_from_remote(&ref_to_check).unwrap_or(false) {
                        return true;
                    }
                }
                false
            });

            if *branch_exists {
                match find_manifest_for_commit(&dep.commit, manifest_dirs) {
                    Ok(dep_manifest_path) => {
                        let manifest_hash = compute_manifest_hash(&dep_manifest_path)?;
                        // use base ref for manifest hash lookup (strips internal paths)
                        match find_commit_by_manifest_hash(repo_path, &ref_to_check, &manifest_hash) {
                            Ok(Some(_)) => {
                                println!("  [floating] {} skipping", dep.commit);
                                continue;
                            }
                            Ok(None) => {
                                println!(
                                    "  [floating/stale] {} manifest changed, rebuilding",
                                    dep.commit
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "  Warning: failed to check manifest hash for {}: {}",
                                    dep.commit, e
                                );
                            }
                        }
                    }
                    Err(_) => {
                        println!("  [floating] {} skipping (no manifest found)", dep.commit);
                        continue;
                    }
                }
            }
        }

        match find_manifest_for_commit(&dep.commit, manifest_dirs) {
            Ok(dep_manifest_path) => {
                if manifest_map.contains_key(&dep_manifest_path) {
                    let dep_node = manifest_map[&dep_manifest_path];
                    if !graph[dep_node].is_skip() {
                        graph.add_edge(dep_node, node, ());
                    }
                    continue;
                }

                println!(
                    "  Found dependency manifest: {}",
                    dep_manifest_path.display()
                );

                let dep_source = if let Some(ref blob_sha) = dep.manifest_ref {
                    ManifestSource::Blob {
                        sha: blob_sha.clone(),
                        path: dep_manifest_path,
                    }
                } else {
                    ManifestSource::Path(dep_manifest_path)
                };

                let dep_node = collect_dependencies_recursive(
                    &dep_source,
                    repo_path,
                    manifest_dirs,
                    graph,
                    manifest_map,
                    force,
                    ref_cache,
                    store,
                )?;

                if !graph[dep_node].is_skip() {
                    graph.add_edge(dep_node, node, ());
                }
            }
            Err(e) => {
                eprintln!(
                    "  Warning: Could not find manifest for dependency {}: {}",
                    dep.commit, e
                );
            }
        }
    }

    Ok(node)
}

/// Show how packages would be built in parallel waves
fn show_parallel_execution_plan(
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
    compact: bool,
) {
    let mut dependencies: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for &node in build_order {
        let deps: Vec<NodeIndex> = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .filter(|&dep| !graph[dep].is_skip())
            .collect();
        dependencies.insert(node, deps);
    }

    let mut remaining: Vec<NodeIndex> = build_order.to_vec();
    let mut completed = HashSet::new();
    let mut wave_num = 0;

    while !remaining.is_empty() {
        wave_num += 1;

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

        for &node in &wave {
            completed.insert(node);
        }

        remaining.retain(|node| !completed.contains(node));
    }
}

/// Build packages in parallel waves
fn build_packages_parallel(
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
    opts: &BuildOpts,
) -> io::Result<()> {
    fs::create_dir_all(".nex/tmp")?;

    let multi_progress = Arc::new(indicatif::MultiProgress::new());

    let total = build_order.len();
    let completed = Arc::new(Mutex::new(HashSet::new()));
    let build_counter = Arc::new(Mutex::new(0usize));

    let mut dependencies: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for &node in build_order {
        let deps: Vec<NodeIndex> = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .filter(|&dep| !graph[dep].is_skip())
            .collect();
        dependencies.insert(node, deps);
    }

    let mut remaining: Vec<NodeIndex> = build_order.to_vec();

    while !remaining.is_empty() {
        let wave: Vec<NodeIndex> = remaining
            .iter()
            .filter(|&&node| {
                let deps = &dependencies[&node];
                let completed_set = completed.lock().unwrap();
                deps.iter().all(|dep| completed_set.contains(dep))
            })
            .cloned()
            .collect();

        if wave.is_empty() {
            return Err(io::Error::other(
                "Cannot make progress: all remaining packages have unmet dependencies",
            ));
        }

        println!("Building wave of {} package(s) in parallel...", wave.len());

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

                let manifest_data = match load_manifest_from_source(source) {
                    Ok(data) => data,
                    Err(e) => return Err(format!("Failed to load {}: {}", path.display(), e)),
                };

                let result = match manifest_data {
                    ManifestData::Package(mut manifest) => {
                        let build_dir = get_build_dir_for_package(&manifest);

                        let build_opts = BuildOpts {
                            repo_path: opts.repo_path.clone(),
                            manifest_file: path.to_str().unwrap().to_string(),
                            check: opts.check,
                            update_checksum: opts.update_checksum,
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
                            dry_run: opts.dry_run,
                            add_checksums: opts.add_checksums,
                            show_dep_paths: opts.show_dep_paths,
                            trace_dependency: opts.trace_dependency.clone(),
                            multi_progress: Some(multi_progress.clone()),
                        };

                        println!(
                            "[{}/{}] Building: {}",
                            build_num, total, manifest.package.slug
                        );

                        let slug = manifest.package.slug.clone();
                        if let Err(e) = crate::build::build_package_manifest_with_dir(
                            &build_opts,
                            &mut manifest,
                            &build_dir,
                        ) {
                            return Err(format!("Failed to build {}: {}", slug, e));
                        }

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
                            dry_run: opts.dry_run,
                            add_checksums: opts.add_checksums,
                            show_dep_paths: opts.show_dep_paths,
                            trace_dependency: opts.trace_dependency.clone(),
                            multi_progress: Some(multi_progress.clone()),
                        };

                        println!(
                            "[{}/{}] Building system: {}",
                            build_num, total, system_manifest.system.slug
                        );

                        let slug = system_manifest.system.slug.clone();
                        if let Err(e) = system::build_system_manifest_with_dir(
                            &build_opts,
                            &system_manifest,
                            &build_dir,
                        ) {
                            return Err(format!("Failed to build system {}: {}", slug, e));
                        }

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

        for (i, result) in results.iter().enumerate() {
            match result {
                Ok(slug) => {
                    println!("✓ Successfully built {}", slug);
                    completed.lock().unwrap().insert(wave[i]);
                }
                Err(e) => {
                    return Err(io::Error::other(e.clone()));
                }
            }
        }

        remaining.retain(|node| !completed.lock().unwrap().contains(node));
    }

    Ok(())
}

/// Show dependency paths from root to each node
fn show_dependency_paths(
    graph: &DiGraph<ManifestSource, ()>,
    manifest_map: &HashMap<PathBuf, NodeIndex>,
    root_path: &Path,
) {
    use petgraph::visit::Dfs;

    let root_node = manifest_map
        .get(root_path)
        .expect("Root manifest should be in map");

    for (path, &node) in manifest_map.iter() {
        if path == root_path || graph[node].is_skip() {
            continue;
        }

        let mut dfs = Dfs::new(graph, *root_node);
        let mut parent_map: HashMap<NodeIndex, Option<NodeIndex>> = HashMap::new();
        parent_map.insert(*root_node, None);

        while let Some(current) = dfs.next(graph) {
            if current == node {
                let mut path_nodes = vec![current];
                let mut cur = current;
                while let Some(Some(parent)) = parent_map.get(&cur) {
                    path_nodes.push(*parent);
                    cur = *parent;
                }
                path_nodes.reverse();

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

            for neighbor in graph.neighbors(current) {
                parent_map.entry(neighbor).or_insert(Some(current));
            }
        }
    }
}

/// Add missing checksums to manifests
fn add_missing_checksums_to_manifests(
    build_order: &[NodeIndex],
    graph: &DiGraph<ManifestSource, ()>,
    repo_path: &str,
    opts: &BuildOpts,
) -> io::Result<()> {
    use crate::manifest::update::update_manifest_checksum_field;
    use crate::store::get_branch_metadata;

    for &node_idx in build_order {
        let source = &graph[node_idx];
        let manifest_path = source.path();
        let manifest_data = load_manifest_from_source(source)?;

        match manifest_data {
            ManifestData::Package(manifest) => {
                if manifest.package.checksum.is_some() {
                    continue;
                }

                println!("Processing: {}", manifest.package.slug);

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

                let mut manifest_copy = manifest.clone();
                let build_dir = get_build_dir_for_package(&manifest);
                let build_opts = BuildOpts {
                    repo_path: repo_path.to_string(),
                    manifest_file: manifest_path.to_str().unwrap().to_string(),
                    check: false,
                    update_checksum: true,
                    compute_deps: opts.compute_deps,
                    runtime_deps_verbose: opts.runtime_deps_verbose,
                    refresh_metadata: false,
                    force: false,
                    build_dir: None,
                    generate_outputs: false,
                    fallback_repos: opts.fallback_repos.clone(),
                    verbose: opts.verbose,
                    record_profile: opts.record_profile,
                    no_progress: opts.no_progress,
                    dry_run: false,
                    add_checksums: false,
                    show_dep_paths: false,
                    trace_dependency: None,
                    multi_progress: None,
                };

                crate::build::build_package_manifest_with_dir(
                    &build_opts,
                    &mut manifest_copy,
                    &build_dir,
                )?;
                println!("  Built and checksummed");
            }
            ManifestData::System(_) => {
                continue;
            }
        }
    }

    Ok(())
}

/// Trace and display dependency chains that include a specific pattern
fn trace_dependency_chains(
    _repo_path: &str,
    manifest_path: &Path,
    _manifest_dirs: &[PathBuf],
    pattern: &str,
) -> io::Result<()> {
    println!("Tracing dependencies matching pattern: '{}'", pattern);
    println!("Starting from: {}\n", manifest_path.display());

    let manifest_index = ManifestIndex::load("pkg")?;

    let manifest_data = load_manifest(manifest_path.to_str().unwrap())?;
    let (root_slug, root_deps) = match manifest_data {
        ManifestData::Package(ref m) => (m.package.slug.clone(), m.dependencies.clone()),
        ManifestData::System(ref s) => {
            let mut all_deps = s.dependencies.clone();
            all_deps.extend(system::dependencies_from_system_packages(&s.packages));
            (s.system.slug.clone(), all_deps)
        }
    };

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

/// Recursively trace a commit and its dependencies for a pattern
fn trace_commit_recursive(
    commit: &str,
    pattern: &str,
    chain: &mut Vec<String>,
    manifest_index: &ManifestIndex,
) -> io::Result<bool> {
    let matches_pattern = commit.contains(pattern);

    let pkg_name = if let Ok(pkg_ref) = crate::refs::PackageRef::parse(commit) {
        format!("{}/{}/{}", pkg_ref.namespace, pkg_ref.slug, pkg_ref.version)
    } else {
        commit.to_string()
    };

    chain.push(pkg_name.clone());

    if matches_pattern {
        println!("Found match: {}", commit);
        println!("  Chain: {}", chain.join(" → "));
        println!();
        chain.pop();
        return Ok(true);
    }

    let deps = get_manifest_deps(commit, manifest_index);

    let mut found_in_subtree = false;
    for dep_commit in &deps {
        if trace_commit_recursive(dep_commit, pattern, chain, manifest_index)? {
            found_in_subtree = true;
        }
    }

    chain.pop();
    Ok(found_in_subtree)
}

/// Get runtime dependencies for a commit from its manifest
fn get_manifest_deps(commit: &str, manifest_index: &ManifestIndex) -> Vec<String> {
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

/// Pin all dependencies in a manifest to their current git blob SHAs
pub fn link_manifest_dependencies(manifest_file: &str) -> io::Result<()> {
    use crate::utils::hash_file_content;

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

    let manifest_dirs = vec![PathBuf::from(".")];

    let original_content = fs::read_to_string(manifest_file)?;

    let mut updated_content = original_content.clone();
    let mut linked_count = 0;

    for dep in &dependencies {
        match find_manifest_for_commit(&dep.commit, &manifest_dirs) {
            Ok(manifest_path) => {
                let blob_sha = hash_file_content(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), blob_sha);

                let commit_pattern = format!("commit: {}", dep.commit);
                if let Some(pos) = updated_content.find(&commit_pattern) {
                    let after_commit = pos + commit_pattern.len();
                    let next_section = updated_content[after_commit..]
                        .find("\n  - ")
                        .or_else(|| updated_content[after_commit..].find("\nsources:"))
                        .or_else(|| updated_content[after_commit..].find("\npackages:"))
                        .unwrap_or(updated_content.len() - after_commit);

                    let entry_section = &updated_content[pos..after_commit + next_section];
                    if !entry_section.contains("manifest_ref:") {
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

    for pkg in &packages {
        match find_manifest_for_commit(&pkg.commit, &manifest_dirs) {
            Ok(manifest_path) => {
                let blob_sha = hash_file_content(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), blob_sha);
            }
            Err(e) => {
                eprintln!("  Warning: {}: {}", pkg.commit, e);
            }
        }
    }

    let env_ref = match &manifest_data {
        ManifestData::Package(m) => Some(m.build.environment.clone()),
        ManifestData::System(s) => Some(s.build.environment.clone()),
    };

    if let Some(env) = env_ref {
        if !env.chars().all(|c| c.is_ascii_hexdigit()) || env.len() != 40 {
            let env_path = Path::new(&env);
            if env_path.exists() {
                let git_check = std::process::Command::new("git")
                    .args(["ls-files", &env])
                    .output()?;
                let is_tracked = !String::from_utf8_lossy(&git_check.stdout).trim().is_empty();

                if !is_tracked {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "Environment file '{}' is not tracked by git. Run 'git add {}' first.",
                            env, env
                        ),
                    ));
                }

                let env_sha = hash_file_content(env_path)?;
                println!("  environment: {} -> {}", env, env_sha);

                let patterns = [
                    format!("environment: {}", env),
                    format!("environment: '{}'", env),
                    format!("environment: \"{}\"", env),
                ];

                for pattern in &patterns {
                    if updated_content.contains(pattern) {
                        updated_content =
                            updated_content.replace(pattern, &format!("environment: {}", env_sha));
                        linked_count += 1;
                        break;
                    }
                }
            } else {
                eprintln!("  Warning: environment file not found: {}", env);
            }
        }
    }

    if linked_count > 0 {
        fs::write(manifest_file, updated_content)?;
        println!("\nLinked {} references in {}", linked_count, manifest_file);
    } else {
        println!("\nNo references to link or all already linked");
    }

    Ok(())
}

/// Build a manifest and all its missing dependencies
pub fn build_with_dependencies(
    manifest_path: &Path,
    manifest_dirs: &[PathBuf],
    opts: &BuildOpts,
) -> io::Result<()> {
    if opts.dry_run {
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

    // open store with remotes for pulling missing refs during planning
    let fallback_paths: Vec<PathBuf> = opts.fallback_repos.iter().map(PathBuf::from).collect();
    let store = Store::open_with_fallback_chain(&opts.repo_path, &fallback_paths).ok();

    let root_source = ManifestSource::Path(manifest_path.to_path_buf());
    collect_dependencies_recursive(
        &root_source,
        &opts.repo_path,
        manifest_dirs,
        &mut graph,
        &mut manifest_map,
        opts.force || opts.add_checksums,
        &mut ref_cache,
        store.as_ref(),
    )?;

    if let Some(ref pattern) = opts.trace_dependency {
        trace_dependency_chains(&opts.repo_path, manifest_path, manifest_dirs, pattern)?;
        return Ok(());
    }

    let valid_nodes: Vec<NodeIndex> = graph
        .node_indices()
        .filter(|&idx| !graph[idx].is_skip())
        .collect();

    if valid_nodes.is_empty() {
        println!("All packages already built!");
        return Ok(());
    }

    println!(
        "\nDependency graph has {} packages to build",
        valid_nodes.len()
    );

    if opts.show_dep_paths {
        println!("\nDependency paths:");
        show_dependency_paths(&graph, &manifest_map, manifest_path);
    }

    let build_order = toposort(&graph, None).map_err(|cycle| {
        let node_source = &graph[cycle.node_id()];
        io::Error::other(format!(
            "Circular dependency detected at node {:?}: {}",
            cycle.node_id(),
            node_source.path().display()
        ))
    })?;

    let build_order: Vec<NodeIndex> = build_order
        .into_iter()
        .filter(|&idx| !graph[idx].is_skip())
        .collect();

    if opts.add_checksums {
        println!("\nAdding missing checksums...");
        add_missing_checksums_to_manifests(&build_order, &graph, &opts.repo_path, opts)?;
        println!("Checksums updated!");
        return Ok(());
    }

    if opts.dry_run {
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

    println!("\nStarting parallel builds...\n");
    build_packages_parallel(&graph, &build_order, opts)?;

    println!("==================================================");
    println!("All packages built successfully!");
    println!("==================================================");

    Ok(())
}
