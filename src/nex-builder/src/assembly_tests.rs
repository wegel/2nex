//! Tests for safe assembly file materialization.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use super::materialize_files;
use crate::assembly_manifest::{AssemblyFile, AssemblyFileKind};

#[test]
fn assembly_file_cannot_traverse_a_target_symlink() {
    let temporary = tempfile::tempdir().expect("create assembly fixture directory");
    let target = temporary.path().join("target");
    let outside = temporary.path().join("outside");
    fs::create_dir(&target).expect("create assembly target");
    fs::create_dir(&outside).expect("create outside directory");
    symlink(&outside, target.join("escape")).expect("create escaping symlink");
    let file = AssemblyFile {
        path: PathBuf::from("/escape/file"),
        kind: AssemblyFileKind::Content {
            content: "unsafe".to_string(),
        },
        mode: None,
        replace: false,
    };

    let error = materialize_files(&[file], temporary.path(), &target).unwrap_err();

    assert!(error.to_string().contains("traverses symlink"));
    assert!(!outside.join("file").exists());
}

#[test]
fn assembly_file_replaces_a_package_file() {
    let temporary = tempfile::tempdir().expect("create assembly fixture directory");
    let target = temporary.path().join("target");
    fs::create_dir(&target).expect("create assembly target");
    fs::write(target.join("policy"), "package").expect("write package file");
    let file = AssemblyFile {
        path: PathBuf::from("/policy"),
        kind: AssemblyFileKind::Content {
            content: "assembly".to_string(),
        },
        mode: None,
        replace: false,
    };

    materialize_files(&[file], temporary.path(), &target).expect("replace package file");

    assert_eq!(
        fs::read_to_string(target.join("policy")).expect("read replaced package file"),
        "assembly"
    );
}

#[test]
fn assembly_file_requires_replace_to_remove_a_directory() {
    let temporary = tempfile::tempdir().expect("create assembly fixture directory");
    let target = temporary.path().join("target");
    fs::create_dir_all(target.join("policy/child")).expect("create package directory");
    let mut file = AssemblyFile {
        path: PathBuf::from("/policy"),
        kind: AssemblyFileKind::Content {
            content: "assembly".to_string(),
        },
        mode: None,
        replace: false,
    };

    let error =
        materialize_files(std::slice::from_ref(&file), temporary.path(), &target).unwrap_err();
    assert!(error.to_string().contains("use replace: true"));

    file.replace = true;
    materialize_files(&[file], temporary.path(), &target).expect("replace package directory");
    assert_eq!(
        fs::read_to_string(target.join("policy")).expect("read replacement file"),
        "assembly"
    );
}
