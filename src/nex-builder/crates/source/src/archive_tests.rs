//! Tests for deterministic archive creation and safe tree copies.

use std::fs;
use std::io;
use std::os::unix::fs::symlink;

use super::{copy_tree, create};
use crate::cache;

#[test]
fn empty_vendor_archive_keeps_its_known_checksum() {
    let temporary = tempfile::tempdir().unwrap();
    let vendor = temporary.path().join("vendor");
    let output = temporary.path().join("vendor.tar.gz");
    fs::create_dir(&vendor).unwrap();
    create(&vendor, &output).unwrap();
    assert_eq!(
        cache::sha256(&output).unwrap(),
        "758df15397b144a35710a7350baa632e73197ece8cfd952e3f0120f1b09df2be"
    );
}

#[test]
fn copy_tree_rejects_directory_link_cycles() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    let child = source.join("child");
    fs::create_dir_all(&child).unwrap();
    symlink(&source, child.join("back")).unwrap();

    let error = copy_tree(&source, &temporary.path().join("copy")).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}
