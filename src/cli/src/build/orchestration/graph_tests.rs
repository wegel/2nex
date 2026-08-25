use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use petgraph::algo::{has_path_connecting, toposort};
use petgraph::graph::{DiGraph, NodeIndex};

use super::{collect_dependencies_recursive, DependencyGraphRequest};
use crate::manifest::ManifestSource;

/// Scenario: a developer builds a child whose base and package are live files.
/// Nex must order the package first, the base second, and the child last.
#[test]
fn live_base_is_a_build_graph_edge() -> io::Result<()> {
    let fixture = Fixture::new()?;
    let child = fixture.root.join("asm/child.yaml").canonicalize()?;
    let base = fixture.root.join("base/base.yaml").canonicalize()?;
    let package = fixture.root.join("pkg/apps/demo.yaml").canonicalize()?;
    let (graph, nodes) = fixture.graph(&child)?;

    assert!(has_path_connecting(
        &graph,
        nodes[&package],
        nodes[&base],
        None,
    ));
    assert!(has_path_connecting(
        &graph,
        nodes[&base],
        nodes[&child],
        None,
    ));
    let order = toposort(&graph, None).expect("the base graph must be acyclic");
    assert!(position(&order, nodes[&package]) < position(&order, nodes[&base]));
    assert!(position(&order, nodes[&base]) < position(&order, nodes[&child]));
    Ok(())
}

/// Scenario: two assemblies name each other as their base.
/// Nex must expose the cycle to the existing graph check instead of recursing forever.
#[test]
fn base_cycle_is_rejected_by_the_build_graph() -> io::Result<()> {
    let fixture = Fixture::new()?;
    write(
        &fixture.root.join("base/base.yaml"),
        &assembly("base", Some("asm/child.yaml"), false),
    )?;
    let child = fixture.root.join("asm/child.yaml").canonicalize()?;
    let (graph, _) = fixture.graph(&child)?;

    assert!(
        toposort(&graph, None).is_err(),
        "the base cycle must remain visible"
    );
    Ok(())
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("repo");
        fs::create_dir_all(root.join(".git"))?;
        write(&root.join("pkg/apps/demo.yaml"), package())?;
        write(&root.join("base/base.yaml"), &assembly("base", None, true))?;
        write(
            &root.join("asm/child.yaml"),
            &assembly("child", Some("base/base.yaml"), false),
        )?;
        Ok(Self { _temp: temp, root })
    }

    fn graph(
        &self,
        child: &Path,
    ) -> io::Result<(DiGraph<ManifestSource, ()>, HashMap<PathBuf, NodeIndex>)> {
        let mut graph = DiGraph::new();
        let mut nodes = HashMap::new();
        let mut cache = HashMap::new();
        let manifest_dirs = vec![self.root.join("pkg")];
        let store_path = self.root.join("store").to_string_lossy().into_owned();
        collect_dependencies_recursive(DependencyGraphRequest {
            manifest_source: &ManifestSource::Path(child.to_path_buf()),
            repo_path: &store_path,
            manifest_dirs: &manifest_dirs,
            graph: &mut graph,
            manifest_map: &mut nodes,
            force: true,
            ref_cache: &mut cache,
            store: None,
            verbose: false,
        })?;
        Ok((graph, nodes))
    }
}

fn assembly(slug: &str, base: Option<&str>, includes_package: bool) -> String {
    let base = base
        .map(|path| format!("base:\n  commit: systems/base/1\n  manifest: {path}\n"))
        .unwrap_or_default();
    let packages = if includes_package {
        "packages:\n- name: demo\n  commit: x86_64/pkg/apps/demo/1/outputs/bin\n"
    } else {
        "packages: []\n"
    };
    format!(
        "system:\n  name: {slug}\n  slug: {slug}\n  version: 1\n{base}{packages}build:\n  environment: abcdef\n  script: \"true\"\n"
    )
}

fn package() -> &'static str {
    "package:\n  name: demo\n  slug: demo\n  namespace: apps\n  version: 1\nsources: []\ndependencies: []\nbuild:\n  environment: abcdef\n  script: \"true\"\nbundles: {}\noutputs: {}\n"
}

fn write(path: &Path, contents: &str) -> io::Result<()> {
    fs::create_dir_all(path.parent().expect("fixture path needs a parent"))?;
    fs::write(path, contents)
}

fn position(order: &[NodeIndex], node: NodeIndex) -> usize {
    order
        .iter()
        .position(|candidate| *candidate == node)
        .expect("node must be in build order")
}
