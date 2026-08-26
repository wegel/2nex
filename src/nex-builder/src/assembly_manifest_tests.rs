//! Tests for retained assembly manifests.

use std::path::Path;

use walkdir::WalkDir;

use super::{
    load_assembly, validate_assembly, AssemblyFileKind, AssemblyManifest, PackagePlacement,
};

const ASSEMBLY: &str = r#"
system:
  schema: 1
  name: Test
  slug: test
  version: "1"
  description: Test assembly
  nex_structure: true
build:
  environment: unused
  script: ""
"#;

#[test]
fn every_retained_assembly_uses_the_supported_schema() {
    let root = repository_root().join("catalog/assemblies");
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
        load_assembly(entry.path()).unwrap_or_else(|error| {
            panic!("{}: {error}", entry.path().display());
        });
        count += 1;
    }
    assert!(count >= 9);
}

#[test]
fn nex_structure_is_not_silently_discarded() {
    let manifest: AssemblyManifest = serde_yaml::from_str(ASSEMBLY).expect("valid Nex assembly");

    assert!(manifest.system.nex_structure);
}

#[test]
fn assembly_schema_rejects_unsupported_versions() {
    let yaml = ASSEMBLY.replace("schema: 1", "schema: 999");
    let error = serde_yaml::from_str::<AssemblyManifest>(&yaml)
        .expect_err("schema version 999 must be rejected");
    assert!(error.to_string().contains("unsupported schema version 999"));
}

#[test]
fn assembly_schema_rejects_misspelled_layout_field() {
    let yaml = ASSEMBLY.replace("nex_structure", "nex_structur");
    let error = serde_yaml::from_str::<AssemblyManifest>(&yaml)
        .expect_err("a misspelled Nex layout field must be rejected");
    assert!(error.to_string().contains("unknown field `nex_structur`"));
}

#[test]
fn assembly_schema_rejects_unknown_file_kind() {
    let yaml = format!("{ASSEMBLY}files:\n- path: /etc/test\n  contents: test\n");
    let error = serde_yaml::from_str::<AssemblyManifest>(&yaml)
        .expect_err("a misspelled assembly file kind must be rejected");
    assert!(error.to_string().contains("did not match any variant"));
}

#[test]
fn assembly_file_has_one_resolved_kind() {
    let yaml = format!("{ASSEMBLY}files:\n- path: /etc/test\n  content: test\n");
    let manifest = serde_yaml::from_str::<AssemblyManifest>(&yaml).expect("valid content file");
    assert!(matches!(
        manifest.files[0].kind,
        AssemblyFileKind::Content { .. }
    ));

    let yaml = format!("{ASSEMBLY}files:\n- path: /etc/empty\n");
    let manifest = serde_yaml::from_str::<AssemblyManifest>(&yaml).expect("valid empty file");
    assert!(matches!(manifest.files[0].kind, AssemblyFileKind::Empty {}));
}

#[test]
fn assembly_file_rejects_multiple_kinds_and_false_directory() {
    let doubled =
        format!("{ASSEMBLY}files:\n- path: /etc/test\n  content: test\n  source: fixture\n");
    assert!(serde_yaml::from_str::<AssemblyManifest>(&doubled).is_err());

    let false_directory = format!("{ASSEMBLY}files:\n- path: /etc/test\n  directory: false\n");
    assert!(serde_yaml::from_str::<AssemblyManifest>(&false_directory).is_err());
}

#[test]
fn package_placement_applies_only_to_nex_assemblies() {
    let package = "packages:\n- commit: x86_64/pkg/tests/tool/1/files\n  placement: root\n";
    let nex = serde_yaml::from_str::<AssemblyManifest>(&format!("{ASSEMBLY}{package}"))
        .expect("valid root-placed Nex package");
    assert_eq!(nex.packages[0].placement, Some(PackagePlacement::Root));
    let serialized = serde_yaml::to_string(&nex.packages[0]).expect("serialize package selection");
    assert!(serialized.contains("placement: root"));
    assert!(!serialized.contains("dependency:"));

    let flat_yaml = format!(
        "{}{package}",
        ASSEMBLY.replace("  nex_structure: true\n", "")
    );
    let flat = serde_yaml::from_str::<AssemblyManifest>(&flat_yaml).expect("valid wire schema");
    assert!(validate_assembly(&flat).is_err());
}

#[test]
fn package_placement_rejects_unknown_fields() {
    let package = "packages:\n- commit: x86_64/pkg/tests/tool/1/files\n  placemen: root\n";
    assert!(serde_yaml::from_str::<AssemblyManifest>(&format!("{ASSEMBLY}{package}")).is_err());
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| path.join("catalog").is_dir())
        .expect("repository root")
}
