use crate::commands::build::BuildOpts;
use std::collections::BTreeMap;

use crate::manifest::{Build, SystemManifest, SystemMeta};

use super::ensure_check_checksum_allows_system_publish;

fn opts(check: bool, update_checksum: bool) -> BuildOpts {
    BuildOpts {
        repo_path: ".nex/repo".to_string(),
        manifest_file: "asm/test.yaml".to_string(),
        manifest_dirs: vec!["pkg".into()],
        writable_manifest_root: None,
        check,
        update_checksum,
        compute_deps: false,
        runtime_deps_verbose: false,
        refresh_metadata: false,
        force: false,
        build_dir: None,
        generate_outputs: false,
        fallback_repos: Vec::new(),
        verbose: false,
        record_profile: false,
        no_progress: true,
        dry_run: false,
        add_checksums: false,
        show_dep_paths: false,
        trace_dependency: None,
        multi_progress: None,
        reuse_rootfs: false,
    }
}

fn manifest(checksum: Option<&str>) -> SystemManifest {
    SystemManifest {
        schema: Some(1),
        system: SystemMeta {
            name: "test".to_string(),
            slug: "test".to_string(),
            version: "1.0".to_string(),
            architecture: None,
            boot_method: None,
            description: None,
            checksum: checksum.map(ToOwned::to_owned),
            stable_checksum: None,
            nex_structure: false,
            extends: None,
        },
        packages: Vec::new(),
        providers: BTreeMap::new(),
        dependencies: Vec::new(),
        sources: Vec::new(),
        files: Vec::new(),
        build: Build {
            environment: "env/test.yaml".to_string(),
            script: "true".to_string(),
            profile: Vec::new(),
        },
        exclude: None,
    }
}

#[test]
fn check_without_update_rejects_stale_system_checksum() {
    let error = ensure_check_checksum_allows_system_publish(
        &opts(true, false),
        &manifest(Some("old")),
        "new",
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("--update-checksum"));
}

#[test]
fn check_with_update_allows_stale_system_checksum() {
    ensure_check_checksum_allows_system_publish(&opts(true, true), &manifest(Some("old")), "new")
        .expect("test setup should succeed");
}
