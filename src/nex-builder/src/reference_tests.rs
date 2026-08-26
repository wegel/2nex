//! Tests for package-reference parsing.

use super::{InputRef, PackageRef, RefKind};

#[test]
fn parses_package_members_without_segment_allocation() {
    let reference = PackageRef::parse("x86_64/pkg/libs/system/glibc/2.39/outputs/runtime")
        .expect("valid package reference");
    assert_eq!(reference.key.namespace, "libs/system");
    assert_eq!(reference.key.slug, "glibc");
    assert_eq!(reference.key.version, "2.39");
    assert_eq!(reference.kind, RefKind::Output("runtime".to_owned()));
}

#[test]
fn accepts_raw_hashes_and_files_refs() {
    assert!(matches!(
        InputRef::parse(&"a".repeat(64)).expect("valid stored hash"),
        InputRef::Stored(_)
    ));
    assert!(matches!(
        PackageRef::parse("x86_64/pkg/core/base/1/files")
            .expect("valid files ref")
            .kind,
        RefKind::Files
    ));
}

#[test]
fn dependency_refs_round_trip_canonically() {
    let value = "x86_64/pkg/core/base/1/bundles/runtime";
    let reference = InputRef::parse(value).expect("valid package input");
    assert_eq!(reference.to_string(), value);
    let yaml = serde_yaml::to_string(&reference).expect("serialize input ref");
    assert_eq!(
        serde_yaml::from_str::<InputRef>(&yaml).expect("parse input ref"),
        reference
    );
}

#[test]
fn rejects_arbitrary_dependency_strings() {
    assert!(InputRef::parse("systems/base/1").is_err());
    assert!(InputRef::parse("mutable-branch").is_err());
}

#[test]
fn rejects_paths_that_escape_the_catalog() {
    assert!(PackageRef::parse("x86_64/pkg/../passwd/1/files").is_err());
    assert!(PackageRef::parse("x86_64/pkg/core/base/1/outputs/../secret").is_err());
}
