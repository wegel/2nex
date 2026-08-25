use std::collections::BTreeMap;

use super::system_metadata;
use crate::manifest::{Build, SystemBase, SystemManifest, SystemMeta};

/// Scenario: a child records which realized base tree supplied its inherited files.
/// Nex must store the exact Zub commit, not only the moving semantic ref from YAML.
#[test]
fn child_metadata_records_the_exact_base_commit() {
    let manifest = SystemManifest {
        schema: Some(1),
        system: SystemMeta {
            name: "child".to_string(),
            slug: "child".to_string(),
            version: "1".to_string(),
            architecture: None,
            boot_method: None,
            description: None,
            checksum: None,
            stable_checksum: None,
            nex_structure: false,
        },
        base: Some(SystemBase {
            commit: "systems/base/1".to_string(),
            manifest: "base/base.yaml".into(),
        }),
        packages: Vec::new(),
        providers: BTreeMap::new(),
        dependencies: Vec::new(),
        sources: Vec::new(),
        files: Vec::new(),
        build: Build {
            environment: "abcdef".to_string(),
            script: String::new(),
            profile: Vec::new(),
        },
    };

    let metadata = system_metadata(&manifest, &[], &[], Some("0123456789abcdef"), "checksum")
        .expect("metadata");

    assert!(metadata.contains(&(
        "nex.system.base".to_string(),
        "0123456789abcdef".to_string()
    )));
}
