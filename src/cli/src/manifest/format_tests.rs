use std::io::ErrorKind;

use serde_yaml::Value;

use super::format::format_manifest_string;
use super::{Overlay, OverlayEntry};

#[test]
fn formats_overlay_without_losing_files_or_comments() {
    let input = r#"files:
# locale policy
- path: /etc/locale.conf
  replace: true
  content: |
    LANG=en_GB.UTF-8
  mode: 420

# resolver policy
- path: /etc/resolv.conf
  symlink: /run/systemd/resolve/stub-resolv.conf
"#;

    let formatted = format_manifest_string(input).expect("overlay should format");
    assert!(
        formatted.contains(
            "files:\n# locale policy\n- path: /etc/locale.conf\n  mode: 420\n  content: |\n    LANG=en_GB.UTF-8\n  replace: true\n"
        ),
        "formatter lost the locale entry or its comment:\n{formatted}"
    );
    assert!(formatted.contains(
        "# resolver policy\n- path: /etc/resolv.conf\n  symlink: /run/systemd/resolve/stub-resolv.conf\n"
    ));

    let overlay: Overlay =
        serde_yaml::from_str(&formatted).expect("formatted overlay should parse");
    assert_eq!(overlay.files.len(), 2);
    assert_eq!(overlay.files[0].mode, Some(420));
    assert_eq!(
        overlay.files[0].content.as_deref(),
        Some("LANG=en_GB.UTF-8\n")
    );
    assert!(overlay.files[0].replace);
}

#[test]
fn preserves_overlay_content_chomping() {
    for content in [
        "no newline",
        "one newline\n",
        "two newlines\n\n",
        "first\n\nthird\n",
        "\n",
        "",
    ] {
        let input = serde_yaml::to_string(&Overlay {
            files: vec![OverlayEntry {
                path: "/test".into(),
                mode: None,
                content: Some(content.to_string()),
                source: None,
                symlink: None,
                directory: false,
                replace: false,
            }],
        })
        .expect("test overlay should serialize");

        let formatted = format_manifest_string(&input).expect("overlay should format");
        let overlay: Overlay =
            serde_yaml::from_str(&formatted).expect("formatted overlay should parse");
        assert_eq!(
            overlay.files[0].content.as_deref(),
            Some(content),
            "formatter changed this overlay:\n{formatted}"
        );
        assert!(
            !formatted.lines().any(|line| line == "    "),
            "formatter added whitespace to a blank line:\n{formatted}"
        );
    }
}

#[test]
fn rejects_unknown_manifest_roots_instead_of_erasing_them() {
    let error = format_manifest_string("unexpected: value\n")
        .expect_err("formatter should reject an unknown manifest root");
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(error
        .to_string()
        .contains("manifest must contain package, system, or files"));
}

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
