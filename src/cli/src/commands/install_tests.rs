//! Tests for package installation build planning.

use std::fs;
use std::io;
use std::path::Path;

use tempfile::TempDir;

use crate::store::Store;

use super::build_package_to_user_repo_with_mode;

#[test]
fn install_builds_missing_dependency_closure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("store");
    if let Err(error) = Store::init(&repo_path) {
        if error.to_string().contains("uid 0 not mapped in namespace") {
            eprintln!("skipping install graph test: host cannot map uid 0");
            return Ok(());
        }
        return Err(error);
    }

    let leaf = temp_dir.path().join("pkg/test/leaf.yaml");
    let root = temp_dir.path().join("pkg/test/root.yaml");
    write_manifest(&leaf, leaf_manifest())?;
    write_manifest(&root, root_manifest())?;

    build_package_to_user_repo_with_mode(
        repo_path.to_str().expect("store path should be UTF-8"),
        &[],
        &root,
        &[temp_dir.path().to_path_buf()],
        true,
    )
}

fn write_manifest(path: &Path, contents: &str) -> io::Result<()> {
    fs::create_dir_all(path.parent().expect("manifest should have a parent"))?;
    fs::write(path, contents)
}

fn leaf_manifest() -> &'static str {
    r#"package:
  schema: 1
  name: leaf
  slug: leaf
  namespace: test
  version: "1.0"
dependencies: []
sources: []
build:
  environment: env/test.yaml
  script: "exit 97"
outputs:
  bin:
    files:
    - path: /usr/bin/leaf
bundles: {}
"#
}

fn root_manifest() -> &'static str {
    r#"package:
  schema: 1
  name: root
  slug: root
  namespace: test
  version: "1.0"
dependencies:
- name: leaf
  commit: x86_64/pkg/test/leaf/1.0/outputs/bin
sources: []
build:
  environment: env/test.yaml
  script: "exit 98"
outputs:
  bin:
    files:
    - path: /usr/bin/root
bundles: {}
"#
}
