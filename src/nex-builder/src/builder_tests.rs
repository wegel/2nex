//! Tests for selected dependency trees.

use std::fs;
use std::os::unix::fs::symlink;

use zub::Repo;

use super::checkout_dependencies;
use crate::manifest::Dependency;
use crate::reference::InputRef;

#[test]
fn runtime_selection_keeps_symlink_targets_but_not_development_files() {
    let temp = tempfile::tempdir().expect("create dependency fixture directory");
    let source = temp.path().join("source/usr/lib");
    fs::create_dir_all(source.join("pkgconfig")).expect("create dependency files");
    fs::write(source.join("libexample.so.1.2"), b"runtime").expect("write runtime library");
    fs::write(source.join("pkgconfig/example.pc"), b"development").expect("write development file");
    symlink("libexample.so.1.2", source.join("libexample.so.1"))
        .expect("create runtime soname link");
    symlink("libexample.so.1", source.join("libexample.so"))
        .expect("create development linker-name link");

    let repo = Repo::init(&temp.path().join("repo")).expect("initialize dependency repo");
    let commit = zub::ops::commit(
        &repo,
        &temp.path().join("source"),
        "example",
        None,
        Some("test"),
    )
    .expect("commit dependency fixture");
    let target = temp.path().join("target");
    checkout_dependencies(
        &repo,
        &[Dependency {
            name: Some("example".into()),
            commit: InputRef::Stored(commit.to_string()),
            paths: vec!["/usr/lib/libexample.so.1".into()],
        }],
        &target,
    )
    .expect("check out selected dependency path");

    assert_eq!(
        fs::read_link(target.join("usr/lib/libexample.so.1")).expect("read runtime soname link"),
        std::path::Path::new("libexample.so.1.2")
    );
    assert_eq!(
        fs::read(target.join("usr/lib/libexample.so.1.2")).expect("read runtime library"),
        b"runtime"
    );
    assert!(!target.join("usr/lib/libexample.so").exists());
    assert!(!target.join("usr/lib/pkgconfig").exists());
}
