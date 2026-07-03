use std::collections::BTreeMap;
use std::io;

use crate::manifest::{load_manifest_from_str, ManifestData, ManifestIndex};

use super::super::flatten::flatten_capsule_precomputed;
use super::capability_provider_files;

fn empty_providers() -> BTreeMap<String, String> {
    BTreeMap::new()
}

#[test]
fn flatten_fails_when_capability_provider_is_unbound() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    write_capsule_file(temp_dir.path(), "usr/bin/app");
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "apps",
        "root",
        r#"
dependencies:
- name: mesa
  commit: x86_64/pkg/libs/graphics/mesa/1.0/outputs/graphics-runtime
outputs:
  bin:
    files:
    - path: /usr/bin/app
      needs:
      - /usr/lib/libEGL.so.1
resolution:
  /usr/lib/libEGL.so.1:
    capability: graphics.egl
    fallback: mesa
"#,
    ));

    let error = flatten_capsule_precomputed(
        "missing-repo",
        temp_dir.path(),
        "x86_64/pkg/apps/root/1.0/outputs/bin",
        &index,
        &empty_providers(),
        &[],
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("graphics.egl"));
    assert!(error.to_string().contains("not bound"));
}

#[test]
fn capability_provider_files_requires_declared_capability() {
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "libs/graphics",
        "mesa",
        r#"
dependencies: []
outputs:
  graphics-runtime:
    provides:
    - graphics.gbm
    files:
    - path: /usr/lib/libEGL.so.1
resolution: {}
"#,
    ));

    let error = capability_provider_files(
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/graphics-runtime",
        "graphics.egl",
        &index,
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("does not provide graphics.egl"));
}

#[test]
fn capability_provider_files_returns_all_output_files() {
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "libs/graphics",
        "mesa",
        r#"
dependencies: []
outputs:
  graphics-runtime:
    provides:
    - graphics.egl
    files:
    - path: /usr/lib/libEGL.so.1
    - path: /usr/share/glvnd/egl_vendor.d/50_mesa.json
resolution: {}
"#,
    ));

    let files = capability_provider_files(
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/graphics-runtime",
        "graphics.egl",
        &index,
    )
    .expect("provider files should resolve");

    assert_eq!(
        files,
        vec![
            "/usr/lib/libEGL.so.1".to_string(),
            "/usr/share/glvnd/egl_vendor.d/50_mesa.json".to_string()
        ]
    );
}

#[test]
fn capability_provider_files_prefers_capability_file_list() {
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "libs/graphics",
        "mesa",
        r#"
dependencies: []
outputs:
  lib:
    provides:
    - graphics.egl
    - graphics.gbm
    capability_files:
      graphics.gbm:
      - /usr/lib/libgbm.so.1
    files:
    - path: /usr/lib/libEGL.so.1
    - path: /usr/lib/libgbm.so.1
resolution: {}
"#,
    ));

    let files = capability_provider_files(
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/lib",
        "graphics.gbm",
        &index,
    )
    .expect("provider files should resolve");

    assert_eq!(files, vec!["/usr/lib/libgbm.so.1".to_string()]);
}

#[test]
fn capability_provider_files_rejects_unknown_capability_file() {
    let mut index = ManifestIndex::default();
    index.add_manifest(package_manifest(
        "libs/graphics",
        "mesa",
        r#"
dependencies: []
outputs:
  lib:
    provides:
    - graphics.gbm
    capability_files:
      graphics.gbm:
      - /usr/lib/missing.so
    files:
    - path: /usr/lib/libgbm.so.1
resolution: {}
"#,
    ));

    let error = capability_provider_files(
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/lib",
        "graphics.gbm",
        &index,
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("not in that output"));
}

fn package_manifest(namespace: &str, slug: &str, body: &str) -> crate::manifest::Manifest {
    let checksum_line = "";
    let yaml = format!(
        r#"
package:
  schema: 1
  name: {slug}
  slug: {slug}
  namespace: {namespace}
  version: 1.0
{checksum_line}sources: []
{body}
build:
  environment: env/test.yaml
  script: "true"
bundles: {{}}
"#,
    );
    match load_manifest_from_str(&yaml).expect("test manifest should parse") {
        ManifestData::Package(manifest) => manifest,
        ManifestData::System(_) => panic!("test manifest should be a package"),
    }
}

fn write_capsule_file(root: &std::path::Path, relative_path: &str) {
    let path = root.join(relative_path);
    std::fs::create_dir_all(path.parent().expect("test path should have a parent"))
        .expect("test setup should succeed");
    std::fs::write(path, b"").expect("test setup should succeed");
}
