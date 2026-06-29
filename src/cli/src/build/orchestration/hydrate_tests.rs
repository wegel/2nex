use crate::manifest::types::Dependency;

use super::{format_dependency_section, hydrated_dependency_name};

#[test]
fn hydrated_dependency_names_use_package_slug_not_pkg_segment() {
    let name =
        hydrated_dependency_name("x86_64/pkg/libs/system/glibc/2.39/bundles/dev", &[]).unwrap();

    assert_eq!(name.as_deref(), Some("glibc"));
}

#[test]
fn hydrated_dependency_names_preserve_direct_names() {
    let direct = Dependency {
        name: Some("libc".to_string()),
        commit: "x86_64/pkg/libs/system/glibc/2.39/bundles/dev".to_string(),
        manifest_ref: None,
    };

    let name = hydrated_dependency_name(&direct.commit, std::slice::from_ref(&direct)).unwrap();

    assert_eq!(name.as_deref(), Some("libc"));
}

#[test]
fn formatted_hydrated_dependencies_use_meaningful_names() {
    let commit = "x86_64/pkg/libs/system/glibc/2.39/bundles/dev".to_string();
    let dependency = Dependency {
        name: hydrated_dependency_name(&commit, &[]).unwrap(),
        commit,
        manifest_ref: None,
    };

    let section = format_dependency_section(&[dependency]);

    assert_eq!(
        section,
        vec![
            "dependencies:".to_string(),
            "  - name: glibc".to_string(),
            "    commit: x86_64/pkg/libs/system/glibc/2.39/bundles/dev".to_string(),
        ]
    );
}
