use std::fs;
use std::io;
use std::path::PathBuf;

use super::system_base_source;
use crate::manifest::{ManifestSource, SystemBase};

/// Scenario: a developer edits the base manifest without committing it.
/// Nex must read that working-tree file so the next child build sees the edit.
#[test]
fn base_without_a_revision_uses_the_live_file() -> io::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir_all(root.join(".git"))?;
    fs::create_dir_all(root.join("asm"))?;
    fs::create_dir_all(root.join("base"))?;
    let child = root.join("asm/child.yaml");
    let base_path = root.join("base/system.yaml");
    fs::write(&child, "system: {}\n")?;
    fs::write(&base_path, "system: {}\n")?;

    let source = system_base_source(&base("base/system.yaml"), &ManifestSource::Path(child))?;

    assert_eq!(source, ManifestSource::Path(base_path.canonicalize()?));
    Ok(())
}

/// Scenario: a manifest tries to name a base outside its Git repository.
/// Nex must reject the path instead of reading an undeclared host file.
#[test]
fn base_path_cannot_escape_its_repository() -> io::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir_all(root.join(".git"))?;
    fs::create_dir_all(root.join("asm"))?;
    let child = root.join("asm/child.yaml");
    fs::write(&child, "system: {}\n")?;

    let error = system_base_source(&base("../outside.yaml"), &ManifestSource::Path(child))
        .expect_err("a base path must stay inside its repository");

    assert!(error.to_string().contains("repository-relative"), "{error}");
    Ok(())
}

fn base(manifest: &str) -> SystemBase {
    SystemBase {
        commit: "systems/base/1".to_string(),
        manifest: PathBuf::from(manifest),
    }
}
