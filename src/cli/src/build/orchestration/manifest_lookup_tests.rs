use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::find_manifest_for_commit;

const NEX_UTILITIES_REF: &str = "x86_64/pkg/core/userland/nex-utilities/0.0.1/outputs/bin";

#[test]
fn finds_manifest_by_declared_slug_when_filename_differs() {
    let root = TempDir::new().expect("create manifest root");
    let manifest_path = write_manifest(root.path(), "2nex-utilities.yaml", "nex-utilities");

    let found = find_manifest_for_commit(NEX_UTILITIES_REF, &[root.path().to_path_buf()])
        .expect("find manifest from its declared slug");

    assert_eq!(found, manifest_path.canonicalize().expect("canonical path"));
}

#[test]
fn rejects_duplicate_declared_slugs() {
    let root = TempDir::new().expect("create manifest root");
    write_manifest(root.path(), "first.yaml", "nex-utilities");
    write_manifest(root.path(), "second.yaml", "nex-utilities");

    let error = find_manifest_for_commit(NEX_UTILITIES_REF, &[root.path().to_path_buf()])
        .expect_err("duplicate package slugs should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        error.to_string().contains("duplicate manifests"),
        "unexpected error: {error}"
    );
}

fn write_manifest(root: &Path, filename: &str, slug: &str) -> std::path::PathBuf {
    let namespace = root.join("pkg/core/userland");
    fs::create_dir_all(&namespace).expect("create namespace");
    let path = namespace.join(filename);
    fs::write(
        &path,
        format!("package:\n  schema: 1\n  slug: {slug}\n  version: 0.0.1\n"),
    )
    .expect("write manifest");
    path
}
