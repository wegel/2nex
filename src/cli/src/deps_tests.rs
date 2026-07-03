use std::collections::BTreeMap;

use crate::manifest::parser::load_manifest_from_str;
use crate::manifest::{Dependency, ManifestData, ManifestIndex};

use super::{resolve_dependency_closure, resolve_dependency_closure_with_providers};

#[test]
fn test_empty_dependencies() {
    let index = ManifestIndex::new();
    let deps: Vec<Dependency> = vec![];
    let result = resolve_dependency_closure(&deps, &index).expect("closure should resolve");
    assert!(result.is_empty());
}

#[test]
fn package_closure_uses_capability_fallback_without_system_providers() {
    let mut index = ManifestIndex::new();
    index.add_manifest(demo_manifest());
    index.add_manifest(provider_manifest("mesa"));
    let deps = demo_dependency();

    let resolved = resolve_dependency_closure(&deps, &index).expect("closure should resolve");

    assert!(resolved
        .contains(&"x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime".to_string()));
}

#[test]
fn system_closure_uses_bound_provider_for_capability() {
    let mut index = ManifestIndex::new();
    index.add_manifest(demo_manifest());
    index.add_manifest(provider_manifest("mesa"));
    index.add_manifest(provider_manifest("nvidia-580"));
    let deps = demo_dependency();
    let mut providers = BTreeMap::new();
    providers.insert(
        "graphics.egl".to_string(),
        "x86_64/pkg/libs/graphics/nvidia-580/24.2.7/outputs/graphics-runtime".to_string(),
    );

    let resolved = resolve_dependency_closure_with_providers(&deps, &index, &providers)
        .expect("closure should resolve");

    assert!(resolved.contains(
        &"x86_64/pkg/libs/graphics/nvidia-580/24.2.7/outputs/graphics-runtime".to_string()
    ));
    assert!(!resolved
        .contains(&"x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime".to_string()));
}

#[test]
fn system_closure_rejects_unbound_capability() {
    let mut index = ManifestIndex::new();
    index.add_manifest(demo_manifest());
    let deps = demo_dependency();

    let error = resolve_dependency_closure_with_providers(&deps, &index, &BTreeMap::new())
        .expect_err("unbound capability should fail");

    assert!(error
        .to_string()
        .contains("unbound capability graphics.egl"));
}

fn demo_manifest() -> crate::manifest::Manifest {
    package_manifest(
        "apps/demo",
        "demo",
        r#"
dependencies:
- name: mesa
  commit: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
outputs:
  bin:
    files:
    - path: /usr/bin/demo
      needs:
      - /usr/lib/libEGL.so.1
resolution:
  /usr/lib/libEGL.so.1:
    capability: graphics.egl
    fallback: mesa
"#,
    )
}

fn demo_dependency() -> Vec<Dependency> {
    vec![Dependency {
        name: Some("demo".to_string()),
        commit: "x86_64/pkg/apps/demo/demo/1.0/outputs/bin".to_string(),
        manifest_ref: None,
    }]
}

fn package_manifest(namespace: &str, slug: &str, body: &str) -> crate::manifest::Manifest {
    let yaml = format!(
        r#"
package:
  schema: 1
  name: {slug}
  slug: {slug}
  namespace: {namespace}
  version: 1.0
sources: []
{body}
build:
  environment: env/test.yaml
  script: "true"
bundles: {{}}
"#,
    );
    match load_manifest_from_str(&yaml).expect("test manifest should parse") {
        ManifestData::Package(manifest) => manifest,
        ManifestData::System(_) => panic!("test manifest should be a package"),
    }
}

fn provider_manifest(slug: &str) -> crate::manifest::Manifest {
    package_manifest(
        "libs/graphics",
        slug,
        r#"
dependencies: []
outputs:
  graphics-runtime:
    provides:
    - graphics.egl
    files:
    - path: /usr/lib/libEGL.so.1
resolution: {}
"#,
    )
}
