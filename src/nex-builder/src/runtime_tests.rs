//! Tests for runtime input closure.

use std::path::PathBuf;

use super::{resolve, Providers};
use crate::manifest::{Dependency, PackageManifest};
use crate::reference::InputRef;

#[test]
fn tool_bundle_pulls_its_runtime_library_into_the_build_root() {
    let (attr, patch) = runtime_manifests();
    let direct = [dependency(
        "patch",
        "x86_64/pkg/core/toolchain/patch/1.0/bundles/dev",
    )];
    let inputs = resolve_inputs(&direct, &attr, &patch);

    assert_eq!(
        commits(&inputs),
        [
            "x86_64/pkg/libs/system/attr/1.0/bundles/dev",
            "x86_64/pkg/core/toolchain/patch/1.0/bundles/dev",
        ]
    );
    assert_eq!(inputs[0].paths, [PathBuf::from("/usr/lib/libattr.so.1")]);
}

#[test]
fn direct_inputs_keep_their_manifest_order() {
    let (attr, patch) = runtime_manifests();
    let direct = [
        dependency("patch", "x86_64/pkg/core/toolchain/patch/1.0/bundles/dev"),
        dependency("attr", "x86_64/pkg/libs/system/attr/1.0/bundles/dev"),
    ];

    assert_eq!(
        commits(&resolve_inputs(&direct, &attr, &patch)),
        [
            "x86_64/pkg/core/toolchain/patch/1.0/bundles/dev",
            "x86_64/pkg/libs/system/attr/1.0/bundles/dev",
        ]
    );
}

#[test]
fn selected_output_pulls_a_needed_output_from_the_same_package() {
    let package = manifest(
        r#"package: {schema: 1, name: Tool, slug: tool, namespace: core/toolchain, version: 1.0, description: Test package}
build: {environment: unused, script: ""}
outputs:
  bin:
    files:
    - path: /usr/bin/tool
      needs: [/usr/lib/libtool.so.1]
  lib:
    files:
    - path: /usr/lib/libtool.so.1
resolution:
  /usr/lib/libtool.so.1: self
"#,
    );
    let direct = [dependency(
        "tool",
        "x86_64/pkg/core/toolchain/tool/1.0/outputs/bin",
    )];
    let inputs =
        resolve(&direct, Providers::Fallback, |_| Ok(&package)).expect("resolve self runtime file");

    assert_eq!(
        commits(&inputs),
        [
            "x86_64/pkg/core/toolchain/tool/1.0/files",
            "x86_64/pkg/core/toolchain/tool/1.0/outputs/bin",
        ]
    );
    assert_eq!(inputs[0].paths, [PathBuf::from("/usr/lib/libtool.so.1")]);
}

fn runtime_manifests() -> (PackageManifest, PackageManifest) {
    let attr = manifest(
        r#"package: {schema: 1, name: Attr, slug: attr, namespace: libs/system, version: 1.0, description: Test package}
build: {environment: unused, script: ""}
bundles: {dev: [lib]}
outputs:
  lib:
    files:
    - path: /usr/lib/libattr.so.1
"#,
    );
    let patch = manifest(
        r#"package: {schema: 1, name: Patch, slug: patch, namespace: core/toolchain, version: 1.0, description: Test package}
dependencies:
- name: attr
  commit: x86_64/pkg/libs/system/attr/1.0/bundles/dev
build: {environment: unused, script: ""}
bundles: {dev: [bin]}
outputs:
  bin:
    files:
    - path: /usr/bin/patch
      needs: [/usr/lib/libattr.so.1]
resolution:
  /usr/lib/libattr.so.1: attr
"#,
    );
    (attr, patch)
}

fn resolve_inputs<'a>(
    direct: &[Dependency],
    attr: &'a PackageManifest,
    patch: &'a PackageManifest,
) -> Vec<Dependency> {
    resolve(direct, Providers::Fallback, |reference| {
        match reference.key.slug.as_str() {
            "attr" => Ok(attr),
            "patch" => Ok(patch),
            slug => panic!("unexpected package {slug}"),
        }
    })
    .expect("resolve runtime closure")
}

fn commits(inputs: &[Dependency]) -> Vec<String> {
    inputs
        .iter()
        .map(|dependency| dependency.commit.to_string())
        .collect()
}

fn manifest(yaml: &str) -> PackageManifest {
    serde_yaml::from_str(yaml).expect("valid runtime test manifest")
}

fn dependency(name: &str, commit: &str) -> Dependency {
    Dependency {
        name: Some(name.to_owned()),
        commit: InputRef::parse(commit).expect("valid test dependency ref"),
        paths: Vec::new(),
    }
}
