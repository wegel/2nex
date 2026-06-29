//! Build orchestration: dependency graph construction and parallel execution

use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;

use crate::commands::build::BuildOpts;
use crate::manifest::types::ManifestSource;
use crate::store::Store;

pub mod checksums;
pub mod graph;
pub mod hydrate;
pub mod link;
pub mod manifest_lookup;
pub mod planner;
pub mod trace;

pub use graph::{collect_dependencies_recursive, DependencyGraphRequest};
pub use hydrate::hydrate_dependencies;
pub use link::link_manifest_dependencies;
pub use manifest_lookup::{
    build_exists_for_manifest, find_manifest_for_commit, get_build_dir_for_package,
};

use self::checksums::add_missing_checksums_to_manifests;
use self::planner::{build_packages_parallel, show_parallel_execution_plan};
use self::trace::trace_dependency_chains;

/// Show dependency paths from root to each node
fn show_dependency_paths(
    graph: &DiGraph<ManifestSource, ()>,
    manifest_map: &HashMap<PathBuf, NodeIndex>,
    root_path: &Path,
) {
    let root_node = manifest_map
        .get(root_path)
        .expect("Root manifest should be in map");

    for path_nodes in dependency_paths(graph, manifest_map, root_path, *root_node) {
        print!("  ");
        for (i, node) in path_nodes.iter().enumerate() {
            let p = graph[*node].path();
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
    }
}

fn dependency_paths(
    graph: &DiGraph<ManifestSource, ()>,
    manifest_map: &HashMap<PathBuf, NodeIndex>,
    root_path: &Path,
    root_node: NodeIndex,
) -> Vec<Vec<NodeIndex>> {
    let mut entries: Vec<_> = manifest_map.iter().collect();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    entries
        .into_iter()
        .filter(|(path, &node)| path.as_path() != root_path && !graph[node].is_skip())
        .filter_map(|(_, &node)| dependency_path_to_node(graph, root_node, node))
        .collect()
}

fn dependency_path_to_node(
    graph: &DiGraph<ManifestSource, ()>,
    root_node: NodeIndex,
    target_node: NodeIndex,
) -> Option<Vec<NodeIndex>> {
    let mut queue = VecDeque::from([root_node]);
    let mut parent_map: HashMap<NodeIndex, Option<NodeIndex>> = HashMap::new();
    parent_map.insert(root_node, None);

    while let Some(current) = queue.pop_front() {
        if current == target_node {
            return Some(rebuild_dependency_path(current, &parent_map));
        }
        for dependency in graph.neighbors_directed(current, Direction::Incoming) {
            if parent_map.contains_key(&dependency) {
                continue;
            }
            parent_map.insert(dependency, Some(current));
            queue.push_back(dependency);
        }
    }

    None
}

fn rebuild_dependency_path(
    target_node: NodeIndex,
    parent_map: &HashMap<NodeIndex, Option<NodeIndex>>,
) -> Vec<NodeIndex> {
    let mut path_nodes = vec![target_node];
    let mut current = target_node;
    while let Some(Some(parent)) = parent_map.get(&current) {
        path_nodes.push(*parent);
        current = *parent;
    }
    path_nodes.reverse();
    path_nodes
}

/// Build a manifest and all its missing dependencies
pub fn build_with_dependencies(
    manifest_path: &Path,
    manifest_dirs: &[PathBuf],
    opts: &BuildOpts,
) -> io::Result<()> {
    print_build_graph_header(manifest_path, opts.dry_run);
    let (graph, manifest_map) = build_dependency_graph(manifest_path, manifest_dirs, opts)?;

    if let Some(ref pattern) = opts.trace_dependency {
        return trace_dependency_chains(&opts.repo_path, manifest_path, manifest_dirs, pattern);
    }

    let valid_nodes = valid_graph_nodes(&graph);
    if valid_nodes.is_empty() {
        println!("All packages already built!");
        return Ok(());
    }

    println!(
        "\nDependency graph has {} packages to build",
        valid_nodes.len()
    );
    maybe_show_dependency_paths(opts, &graph, &manifest_map, manifest_path);

    let build_order = build_order(&graph)?;
    handle_build_order(manifest_path, opts, &graph, &build_order)
}

fn print_build_graph_header(manifest_path: &Path, dry_run: bool) {
    if dry_run {
        println!(
            "DRY RUN: Analyzing dependency graph for {}",
            manifest_path.display()
        );
    } else {
        println!("Building dependency graph for {}", manifest_path.display());
    }
}

fn build_dependency_graph(
    manifest_path: &Path,
    manifest_dirs: &[PathBuf],
    opts: &BuildOpts,
) -> io::Result<(DiGraph<ManifestSource, ()>, HashMap<PathBuf, NodeIndex>)> {
    let mut graph = DiGraph::new();
    let mut manifest_map = HashMap::new();
    let mut ref_cache = HashMap::new();
    let fallback_paths: Vec<PathBuf> = opts.fallback_repos.iter().map(PathBuf::from).collect();
    let store = Store::open_with_fallback_chain(&opts.repo_path, &fallback_paths).ok();
    let root_source = ManifestSource::Path(manifest_path.to_path_buf());

    collect_dependencies_recursive(DependencyGraphRequest {
        manifest_source: &root_source,
        repo_path: &opts.repo_path,
        manifest_dirs,
        graph: &mut graph,
        manifest_map: &mut manifest_map,
        force: opts.force || opts.add_checksums,
        ref_cache: &mut ref_cache,
        store: store.as_ref(),
        verbose: opts.verbose,
    })?;

    Ok((graph, manifest_map))
}

fn valid_graph_nodes(graph: &DiGraph<ManifestSource, ()>) -> Vec<NodeIndex> {
    graph
        .node_indices()
        .filter(|&idx| !graph[idx].is_skip())
        .collect()
}

fn maybe_show_dependency_paths(
    opts: &BuildOpts,
    graph: &DiGraph<ManifestSource, ()>,
    manifest_map: &HashMap<PathBuf, NodeIndex>,
    manifest_path: &Path,
) {
    if opts.show_dep_paths {
        println!("\nDependency paths:");
        show_dependency_paths(graph, manifest_map, manifest_path);
    }
}

fn build_order(graph: &DiGraph<ManifestSource, ()>) -> io::Result<Vec<NodeIndex>> {
    let build_order = toposort(graph, None).map_err(|cycle| {
        let node_source = &graph[cycle.node_id()];
        io::Error::other(format!(
            "Circular dependency detected at node {:?}: {}",
            cycle.node_id(),
            node_source.path().display()
        ))
    })?;

    Ok(build_order
        .into_iter()
        .filter(|&idx| !graph[idx].is_skip())
        .collect())
}

fn handle_build_order(
    manifest_path: &Path,
    opts: &BuildOpts,
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
) -> io::Result<()> {
    if opts.add_checksums {
        println!("\nAdding missing checksums...");
        add_missing_checksums_to_manifests(build_order, graph, &opts.repo_path, opts)?;
        println!("Checksums updated!");
        return Ok(());
    }

    if opts.dry_run {
        println!("\nDRY RUN: Parallel execution plan:");
        show_parallel_execution_plan(graph, build_order, false);
        println!("\nDRY RUN: Would build {} packages", build_order.len());
        return Ok(());
    }

    run_build_order(manifest_path, opts, graph, build_order)
}

fn run_build_order(
    _manifest_path: &Path,
    opts: &BuildOpts,
    graph: &DiGraph<ManifestSource, ()>,
    build_order: &[NodeIndex],
) -> io::Result<()> {
    print_build_order(graph, build_order);
    println!("\nParallel execution plan:");
    show_parallel_execution_plan(graph, build_order, true);

    println!("\nStarting parallel builds...\n");
    build_packages_parallel(graph, build_order, opts)?;

    println!("==================================================");
    println!("All packages built successfully!");
    println!("==================================================");

    Ok(())
}

fn print_build_order(graph: &DiGraph<ManifestSource, ()>, build_order: &[NodeIndex]) {
    println!("\nBuild order:");
    for (index, &node_idx) in build_order.iter().enumerate() {
        let source = &graph[node_idx];
        println!("  {}. {}", index + 1, source.path().display());
    }
}

#[cfg(test)]
#[path = "orchestration_tests.rs"]
mod orchestration_tests;
