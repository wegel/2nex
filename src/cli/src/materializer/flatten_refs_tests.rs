use std::collections::HashMap;

use crate::manifest::types::{Build, Manifest, Package};

use super::derive_files_commit_for_manifest;

#[test]
fn self_files_ref_uses_prefixed_namespace_once() {
    let manifest = Manifest {
        package: Package {
            name: "root".to_string(),
            slug: "root".to_string(),
            namespace: "pkg/apps".to_string(),
            version: "1.0".to_string(),
            checksum: Some("abc".to_string()),
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
    };

    let files_ref = derive_files_commit_for_manifest(&manifest);

    assert_eq!(
        files_ref.as_deref(),
        Some("x86_64/pkg/apps/root/1.0/abc/files")
    );
}
