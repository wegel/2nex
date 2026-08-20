use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use petgraph::graph::DiGraph;
use tempfile::TempDir;

use crate::build::compute_manifest_hash;
use crate::manifest::types::ManifestSource;
use crate::store::{commit_tree, Store};

use super::{collect_dependencies_recursive, dependency_paths, DependencyGraphRequest};

fn write_manifest(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn host_lacks_root_user_namespace_mapping(error: &io::Error) -> bool {
    error.to_string().contains("uid 0 not mapped in namespace")
}

fn skip_if_host_lacks_root_user_namespace_mapping(error: io::Error) -> io::Result<()> {
    if host_lacks_root_user_namespace_mapping(&error) {
        eprintln!(
            "skipping stale dependency rebuild test: host cannot map uid 0 in a user namespace"
        );
        return Ok(());
    }
    Err(error)
}

#[test]
fn rebuilds_when_dependency_manifest_changes() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let Some(repo_path) = init_test_store(&temp_dir)? else {
        return Ok(());
    };
    let root_manifest_path = temp_dir.path().join("pkg/apps/kernel.yaml");
    let dep_manifest_path = temp_dir.path().join("pkg/deps/initramfs.yaml");

    let dep_manifest_v1 = write_dependency_manifest(&dep_manifest_path)?;
    write_built_output(
        &repo_path,
        &dep_manifest_path,
        "x86_64/pkg/deps/initramfs/1.0/outputs/boot",
        "boot",
    )?;

    write_root_manifest(&root_manifest_path)?;
    write_built_output(
        &repo_path,
        &root_manifest_path,
        "x86_64/pkg/apps/kernel/1.0/outputs/bin",
        "kernel",
    )?;

    let dep_manifest_v2 = dep_manifest_v1.replace("description: v1", "description: v2");
    write_manifest(&dep_manifest_path, &dep_manifest_v2)?;

    let (graph, manifest_map) =
        collect_test_graph(&repo_path, &temp_dir, root_manifest_path.clone())?;
    assert_node_rebuilds(&graph, &manifest_map, &root_manifest_path, "root");
    assert_node_rebuilds(&graph, &manifest_map, &dep_manifest_path, "dependency");

    Ok(())
}

#[test]
fn fresh_fallback_dependency_is_not_scheduled() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let primary_repo = temp_dir.path().join("primary");
    let fallback_repo = temp_dir.path().join("fallback");
    if !init_store_at(&primary_repo)? || !init_store_at(&fallback_repo)? {
        return Ok(());
    }

    let root_manifest_path = temp_dir.path().join("pkg/apps/kernel.yaml");
    let dep_manifest_path = temp_dir.path().join("pkg/deps/initramfs.yaml");
    write_dependency_manifest(&dep_manifest_path)?;
    write_root_manifest(&root_manifest_path)?;
    write_built_output(
        fallback_repo.to_str().expect("fallback repo path"),
        &dep_manifest_path,
        "x86_64/pkg/deps/initramfs/1.0/outputs/boot",
        "boot",
    )?;
    write_built_output(
        primary_repo.to_str().expect("primary repo path"),
        &root_manifest_path,
        "x86_64/pkg/apps/kernel/1.0/outputs/bin",
        "kernel",
    )?;

    let (graph, manifest_map) = collect_test_graph_with_fallbacks(
        &primary_repo,
        &[fallback_repo],
        &temp_dir,
        root_manifest_path.clone(),
    )?;

    assert!(!manifest_map.contains_key(&dep_manifest_path));
    let root = manifest_map
        .get(&root_manifest_path)
        .expect("root manifest should be in graph");
    assert!(
        graph[*root].is_skip(),
        "the fresh graph should schedule no build"
    );
    Ok(())
}

#[test]
fn stale_fallback_dependency_is_scheduled() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let primary_repo = temp_dir.path().join("primary");
    let fallback_repo = temp_dir.path().join("fallback");
    if !init_store_at(&primary_repo)? || !init_store_at(&fallback_repo)? {
        return Ok(());
    }

    let root_manifest_path = temp_dir.path().join("pkg/apps/kernel.yaml");
    let dep_manifest_path = temp_dir.path().join("pkg/deps/initramfs.yaml");
    let dep_manifest_v1 = write_dependency_manifest(&dep_manifest_path)?;
    write_root_manifest(&root_manifest_path)?;
    write_built_output(
        fallback_repo.to_str().expect("fallback repo path"),
        &dep_manifest_path,
        "x86_64/pkg/deps/initramfs/1.0/outputs/boot",
        "boot",
    )?;
    write_built_output(
        primary_repo.to_str().expect("primary repo path"),
        &root_manifest_path,
        "x86_64/pkg/apps/kernel/1.0/outputs/bin",
        "kernel",
    )?;
    write_manifest(
        &dep_manifest_path,
        &dep_manifest_v1.replace("description: v1", "description: v2"),
    )?;

    let (graph, manifest_map) = collect_test_graph_with_fallbacks(
        &primary_repo,
        &[fallback_repo],
        &temp_dir,
        root_manifest_path.clone(),
    )?;

    assert_node_rebuilds(&graph, &manifest_map, &dep_manifest_path, "dependency");
    assert_node_rebuilds(&graph, &manifest_map, &root_manifest_path, "root");
    Ok(())
}

#[test]
fn missing_dependency_manifest_stops_graph_collection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let root_manifest_path = temp_dir.path().join("pkg/apps/kernel.yaml");
    write_root_manifest(&root_manifest_path)?;

    let mut graph = DiGraph::new();
    let mut manifest_map = HashMap::new();
    let mut ref_cache = HashMap::new();
    let manifest_dirs = vec![temp_dir.path().to_path_buf()];
    let root_source = ManifestSource::Path(root_manifest_path);
    let repo_path = temp_dir.path().join("repo");

    let error = collect_dependencies_recursive(DependencyGraphRequest {
        manifest_source: &root_source,
        repo_path: repo_path.to_str().expect("test setup should succeed"),
        manifest_dirs: &manifest_dirs,
        graph: &mut graph,
        manifest_map: &mut manifest_map,
        force: false,
        ref_cache: &mut ref_cache,
        store: None,
        verbose: false,
    })
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("cannot build dependency"));
    assert!(message.contains("x86_64/pkg/deps/initramfs/1.0/outputs/boot"));
    assert!(message.contains("no manifest was found"));
    Ok(())
}

#[test]
fn dependency_paths_follow_dependency_edges_back_from_root() {
    let temp_dir = TempDir::new().expect("test setup should succeed");
    let root_path = temp_dir.path().join("pkg/apps/root.yaml");
    let direct_dep_path = temp_dir.path().join("pkg/libs/direct.yaml");
    let transitive_dep_path = temp_dir.path().join("pkg/libs/transitive.yaml");

    let mut graph = DiGraph::new();
    let root = graph.add_node(ManifestSource::Path(root_path.clone()));
    let direct_dep = graph.add_node(ManifestSource::Path(direct_dep_path.clone()));
    let transitive_dep = graph.add_node(ManifestSource::Path(transitive_dep_path.clone()));
    graph.add_edge(direct_dep, root, ());
    graph.add_edge(transitive_dep, direct_dep, ());

    let manifest_map = HashMap::from([
        (root_path.clone(), root),
        (direct_dep_path, direct_dep),
        (transitive_dep_path, transitive_dep),
    ]);

    let paths = dependency_paths(&graph, &manifest_map, &root_path, root);

    assert_eq!(
        paths,
        vec![
            vec![root, direct_dep],
            vec![root, direct_dep, transitive_dep]
        ]
    );
}

fn init_test_store(temp_dir: &TempDir) -> io::Result<Option<String>> {
    let repo_dir = temp_dir.path().join("repo");
    if let Err(error) = Store::init(&repo_dir) {
        if host_lacks_root_user_namespace_mapping(&error) {
            eprintln!(
                "skipping stale dependency rebuild test: host cannot map uid 0 in a user namespace"
            );
            return Ok(None);
        }
        return Err(error);
    }
    Ok(Some(repo_dir.to_string_lossy().to_string()))
}

fn init_store_at(repo_dir: &Path) -> io::Result<bool> {
    match Store::init(repo_dir) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => {
            eprintln!("skipping fallback graph test: host cannot map uid 0");
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn write_dependency_manifest(path: &Path) -> io::Result<String> {
    let manifest = r#"package:
  schema: 1
  name: initramfs
  slug: initramfs
  namespace: deps
  version: "1.0"
  description: v1
dependencies: []
sources: []
build:
  environment: env/test.yaml
  script: "true"
outputs:
  boot:
    files:
      - path: /boot/initramfs.cpio
bundles: {}
"#;
    write_manifest(path, manifest)?;
    Ok(manifest.to_string())
}

fn write_root_manifest(path: &Path) -> io::Result<()> {
    let manifest = r#"package:
  schema: 1
  name: kernel
  slug: kernel
  namespace: apps
  version: "1.0"
dependencies:
  - name: initramfs
    commit: x86_64/pkg/deps/initramfs/1.0/outputs/boot
sources: []
build:
  environment: env/test.yaml
  script: "true"
outputs:
  bin:
    files:
      - path: /usr/bin/kernel
bundles: {}
"#;
    write_manifest(path, manifest)
}

fn write_built_output(
    repo_path: &str,
    manifest_path: &Path,
    ref_name: &str,
    file_name: &str,
) -> io::Result<()> {
    let manifest_hash = compute_manifest_hash(manifest_path)?;
    let tree_dir = manifest_path.with_file_name(format!("{}_tree", file_name));
    fs::create_dir_all(&tree_dir)?;
    fs::write(tree_dir.join(file_name), file_name)?;
    if let Err(error) = commit_tree(
        repo_path,
        ref_name,
        &tree_dir,
        &[("nex.manifest.hash".to_string(), manifest_hash)],
    ) {
        return skip_if_host_lacks_root_user_namespace_mapping(error);
    }
    Ok(())
}

fn collect_test_graph(
    repo_path: &str,
    temp_dir: &TempDir,
    root_manifest_path: std::path::PathBuf,
) -> io::Result<(
    DiGraph<ManifestSource, ()>,
    HashMap<std::path::PathBuf, petgraph::prelude::NodeIndex>,
)> {
    collect_test_graph_with_fallbacks(Path::new(repo_path), &[], temp_dir, root_manifest_path)
}

fn collect_test_graph_with_fallbacks(
    repo_path: &Path,
    fallback_paths: &[std::path::PathBuf],
    temp_dir: &TempDir,
    root_manifest_path: std::path::PathBuf,
) -> io::Result<(
    DiGraph<ManifestSource, ()>,
    HashMap<std::path::PathBuf, petgraph::prelude::NodeIndex>,
)> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_paths)?;
    let mut graph = DiGraph::new();
    let mut manifest_map = HashMap::new();
    let mut ref_cache = HashMap::new();
    let manifest_dirs = vec![temp_dir.path().to_path_buf()];
    let root_source = ManifestSource::Path(root_manifest_path.clone());

    collect_dependencies_recursive(DependencyGraphRequest {
        manifest_source: &root_source,
        repo_path: repo_path.to_str().expect("repo path should be UTF-8"),
        manifest_dirs: &manifest_dirs,
        graph: &mut graph,
        manifest_map: &mut manifest_map,
        force: false,
        ref_cache: &mut ref_cache,
        store: Some(&store),
        verbose: false,
    })?;
    Ok((graph, manifest_map))
}

fn assert_node_rebuilds(
    graph: &DiGraph<ManifestSource, ()>,
    manifest_map: &HashMap<std::path::PathBuf, petgraph::prelude::NodeIndex>,
    path: &Path,
    label: &str,
) {
    let node = *manifest_map
        .get(path)
        .unwrap_or_else(|| panic!("{} manifest missing from graph", label));
    assert!(
        !graph[node].is_skip(),
        "{} should rebuild when a dependency is stale",
        label
    );
}
