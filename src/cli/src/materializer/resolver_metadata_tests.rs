use std::collections::HashMap;

use crate::manifest::types::{Build, Bundle, Manifest, OutputSpec, Package};

use super::missing_runtime_metadata;

#[test]
fn reports_missing_bundle_metadata() {
    let manifest = manifest_with_outputs(["bin"]);

    let missing = missing_runtime_metadata("x86_64/pkg/apps/demo/1.0/bundles/full", &manifest);

    assert_eq!(missing, vec!["bundle 'full'".to_string()]);
}

#[test]
fn reports_missing_bundle_output_metadata() {
    let mut manifest = manifest_with_outputs(["bin"]);
    manifest.bundles.insert(
        "full".to_string(),
        Bundle {
            includes: vec!["bin".to_string(), "lib".to_string()],
        },
    );

    let missing = missing_runtime_metadata("x86_64/pkg/apps/demo/1.0/bundles/full", &manifest);

    assert_eq!(missing, vec!["output 'lib' in bundle 'full'".to_string()]);
}

#[test]
fn reports_missing_output_metadata() {
    let manifest = manifest_with_outputs(["bin"]);

    let missing = missing_runtime_metadata("x86_64/pkg/apps/demo/1.0/outputs/lib", &manifest);

    assert_eq!(
        missing,
        vec!["output 'lib' in x86_64/pkg/apps/demo/1.0/outputs/lib".to_string()]
    );
}

fn manifest_with_outputs<const N: usize>(outputs: [&str; N]) -> Manifest {
    let outputs = outputs
        .into_iter()
        .map(|name| {
            (
                name.to_string(),
                OutputSpec {
                    provides: Vec::new(),
                    capability_files: std::collections::BTreeMap::new(),
                    files: Vec::new(),
                },
            )
        })
        .collect();

    Manifest {
        package: Package {
            name: "demo".to_string(),
            slug: "demo".to_string(),
            version: "1.0".to_string(),
            namespace: "apps".to_string(),
            checksum: Some("abc".to_string()),
            stable_checksum: None,
            seed: false,
        },
        dependencies: Vec::new(),
        sources: Vec::new(),
        build: Build {
            environment: String::new(),
            script: String::new(),
            profile: Vec::new(),
        },
        outputs,
        bundles: HashMap::new(),
        resolution: HashMap::new(),
    }
}
