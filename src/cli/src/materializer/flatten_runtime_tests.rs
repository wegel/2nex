use std::io;

use crate::manifest::{load_manifest_from_str, ManifestData, ManifestIndex};

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

#[test]
fn flatten_fails_when_self_files_commit_cannot_be_derived() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("usr/bin")).unwrap();
    std::fs::write(temp_dir.path().join("usr/bin/app"), b"").unwrap();
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "apps",
        "root",
        r#"
dependencies: []
outputs:
  bin:
    files:
    - path: /usr/bin/app
      needs:
      - /usr/lib/libself.so
resolution:
  /usr/lib/libself.so: self
"#,
    ));

    let error = flatten_capsule_precomputed(
        "missing-repo",
        temp_dir.path(),
        "x86_64/pkg/apps/root/1.0/outputs/bin",
        &index,
        &[],
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("self runtime files"));
    assert!(error.to_string().contains("/usr/lib/libself.so"));
}

#[test]
fn flatten_fails_when_dependency_files_commit_cannot_be_derived() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("usr/bin")).unwrap();
    std::fs::write(temp_dir.path().join("usr/bin/app"), b"").unwrap();
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "apps",
        "root",
        r#"
dependencies:
- name: dep
  commit: x86_64/pkg/libs/dep/1.0/outputs/lib
outputs:
  bin:
    files:
    - path: /usr/bin/app
      needs:
      - /usr/lib/libdep.so
resolution:
  /usr/lib/libdep.so: dep
"#,
    ));
    index.add_manifest(package_manifest(
        "libs",
        "dep",
        r#"
dependencies: []
outputs:
  lib:
    files:
    - path: /usr/lib/libdep.so
resolution: {}
"#,
    ));

    let error = flatten_capsule_precomputed(
        "missing-repo",
        temp_dir.path(),
        "x86_64/pkg/apps/root/1.0/outputs/bin",
        &index,
        &[],
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("no files commit can be derived"));
    assert!(error.to_string().contains("dep"));
}

#[test]
fn flatten_fails_when_resolution_names_undeclared_dependency() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("usr/bin")).unwrap();
    std::fs::write(temp_dir.path().join("usr/bin/app"), b"").unwrap();
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "apps",
        "root",
        r#"
dependencies: []
outputs:
  bin:
    files:
    - path: /usr/bin/app
      needs:
      - /usr/lib/libdep.so
resolution:
  /usr/lib/libdep.so: dep
"#,
    ));

    let error = flatten_capsule_precomputed(
        "missing-repo",
        temp_dir.path(),
        "x86_64/pkg/apps/root/1.0/outputs/bin",
        &index,
        &[],
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("not declared"));
    assert!(error.to_string().contains("/usr/lib/libdep.so"));
}

fn package_manifest(namespace: &str, slug: &str, body: &str) -> crate::manifest::Manifest {
    let yaml = format!(
        r#"
package:
  schema: 1
  name: {slug}
  slug: {slug}
  namespace: {namespace}
  version: 1.0
sources: []
build:
  environment: env/test.yaml
  script: "true"
bundles: {{}}
{body}
"#
    );
    match load_manifest_from_str(&yaml).unwrap() {
        ManifestData::Package(manifest) => manifest,
        ManifestData::System(_) => panic!("expected package manifest"),
    }
}
