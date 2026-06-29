//! Parallel build planning and execution for dependency graphs.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use petgraph::graph::{DiGraph, NodeIndex};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::commands::build::BuildOpts;
use crate::manifest::types::ManifestSource;
use crate::manifest::{load_manifest_from_source, ManifestData};
use crate::system;

use super::manifest_lookup::get_build_dir_for_package;

/// Show how packages would be built in parallel waves.
pub fn show_parallel_execution_plan(
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
    compact: bool,
) {
    let dependencies = dependency_map(graph, build_order);
    let mut remaining = build_order.to_vec();
    let mut completed = HashSet::new();
    let mut wave_num = 0;

    while !remaining.is_empty() {
        wave_num += 1;
        let wave = ready_wave(&remaining, &dependencies, &completed);
        if wave.is_empty() {
            println!("\nERROR: Cannot make progress - circular dependency detected");
            break;
        }

        let wave_names = wave_names(graph, &wave);
        print_wave(wave_num, &wave_names, compact);
        completed.extend(wave.iter().copied());
        remaining.retain(|node| !completed.contains(node));
    }
}

/// Build packages in parallel waves.
pub fn build_packages_parallel(
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
    opts: &BuildOpts,
) -> io::Result<()> {
    fs::create_dir_all(".nex/tmp")?;

    let dependencies = dependency_map(graph, build_order);
    let mut remaining = build_order.to_vec();
    let state = ParallelBuildState::new(build_order.len());

    while !remaining.is_empty() {
        let wave = state.ready_wave(&remaining, &dependencies)?;
        println!("Building wave of {} package(s) in parallel...", wave.len());
        state.build_wave(graph, opts, &wave)?;
        remaining.retain(|node| !state.has_completed(*node));
    }

    Ok(())
}

struct ParallelBuildState {
    multi_progress: Arc<indicatif::MultiProgress>,
    total: usize,
    completed: Arc<Mutex<HashSet<NodeIndex>>>,
    build_counter: Arc<Mutex<usize>>,
}

impl ParallelBuildState {
    fn new(total: usize) -> Self {
        Self {
            multi_progress: Arc::new(indicatif::MultiProgress::new()),
            total,
            completed: Arc::new(Mutex::new(HashSet::new())),
            build_counter: Arc::new(Mutex::new(0)),
        }
    }

    fn ready_wave(
        &self,
        remaining: &[NodeIndex],
        dependencies: &HashMap<NodeIndex, Vec<NodeIndex>>,
    ) -> io::Result<Vec<NodeIndex>> {
        let completed = self
            .completed
            .lock()
            .expect("build scheduler completion set should not be poisoned");
        let wave = ready_wave(remaining, dependencies, &completed);
        if wave.is_empty() {
            Err(io::Error::other(
                "Cannot make progress: all remaining packages have unmet dependencies",
            ))
        } else {
            Ok(wave)
        }
    }

    fn build_wave(
        &self,
        graph: &DiGraph<ManifestSource, ()>,
        opts: &BuildOpts,
        wave: &[NodeIndex],
    ) -> io::Result<()> {
        let results = wave
            .par_iter()
            .map(|&node_idx| self.build_node(graph, opts, node_idx))
            .collect::<Vec<_>>();
        self.record_wave_results(wave, &results)
    }

    fn build_node(
        &self,
        graph: &DiGraph<ManifestSource, ()>,
        opts: &BuildOpts,
        node_idx: NodeIndex,
    ) -> Result<String, String> {
        let source = &graph[node_idx];
        let path = source.path();
        let build_num = self.next_build_number();
        let manifest_data = load_manifest_from_source(source)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;

        match manifest_data {
            ManifestData::Package(mut manifest) => {
                self.build_package(opts, path, build_num, &mut manifest)
            }
            ManifestData::System(system_manifest) => {
                self.build_system(opts, path, build_num, &system_manifest)
            }
        }
    }

    fn next_build_number(&self) -> usize {
        let mut counter = self
            .build_counter
            .lock()
            .expect("build scheduler counter should not be poisoned");
        *counter += 1;
        *counter
    }

    fn build_package(
        &self,
        opts: &BuildOpts,
        path: &Path,
        build_num: usize,
        manifest: &mut crate::manifest::types::Manifest,
    ) -> Result<String, String> {
        let build_dir = get_build_dir_for_package(manifest);
        let build_opts = build_opts_for_node(opts, path, self.multi_progress.clone());
        println!(
            "[{}/{}] Building: {}",
            build_num, self.total, manifest.package.slug
        );

        let slug = manifest.package.slug.clone();
        crate::build::build_package_manifest_with_dir(&build_opts, manifest, &build_dir)
            .map_err(|e| format!("Failed to build {}: {}", slug, e))?;
        cleanup_build_dir(&build_dir);
        Ok(slug)
    }

    fn build_system(
        &self,
        opts: &BuildOpts,
        path: &Path,
        build_num: usize,
        manifest: &crate::manifest::types::SystemManifest,
    ) -> Result<String, String> {
        let build_dir = format!(
            ".nex/tmp/build_rootfs_{}_system",
            manifest.system.slug.replace("/", "_")
        );
        let build_opts = build_opts_for_node(opts, path, self.multi_progress.clone());
        println!(
            "[{}/{}] Building system: {}",
            build_num, self.total, manifest.system.slug
        );

        let slug = manifest.system.slug.clone();
        system::build_system_manifest_with_dir(&build_opts, manifest, &build_dir)
            .map_err(|e| format!("Failed to build system {}: {}", slug, e))?;
        cleanup_build_dir(&build_dir);
        Ok(slug)
    }

    fn record_wave_results(
        &self,
        wave: &[NodeIndex],
        results: &[Result<String, String>],
    ) -> io::Result<()> {
        for (index, result) in results.iter().enumerate() {
            match result {
                Ok(slug) => {
                    println!("Successfully built {}", slug);
                    self.completed
                        .lock()
                        .expect("build scheduler completion set should not be poisoned")
                        .insert(wave[index]);
                }
                Err(e) => return Err(io::Error::other(e.clone())),
            }
        }
        Ok(())
    }

    fn has_completed(&self, node: NodeIndex) -> bool {
        self.completed
            .lock()
            .expect("build scheduler completion set should not be poisoned")
            .contains(&node)
    }
}

fn dependency_map(
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
) -> HashMap<NodeIndex, Vec<NodeIndex>> {
    build_order
        .iter()
        .map(|&node| {
            let deps = graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .filter(|&dep| !graph[dep].is_skip())
                .collect();
            (node, deps)
        })
        .collect()
}

fn ready_wave(
    remaining: &[NodeIndex],
    dependencies: &HashMap<NodeIndex, Vec<NodeIndex>>,
    completed: &HashSet<NodeIndex>,
) -> Vec<NodeIndex> {
    remaining
        .iter()
        .filter(|&&node| {
            dependencies[&node]
                .iter()
                .all(|dep| completed.contains(dep))
        })
        .copied()
        .collect()
}

fn wave_names(graph: &DiGraph<ManifestSource, ()>, wave: &[NodeIndex]) -> Vec<String> {
    wave.iter()
        .map(|&node_idx| {
            graph[node_idx]
                .path()
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown")
                .trim_end_matches(".yaml")
                .to_string()
        })
        .collect()
}

fn print_wave(wave_num: usize, wave_names: &[String], compact: bool) {
    if compact {
        println!(
            "  Wave {} ({}): {}",
            wave_num,
            wave_names.len(),
            wave_names.join(", ")
        );
    } else {
        println!(
            "\n  Wave {}: {} package(s) in parallel",
            wave_num,
            wave_names.len()
        );
        for name in wave_names {
            println!("    - {}", name);
        }
    }
}

fn build_opts_for_node(
    opts: &BuildOpts,
    path: &Path,
    multi_progress: Arc<indicatif::MultiProgress>,
) -> BuildOpts {
    BuildOpts {
        repo_path: opts.repo_path.clone(),
        manifest_file: path.to_string_lossy().to_string(),
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
        multi_progress: Some(multi_progress),
        reuse_rootfs: opts.reuse_rootfs,
    }
}

fn cleanup_build_dir(build_dir: &str) {
    let build_path = Path::new(build_dir);
    if build_path.exists() {
        if let Err(e) = fs::remove_dir_all(build_path) {
            eprintln!("Warning: failed to clean up {}: {}", build_dir, e);
        }
    }
}
