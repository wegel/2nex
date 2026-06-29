use std::collections::HashMap;

use crate::commands::build::BuildOpts;
use crate::manifest::{Build, Manifest, Package};

use super::ensure_check_checksum_allows_package_publish;

fn opts(check: bool, update_checksum: bool) -> BuildOpts {
    BuildOpts {
        repo_path: ".nex/repo".to_string(),
        manifest_file: "pkg/test.yaml".to_string(),
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

fn manifest(checksum: Option<&str>) -> Manifest {
    Manifest {
        package: Package {
            name: "test".to_string(),
            slug: "test".to_string(),
            namespace: "apps".to_string(),
            version: "1.0".to_string(),
            checksum: checksum.map(ToOwned::to_owned),
            stable_checksum: None,
            seed: false,
        },
        dependencies: Vec::new(),
        sources: Vec::new(),
        build: Build {
            environment: "env/test.yaml".to_string(),
            script: "true".to_string(),
            profile: Vec::new(),
        },
        outputs: HashMap::new(),
        bundles: HashMap::new(),
        resolution: HashMap::new(),
    }
}

#[test]
fn check_without_update_rejects_stale_package_checksum() {
    let error = ensure_check_checksum_allows_package_publish(
        &opts(true, false),
        &manifest(Some("old")),
        "new",
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("--update-checksum"));
}

#[test]
fn check_with_update_allows_stale_package_checksum() {
    ensure_check_checksum_allows_package_publish(&opts(true, true), &manifest(Some("old")), "new")
        .expect("test setup should succeed");
}
