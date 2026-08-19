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
fn product_assembly_extends_upstream_and_keeps_each_file_owner() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product");
    let upstream = product.join("upstream/nex");
    let child_path = product.join("asm/device.yaml");
    let base_path = upstream.join("asm/base.yaml");
    fs::create_dir_all(product.join(".git")).expect("product git marker");
    fs::create_dir_all(upstream.join(".git")).expect("upstream git marker");
    fs::create_dir_all(child_path.parent().unwrap()).expect("product asm dir");
    fs::create_dir_all(base_path.parent().unwrap()).expect("upstream asm dir");
    fs::write(&base_path, assembly("base", None, "/etc/base.conf")).expect("base assembly");
    fs::write(
        &child_path,
        assembly(
            "device",
            Some("upstream/nex/asm/base.yaml"),
            "/etc/device.conf",
        ),
    )
    .expect("child assembly");

    let resolved = resolve_inheritance(&child_path).expect("resolved assembly");

    let paths: Vec<_> = resolved
        .files
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    assert_eq!(
        paths,
        vec![
            std::path::PathBuf::from("/etc/base.conf"),
            std::path::PathBuf::from("/etc/device.conf"),
        ]
    );
    let owners: Vec<_> = resolved
        .files
        .iter()
        .map(|entry| entry.base_dir.clone().expect("owner"))
        .collect();
    assert_eq!(owners, vec![upstream.clone(), product.clone()]);
}

fn package(name: &str, commit: &str) -> SystemPackage {
    SystemPackage {
        commit: commit.to_string(),
        name: Some(name.to_string()),
        manifest_ref: None,
    }
}

#[test]
fn resolved_child_keeps_its_own_checksum_and_drops_the_parent_one() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repository = temp_dir.path().join("repo");
    let child_path = repository.join("asm/child.yaml");
    let base_path = repository.join("asm/base.yaml");
    fs::create_dir_all(repository.join(".git")).expect("git marker");
    fs::create_dir_all(child_path.parent().unwrap()).expect("asm dir");
    fs::write(
        &base_path,
        assembly_with_checksum("base", None, "/etc/base.conf", Some("aaaa")),
    )
    .expect("base assembly");
    fs::write(
        &child_path,
        assembly_with_checksum("child", Some("asm/base.yaml"), "/etc/child.conf", Some("bbbb")),
    )
    .expect("child assembly");

    let resolved = resolve_inheritance(&child_path).expect("resolved assembly");

    assert_eq!(resolved.system.checksum.as_deref(), Some("bbbb"));
}

#[test]
fn resolved_child_without_a_checksum_records_none() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repository = temp_dir.path().join("repo");
    let child_path = repository.join("asm/child.yaml");
    let base_path = repository.join("asm/base.yaml");
    fs::create_dir_all(repository.join(".git")).expect("git marker");
    fs::create_dir_all(child_path.parent().unwrap()).expect("asm dir");
    fs::write(
        &base_path,
        assembly_with_checksum("base", None, "/etc/base.conf", Some("aaaa")),
    )
    .expect("base assembly");
    fs::write(
        &child_path,
        assembly_with_checksum("child", Some("asm/base.yaml"), "/etc/child.conf", None),
    )
    .expect("child assembly");

    let resolved = resolve_inheritance(&child_path).expect("resolved assembly");

    assert_eq!(resolved.system.checksum, None);
}

fn assembly_with_checksum(
    slug: &str,
    extends: Option<&str>,
    file_path: &str,
    checksum: Option<&str>,
) -> String {
    let checksum = checksum
        .map(|value| format!("  checksum: {value}\n"))
        .unwrap_or_default();
    assembly(slug, extends, file_path).replace(
        &format!("  version: 1.0\n"),
        &format!("  version: 1.0\n{checksum}"),
    )
}

fn assembly(slug: &str, extends: Option<&str>, file_path: &str) -> String {
    let extends = extends
        .map(|path| format!("  extends: {path}\n"))
        .unwrap_or_default();
    format!(
        "system:\n  name: {slug}\n  slug: {slug}\n  version: 1.0\n{extends}\npackages:\n- name: demo\n  commit: x86_64/pkg/demo/1.0/outputs/bin\n\nfiles:\n- path: {file_path}\n  content: \"{slug}\\n\"\n\nbuild:\n  environment: abcdef\n  script: \"\"\n"
    )
}
