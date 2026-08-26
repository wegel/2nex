//! Tests for build-recipe identity.

use std::fs;

use zub::Repo;

use super::add_catalog;
use crate::builder::reference_tree;
use crate::manifest::Dependency;
use crate::reference::InputRef;

#[test]
fn dependency_reference_is_not_recipe_data() {
    let dependency = |commit: &str| Dependency {
        name: None,
        commit: InputRef::Stored(commit.to_owned()),
        paths: vec!["usr/bin".into()],
    };
    let first =
        serde_yaml::to_string(&dependency(&"a".repeat(64))).expect("serialize first dependency");
    let second =
        serde_yaml::to_string(&dependency(&"b".repeat(64))).expect("serialize second dependency");
    assert_eq!(first, second);
}

#[test]
fn dependency_tree_ignores_commit_metadata() {
    let temp = tempfile::tempdir().expect("create recipe fixture directory");
    let input = temp.path().join("input");
    fs::create_dir(&input).expect("create recipe input");
    fs::write(input.join("file"), b"first").expect("write first input content");
    let repo = Repo::init(&temp.path().join("repo")).expect("initialize recipe repo");
    let first = zub::ops::commit(&repo, &input, "dependency", None, Some("test"))
        .expect("commit first input");
    let tree = reference_tree(&repo, &first.to_string()).expect("read first input tree");

    let metadata = zub::ops::commit_tree_with_metadata(
        &repo,
        &tree,
        "dependency",
        Some("metadata only"),
        Some("test"),
        &[("clock", "later")],
    )
    .expect("commit metadata-only change");
    assert_eq!(
        tree,
        reference_tree(&repo, &metadata.to_string()).expect("read metadata-only tree")
    );

    fs::write(input.join("file"), b"second").expect("write changed input content");
    let changed = zub::ops::commit(&repo, &input, "dependency", None, Some("test"))
        .expect("commit changed input");
    assert_ne!(
        tree,
        reference_tree(&repo, &changed.to_string()).expect("read changed input tree")
    );
}

#[test]
fn embedded_catalog_manifests_are_recipe_data() {
    let temporary = tempfile::tempdir().expect("create catalog fixture directory");
    let package = temporary.path().join("pkg/core");
    fs::create_dir_all(&package).expect("create catalog package directory");
    fs::write(package.join("tool.yaml"), "first").expect("write first catalog manifest");
    let mut first = blake3::Hasher::new();
    add_catalog(&mut first, temporary.path()).expect("hash first catalog");

    fs::write(package.join("tool.yaml"), "second").expect("write changed catalog manifest");
    let mut second = blake3::Hasher::new();
    add_catalog(&mut second, temporary.path()).expect("hash changed catalog");

    assert_ne!(first.finalize(), second.finalize());
}
