//! Tests for retained package manifests.

use std::path::Path;

use walkdir::WalkDir;

use super::{load_manifest, PackageManifest, Source};
use crate::profile::BuildProfile;

const PACKAGE: &str = r#"
package:
  schema: 1
  name: Test
  slug: test
  namespace: test
  version: "1"
  description: Test package
build:
  environment: unused
  script: ""
outputs:
  bin:
    files:
    - path: /usr/bin/test
"#;

#[test]
fn every_retained_package_uses_the_supported_schema() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| path.join("catalog").is_dir())
        .expect("repository root")
        .join("catalog/pkg");
    let manifests = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|value| value == "yaml")
        });
    let mut count = 0;
    for entry in manifests {
        let manifest = load_manifest(entry.path()).unwrap_or_else(|error| {
            panic!("{}: {error}", entry.path().display());
        });
        if !manifest.build.profile.is_empty() {
            BuildProfile::parse(&manifest.build.profile).unwrap_or_else(|error| {
                panic!("{}: {error}", entry.path().display());
            });
        }
        count += 1;
    }
    assert!(count > 500);
}

#[test]
fn source_type_requires_exactly_one_kind() {
    let hash = "a".repeat(64);
    let missing = format!("name: source\nsha256: {hash}\n");
    let doubled = format!("name: source\nsha256: {hash}\nurl: a\nfile: b\n");
    assert!(serde_yaml::from_str::<Source>(&missing).is_err());
    assert!(serde_yaml::from_str::<Source>(&doubled).is_err());
}

#[test]
fn package_schema_rejects_unsupported_versions() {
    let yaml = PACKAGE.replace("schema: 1", "schema: 999");
    let error = serde_yaml::from_str::<PackageManifest>(&yaml)
        .expect_err("schema version 999 must be rejected");
    assert!(error.to_string().contains("unsupported schema version 999"));
}

#[test]
fn package_schema_rejects_unknown_build_fields() {
    let yaml = PACKAGE.replace("script: \"\"", "script: \"\"\n  scrpit: \"\"");
    let error = serde_yaml::from_str::<PackageManifest>(&yaml)
        .expect_err("a misspelled build field must be rejected");
    assert!(error.to_string().contains("unknown field `scrpit`"));
}

#[test]
fn package_schema_rejects_unknown_output_fields() {
    let yaml = PACKAGE.replace("  bin:\n", "  bin:\n    filez: []\n");
    let error = serde_yaml::from_str::<PackageManifest>(&yaml)
        .expect_err("a misspelled output field must be rejected");
    assert!(error.to_string().contains("unknown field `filez`"));
}

#[test]
fn bundle_members_form_a_set() {
    let manifest: PackageManifest = serde_yaml::from_str(
        r#"package: {schema: 1, name: Example, slug: example, namespace: test, version: 1, description: Test package}
build: {environment: unused, script: ""}
outputs: {bin: {files: [{path: /usr/bin/example}]}}
bundles: {full: [bin, bin]}
"#,
    )
    .expect("valid package with duplicate set input");

    assert_eq!(manifest.bundles["full"].len(), 1);
}
