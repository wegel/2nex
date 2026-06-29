use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use petgraph::graph::DiGraph;
use tempfile::TempDir;

use crate::build::compute_manifest_hash;
use crate::manifest::types::ManifestSource;
use crate::store::{commit_tree, Store};

use super::{collect_dependencies_recursive, DependencyGraphRequest};

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
        repo_path: repo_path.to_str().unwrap(),
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
    let store = Store::open(repo_path)?;
    let mut graph = DiGraph::new();
    let mut manifest_map = HashMap::new();
    let mut ref_cache = HashMap::new();
    let manifest_dirs = vec![temp_dir.path().to_path_buf()];
    let root_source = ManifestSource::Path(root_manifest_path.clone());

    collect_dependencies_recursive(DependencyGraphRequest {
        manifest_source: &root_source,
        repo_path,
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
