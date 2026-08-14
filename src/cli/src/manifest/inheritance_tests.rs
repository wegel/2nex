use std::fs;

use super::{merge_packages, resolve_inheritance, should_exclude_package};
use crate::manifest::{ExcludeSpec, SystemPackage};

#[test]
fn excludes_package_by_name() {
    let package = package("foo", "x86_64/pkg/foo/1.0/outputs/bin");
    let excludes = vec![ExcludeSpec::ByName {
        name: "foo".to_string(),
    }];

    assert!(should_exclude_package(&package, &excludes));
}

#[test]
fn excludes_package_by_commit() {
    let package = package("foo", "x86_64/pkg/foo/1.0/outputs/bin");
    let excludes = vec![ExcludeSpec::ByCommit {
        commit: package.commit.clone(),
    }];

    assert!(should_exclude_package(&package, &excludes));
}

#[test]
fn keeps_package_without_matching_exclusion() {
    let package = package("foo", "x86_64/pkg/foo/1.0/outputs/bin");
    let excludes = vec![ExcludeSpec::ByName {
        name: "bar".to_string(),
    }];

    assert!(!should_exclude_package(&package, &excludes));
}

#[test]
fn child_package_replaces_same_named_parent() {
    let base = vec![package("foo", "old-commit")];
    let child = vec![package("foo", "new-commit")];

    let result = merge_packages(&base, &child, &[]);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].commit, "new-commit");
}

#[test]
fn child_package_appends_a_different_name() {
    let base = vec![package("foo", "foo-commit")];
    let child = vec![package("bar", "bar-commit")];

    let result = merge_packages(&base, &child, &[]);

    assert_eq!(result.len(), 2);
}

#[test]
fn product_assembly_extends_upstream_and_keeps_each_overlay_owner() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product");
    let upstream = product.join("upstream/nex");
    let child_path = product.join("asm/device.yaml");
    let base_path = upstream.join("asm/base.yaml");
    fs::create_dir_all(product.join(".git")).expect("product git marker");
    fs::create_dir_all(upstream.join(".git")).expect("upstream git marker");
    fs::create_dir_all(child_path.parent().unwrap()).expect("product asm dir");
    fs::create_dir_all(base_path.parent().unwrap()).expect("upstream asm dir");
    fs::write(&base_path, assembly("base", None, "asm/base-overlay.yaml")).expect("base assembly");
    fs::write(
        &child_path,
        assembly(
            "device",
            Some("upstream/nex/asm/base.yaml"),
            "asm/device-overlay.yaml",
        ),
    )
    .expect("child assembly");

    let resolved = resolve_inheritance(&child_path).expect("resolved assembly");

    assert_eq!(
        resolved.overlays,
        vec![
            upstream.join("asm/base-overlay.yaml"),
            product.join("asm/device-overlay.yaml"),
        ]
    );
}

fn package(name: &str, commit: &str) -> SystemPackage {
    SystemPackage {
        commit: commit.to_string(),
        name: Some(name.to_string()),
    }
}

fn assembly(slug: &str, extends: Option<&str>, overlay: &str) -> String {
    let extends = extends
        .map(|path| format!("  extends: {path}\n"))
        .unwrap_or_default();
    format!(
        "system:\n  name: {slug}\n  slug: {slug}\n  version: 1.0\n{extends}\noverlays:\n- {overlay}\n\npackages:\n- name: demo\n  commit: x86_64/pkg/demo/1.0/outputs/bin\n\nbuild:\n  environment: abcdef\n  script: \"\"\n"
    )
}
