//! Tests for scheduler state without worker processes.

use std::io;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use super::Scheduler;
use crate::graph::{BuildGraph, Node, NodeKind};
use crate::manifest::{BuildEnvironment, PackageManifest};
use crate::node_runner::NodeOutcome;
use crate::Error;

#[test]
fn scheduler_tracks_ready_resources_completion_and_errors() {
    let graph = graph(&[&[], &[], &[0, 1]]);
    let mut scheduler = Scheduler::new(&graph);
    assert_eq!(scheduler.ready.iter().copied().collect::<Vec<_>>(), [0, 1]);

    scheduler.ready.remove(&0);
    scheduler.ready.remove(&1);
    scheduler.started(cpus(2));
    scheduler.started(cpus(3));
    assert_eq!((scheduler.active, scheduler.used_cpus), (2, 5));

    scheduler.record(0, cpus(2), Ok(NodeOutcome::Built));
    assert_eq!((scheduler.active, scheduler.used_cpus), (1, 3));
    assert_eq!((scheduler.completed, scheduler.built), (1, 1));
    assert!(scheduler.ready.is_empty());

    scheduler.record(1, cpus(3), Ok(NodeOutcome::Reused));
    assert_eq!(scheduler.ready.iter().copied().collect::<Vec<_>>(), [2]);
    assert_eq!((scheduler.completed, scheduler.reused), (2, 1));

    scheduler.ready.remove(&2);
    scheduler.started(cpus(1));
    scheduler.record(2, cpus(1), Err(Error::Io(io::Error::other("failed"))));
    assert_eq!((scheduler.active, scheduler.used_cpus), (0, 0));
    assert_eq!(scheduler.completed, 3);
    assert!(scheduler.errors.contains_key(&2));
}

fn graph(dependencies: &[&[usize]]) -> BuildGraph {
    let manifest: PackageManifest = serde_yaml::from_str(PACKAGE).expect("test package manifest");
    let environment: BuildEnvironment =
        serde_yaml::from_str(ENVIRONMENT).expect("test build environment");
    let nodes = dependencies
        .iter()
        .enumerate()
        .map(|(index, dependencies)| Node {
            label: format!("node {index}"),
            manifest_path: PathBuf::from("package.yaml"),
            environment: environment.clone(),
            dependencies: dependencies.to_vec(),
            kind: NodeKind::Package {
                manifest: manifest.clone(),
                inputs: Vec::new(),
            },
        })
        .collect();
    BuildGraph {
        nodes,
        root: dependencies.len() - 1,
    }
}

fn cpus(count: usize) -> NonZeroUsize {
    NonZeroUsize::new(count).expect("nonzero test CPU count")
}

const PACKAGE: &str = r#"
package: {schema: 1, name: Test, slug: test, namespace: test, version: 1, description: Test}
build: {environment: unused, script: ""}
"#;

const ENVIRONMENT: &str = r#"
name: test
execution: {chroot: false}
paths: {work: work, out: out, inputs: inputs}
env: {}
preamble: ""
"#;
