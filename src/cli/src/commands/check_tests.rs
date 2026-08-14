use std::fs;
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::manifest::parser::load_manifest_from_str;
use crate::manifest::{ManifestData, ManifestIndex, Source};

use super::{
    validate_declared_sources, validate_manifest_refs, validate_provider_ref, validate_source_shape,
};

#[test]
fn source_shape_requires_one_primary_selector_and_canonical_sha256() {
    let source: Source = serde_yaml::from_str(
        r#"name: patch
url: https://example.test/fix.patch
file: pkg/apps/fix.patch
sha256: ABCD
"#,
    )
    .expect("source");
    let mut errors = Vec::new();

    validate_source_shape(&source, &mut errors);

    assert!(errors.iter().any(|error| error.contains("exactly one")));
    assert!(errors.iter().any(|error| error.contains("64 lowercase")));
}

#[test]
fn manifest_ref_requires_a_full_lowercase_git_object_id() {
    let manifest = load_manifest_from_str(&package_with_dependency("main"))
        .expect("package with invalid manifest ref");

    let errors = validate_manifest_refs(&manifest);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("full lowercase Git object ID"));
}

#[test]
fn declared_local_source_must_be_tracked_and_match_its_sha256() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repository = temp_dir.path().join("repo");
    let manifest_path = repository.join("pkg/apps/demo.yaml");
    let patch_path = repository.join("pkg/apps/demo.patch");
    fs::create_dir_all(manifest_path.parent().unwrap()).expect("package dir");
    git(&repository, &["init"]);
    fs::write(&patch_path, "patch bytes\n").expect("patch");
    git(&repository, &["add", "pkg/apps/demo.patch"]);

    let valid = source_manifest(&sha256("patch bytes\n"));
    fs::write(&manifest_path, &valid).expect("manifest");
    assert!(validate_declared_sources(&manifest_path, &valid)
        .expect("valid sources")
        .is_empty());

    let stale = source_manifest(&sha256("other bytes\n"));
    let errors = validate_declared_sources(&manifest_path, &stale).expect("source errors");
    assert!(errors.iter().any(|error| error.contains("sha256 mismatch")));
}

#[cfg(unix)]
#[test]
fn declared_local_source_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repository = temp_dir.path().join("repo");
    let manifest_path = repository.join("pkg/apps/demo.yaml");
    let patch_path = repository.join("pkg/apps/demo.patch");
    let link_path = repository.join("pkg/apps/demo-link.patch");
    fs::create_dir_all(manifest_path.parent().unwrap()).expect("package dir");
    git(&repository, &["init"]);
    fs::write(&patch_path, "patch bytes\n").expect("patch");
    symlink("demo.patch", &link_path).expect("patch symlink");
    git(
        &repository,
        &["add", "pkg/apps/demo.patch", "pkg/apps/demo-link.patch"],
    );
    let manifest =
        source_manifest(&sha256("patch bytes\n")).replace("demo.patch", "demo-link.patch");
    fs::write(&manifest_path, &manifest).expect("manifest");

    let errors = validate_declared_sources(&manifest_path, &manifest).expect("source errors");

    assert!(errors
        .iter()
        .any(|error| error.contains("without symlinks")));
}

#[test]
fn provider_ref_must_name_output_or_bundle() {
    let mut index = ManifestIndex::new();
    index.add_manifest(provider_manifest(
        r#"
outputs:
  graphics-runtime:
    provides:
    - graphics.egl
    files:
    - path: /usr/lib/libEGL.so.1
"#,
    ));

    let error = validate_provider_ref(
        "graphics.egl",
        "x86_64/pkg/libs/graphics/mesa/1.0/files",
        &index,
    )
    .expect_err("files ref should fail");

    assert!(error
        .to_string()
        .contains("must be an output or bundle ref"));
}

#[test]
fn provider_ref_must_declare_capability() {
    let mut index = ManifestIndex::new();
    index.add_manifest(provider_manifest(
        r#"
outputs:
  graphics-runtime:
    provides:
    - graphics.gbm
    files:
    - path: /usr/lib/libEGL.so.1
"#,
    ));

    let error = validate_provider_ref(
        "graphics.egl",
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/graphics-runtime",
        &index,
    )
    .expect_err("provider without capability should fail");

    assert!(error.to_string().contains("does not declare graphics.egl"));
}

#[test]
fn provider_ref_accepts_declared_output_capability() {
    let mut index = ManifestIndex::new();
    index.add_manifest(provider_manifest(
        r#"
outputs:
  graphics-runtime:
    provides:
    - graphics.egl
    files:
    - path: /usr/lib/libEGL.so.1
"#,
    ));

    validate_provider_ref(
        "graphics.egl",
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/graphics-runtime",
        &index,
    )
    .expect("declared provider should pass");
}

fn provider_manifest(body: &str) -> crate::manifest::Manifest {
    let yaml = format!(
        r#"
package:
  schema: 1
  name: mesa
  slug: mesa
  namespace: libs/graphics
  version: 1.0
sources: []
dependencies: []
build:
  environment: env/test.yaml
  script: "true"
bundles: {{}}
{body}
resolution: {{}}
"#,
    );
    match load_manifest_from_str(&yaml).expect("test manifest should parse") {
        ManifestData::Package(manifest) => manifest,
        ManifestData::System(_) => panic!("test manifest should be a package"),
    }
}

fn source_manifest(hash: &str) -> String {
    format!("sources:\n- name: patch\n  file: pkg/apps/demo.patch\n  sha256: {hash}\n")
}

fn package_with_dependency(manifest_ref: &str) -> String {
    format!(
        r#"package:
  name: Consumer
  slug: consumer
  namespace: apps
  version: 1
dependencies:
- commit: x86_64/pkg/apps/demo/1/bundles/full
  manifest_ref: {manifest_ref}
sources: []
build:
  environment: abcdef
  script: "true"
bundles: {{}}
outputs: {{}}
"#
    )
}

fn sha256(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn git(directory: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .expect("git command");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
