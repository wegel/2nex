use serde_yaml::Value;

use super::format::format_manifest_string;

#[test]
fn formats_graphics_providers_after_packages() {
    let input = r#"system:
  schema: 1
  name: test system
  slug: test
  version: 1.0
packages:
- name: vwl
  commit: x86_64/pkg/desktop/compositor/vwl/0.1.0/outputs/bin
providers:
  graphics.gles: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
  graphics.egl: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
build:
  environment: env/test.yaml
  script: "true"
"#;

    let formatted = format_manifest_string(input).expect("formatting should succeed");
    let packages_index = formatted
        .find("\npackages:\n")
        .expect("formatted manifest should contain packages");
    let providers_index = formatted
        .find("\nproviders:\n")
        .expect("formatted manifest should contain providers");
    let build_index = formatted
        .find("\nbuild:\n")
        .expect("formatted manifest should contain build");

    assert!(packages_index < providers_index);
    assert!(providers_index < build_index);
    assert!(formatted.contains(
        "providers:\n  graphics.egl: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime\n  graphics.gles: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime\n"
    ));
    serde_yaml::from_str::<Value>(&formatted).expect("formatted YAML should parse");
}

#[test]
fn formats_output_provides_before_files() {
    let input = r#"package:
  schema: 1
  name: mesa
  slug: mesa
  namespace: libs/graphics
  version: 24.2.7
sources: []
dependencies: []
build:
  environment: env/test.yaml
  script: "true"
bundles:
  runtime:
  - graphics-runtime
outputs:
  graphics-runtime:
    files:
    - path: /usr/lib/libEGL.so.1
    provides:
    - graphics.gles
    - graphics.egl
resolution: {}
"#;

    let formatted = format_manifest_string(input).expect("formatting should succeed");

    assert!(formatted.contains(
        "  graphics-runtime:\n    provides:\n    - graphics.egl\n    - graphics.gles\n    files:\n    - path: /usr/lib/libEGL.so.1\n"
    ));
    serde_yaml::from_str::<Value>(&formatted).expect("formatted YAML should parse");
}

#[test]
fn formats_capability_resolution_entries() {
    let input = r#"package:
  schema: 1
  name: vwl
  slug: vwl
  namespace: desktop/compositor
  version: 0.1.0
sources: []
dependencies: []
build:
  environment: env/test.yaml
  script: "true"
bundles:
  runtime:
  - bin
outputs:
  bin:
    files:
    - path: /usr/bin/vwl
      needs:
      - /usr/lib/libEGL.so.1
      - /usr/lib/libc.so.6
resolution:
  /usr/lib/libc.so.6: glibc
  /usr/lib/libEGL.so.1:
    fallback: mesa
    capability: graphics.egl
"#;

    let formatted = format_manifest_string(input).expect("formatting should succeed");

    assert!(formatted.contains(
        "resolution:\n  /usr/lib/libc.so.6: glibc\n  /usr/lib/libEGL.so.1:\n    capability: graphics.egl\n    fallback: mesa\n"
    ));
    serde_yaml::from_str::<Value>(&formatted).expect("formatted YAML should parse");
}
