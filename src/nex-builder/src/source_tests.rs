//! Tests for source staging names and variables.

use std::fs;

use super::stage_sources;
use crate::manifest::{BuildEnvironment, BuildPaths, Execution, Source, SourceSpec};

fn environment() -> BuildEnvironment {
    BuildEnvironment {
        name: "test".into(),
        execution: Execution { chroot: true },
        paths: BuildPaths {
            work: "work".into(),
            out: "out".into(),
            inputs: "inputs".into(),
        },
        env: Default::default(),
        preamble: String::new(),
    }
}

fn source(name: &str, file: &str) -> Source {
    Source {
        name: name.into(),
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        kind: SourceSpec::File { file: file.into() },
    }
}

#[test]
fn local_file_keeps_its_basename() {
    let temp = tempfile::tempdir().expect("create source fixture directory");
    let manifest = temp.path().join("catalog/pkg/example.yaml");
    let file = temp.path().join("pkg/shim.c");
    fs::create_dir_all(manifest.parent().expect("manifest parent"))
        .expect("create manifest directory");
    fs::create_dir_all(file.parent().expect("source parent")).expect("create source directory");
    fs::write(&file, b"abc").expect("write local source");

    let variables = stage_sources(
        &[source("shim", "pkg/shim.c")],
        &manifest,
        &temp.path().join("cache"),
        &temp.path().join("root"),
        &environment(),
    )
    .expect("stage local source");

    assert!(temp.path().join("root/inputs/shim.c").is_file());
    assert_eq!(variables["SOURCE0"], "/inputs/shim.c");
    assert_eq!(variables["SOURCE_shim"], "/inputs/shim.c");
}

#[test]
fn local_basename_collision_is_rejected_before_acquisition() {
    let temp = tempfile::tempdir().expect("create collision fixture directory");
    let error = stage_sources(
        &[
            source("first", "one/input.c"),
            source("second", "two/input.c"),
        ],
        &temp.path().join("manifest.yaml"),
        &temp.path().join("cache"),
        &temp.path().join("root"),
        &environment(),
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("more than one source stages as input.c"));
}
