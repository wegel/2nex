use std::io;

use crate::manifest::ManifestIndex;

use super::{flatten_capsule_precomputed, flatten_export_error};

#[test]
fn flatten_export_error_names_commit_and_path() {
    let error = flatten_export_error(
        "x86_64/pkg/libs/example/1.0/hash/files",
        "/usr/lib/libmissing.so.1",
        io::Error::new(io::ErrorKind::NotFound, "not in commit"),
    );

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    let message = error.to_string();
    assert!(message.contains("/usr/lib/libmissing.so.1"));
    assert!(message.contains("x86_64/pkg/libs/example/1.0/hash/files"));
    assert!(message.contains("not in commit"));
}

#[test]
fn flatten_fails_when_root_manifest_is_missing() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let index = ManifestIndex::default();

    let error = flatten_capsule_precomputed(
        "missing-repo",
        temp_dir.path(),
        "x86_64/pkg/apps/example/1.0/outputs/bin",
        &index,
        &[],
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(error.to_string().contains("no manifest was found"));
}
