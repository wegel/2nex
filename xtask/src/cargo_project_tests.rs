//! Tests for Cargo project scope and source-kind detection.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{is_library_source, scopes_for_staged_paths, CargoScope, RustPackage, RustProject};

fn projects() -> Vec<RustProject> {
    vec![
        project(
            "builder",
            "src/nex-builder",
            &[
                ("nex-builder", "src/nex-builder"),
                ("nex-source", "src/nex-builder/crates/source"),
            ],
        ),
        project(
            "bootloader",
            "src/bootloader",
            &[("nex-bootloader", "src/bootloader")],
        ),
        project("hooks", "xtask", &[("xtask", "xtask")]),
    ]
}

fn project(name: &str, root: &str, packages: &[(&str, &str)]) -> RustProject {
    RustProject {
        name: name.to_owned(),
        manifest_path: PathBuf::from(root).join("Cargo.toml"),
        root: PathBuf::from(root),
        packages: packages
            .iter()
            .map(|(name, root)| RustPackage {
                name: (*name).to_owned(),
                root: PathBuf::from(root),
                library_root: Some(PathBuf::from(root).join("src")),
                non_library_entrypoints: BTreeSet::new(),
                dependencies: if *name == "nex-builder" {
                    BTreeSet::from(["nex-source".to_owned()])
                } else {
                    BTreeSet::new()
                },
            })
            .collect(),
        clippy_args: Vec::new(),
        test_args: Vec::new(),
    }
}

fn packages(names: &[&str]) -> CargoScope {
    CargoScope::Packages(names.iter().map(|name| (*name).to_owned()).collect())
}

#[test]
fn maps_source_to_its_nearest_package() {
    let paths = vec![PathBuf::from("src/nex-builder/crates/source/src/lib.rs")];
    let scopes = scopes_for_staged_paths(&projects(), &paths);
    assert_eq!(
        scopes,
        BTreeMap::from([(0, packages(&["nex-builder", "nex-source"]))])
    );
}

#[test]
fn combines_packages_from_one_workspace() {
    let paths = vec![
        PathBuf::from("src/nex-builder/src/lib.rs"),
        PathBuf::from("src/nex-builder/crates/source/src/lib.rs"),
    ];
    let scopes = scopes_for_staged_paths(&projects(), &paths);
    assert_eq!(
        scopes,
        BTreeMap::from([(0, packages(&["nex-builder", "nex-source"]))])
    );
}

#[test]
fn manifest_change_checks_the_whole_project() {
    let paths = vec![PathBuf::from("src/nex-builder/crates/source/Cargo.toml")];
    let scopes = scopes_for_staged_paths(&projects(), &paths);
    assert_eq!(scopes, BTreeMap::from([(0, CargoScope::Workspace)]));
}

#[test]
fn hook_config_change_checks_every_project() {
    let paths = vec![PathBuf::from(".githooks/config.toml")];
    let scopes = scopes_for_staged_paths(&projects(), &paths);
    assert_eq!(
        scopes,
        BTreeMap::from([
            (0, CargoScope::Workspace),
            (1, CargoScope::Workspace),
            (2, CargoScope::Workspace),
        ])
    );
}

#[test]
fn ignores_non_rust_files() {
    let paths = vec![PathBuf::from("catalog/pkg/example/package.yaml")];
    assert_eq!(
        scopes_for_staged_paths(&projects(), &paths),
        BTreeMap::new()
    );
}

#[test]
fn package_sets_have_stable_order() {
    let scope = packages(&["z", "a"]);
    assert_eq!(
        scope,
        CargoScope::Packages(BTreeSet::from(["a".to_owned(), "z".to_owned()]))
    );
}

#[test]
fn distinguishes_library_modules_from_binary_entrypoints() {
    let mut projects = projects();
    projects[0].packages[0]
        .non_library_entrypoints
        .insert(PathBuf::from("src/nex-builder/src/main.rs"));
    assert!(is_library_source(
        &projects,
        Path::new("src/nex-builder/src/graph.rs")
    ));
    assert!(!is_library_source(
        &projects,
        Path::new("src/nex-builder/src/main.rs")
    ));
}
