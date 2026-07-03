use crate::manifest::parser::load_manifest_from_str;
use crate::manifest::{ManifestData, ManifestIndex};

use super::validate_provider_ref;

#[test]
fn provider_ref_must_name_output_or_bundle() {
    let mut index = ManifestIndex::new();
    index.add_manifest(provider_manifest(
        r#"
outputs:
  graphics-runtime:
    provides:
    - graphics.egl
    files:
    - path: /usr/lib/libEGL.so.1
"#,
    ));

    let error = validate_provider_ref(
        "graphics.egl",
        "x86_64/pkg/libs/graphics/mesa/1.0/files",
        &index,
    )
    .expect_err("files ref should fail");

    assert!(error
        .to_string()
        .contains("must be an output or bundle ref"));
}

#[test]
fn provider_ref_must_declare_capability() {
    let mut index = ManifestIndex::new();
    index.add_manifest(provider_manifest(
        r#"
outputs:
  graphics-runtime:
    provides:
    - graphics.gbm
    files:
    - path: /usr/lib/libEGL.so.1
"#,
    ));

    let error = validate_provider_ref(
        "graphics.egl",
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/graphics-runtime",
        &index,
    )
    .expect_err("provider without capability should fail");

    assert!(error.to_string().contains("does not declare graphics.egl"));
}

#[test]
fn provider_ref_accepts_declared_output_capability() {
    let mut index = ManifestIndex::new();
    index.add_manifest(provider_manifest(
        r#"
outputs:
  graphics-runtime:
    provides:
    - graphics.egl
    files:
    - path: /usr/lib/libEGL.so.1
"#,
    ));

    validate_provider_ref(
        "graphics.egl",
        "x86_64/pkg/libs/graphics/mesa/1.0/outputs/graphics-runtime",
        &index,
    )
    .expect("declared provider should pass");
}

fn provider_manifest(body: &str) -> crate::manifest::Manifest {
    let yaml = format!(
        r#"
package:
  schema: 1
  name: mesa
  slug: mesa
  namespace: libs/graphics
  version: 1.0
sources: []
dependencies: []
build:
  environment: env/test.yaml
  script: "true"
bundles: {{}}
{body}
resolution: {{}}
"#,
    );
    match load_manifest_from_str(&yaml).expect("test manifest should parse") {
        ManifestData::Package(manifest) => manifest,
        ManifestData::System(_) => panic!("test manifest should be a package"),
    }
}
