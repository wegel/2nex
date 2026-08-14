use std::fs;
use std::path::Path;

use super::ManifestIndex;

#[test]
fn empty_index_has_no_providers() {
    let index = ManifestIndex::new();

    assert_eq!(index.manifest_count(), 0);
    assert_eq!(index.file_count(), 0);
    assert!(index.resolve("libc.so.6").is_none());
}

#[test]
fn one_directory_rejects_duplicate_package_identity() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let package_dir = temp_dir.path().join("pkg/libs/system");
    fs::create_dir_all(&package_dir).expect("package dir");
    write_manifest(&package_dir.join("demo.yaml"));
    write_manifest(&package_dir.join("demo.debug.yaml"));

    let error = ManifestIndex::load(temp_dir.path().join("pkg"))
        .expect_err("duplicate package identity should fail");

    assert!(error
        .to_string()
        .contains("duplicate package manifest identity"));
    assert!(error.to_string().contains("pkg/libs/system/demo"));
}

#[test]
fn product_cannot_shadow_an_upstream_package() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product/pkg/libs/system");
    let upstream = temp_dir.path().join("upstream/pkg/libs/system");
    fs::create_dir_all(&product).expect("product package dir");
    fs::create_dir_all(&upstream).expect("upstream package dir");
    write_manifest(&product.join("demo.yaml"));
    write_manifest(&upstream.join("renamed-demo.yaml"));

    let error = ManifestIndex::load_many(&[
        temp_dir.path().join("product/pkg"),
        temp_dir.path().join("upstream/pkg"),
    ])
    .expect_err("cross-repository duplicate should fail");

    assert!(error.to_string().contains("product/pkg"));
    assert!(error.to_string().contains("upstream/pkg"));
}

fn write_manifest(path: &Path) {
    fs::write(path, manifest()).expect("package manifest");
}

fn manifest() -> &'static str {
    r#"package:
  schema: 1
  name: demo
  slug: demo
  namespace: libs/system
  version: 1.0

sources: []
dependencies: []

build:
  environment: env/test.yaml
  script: |
    touch "${OUT_DIR}/demo"

bundles:
  dev:
  - bin

outputs:
  bin:
    files:
    - path: /usr/bin/demo
"#
}
