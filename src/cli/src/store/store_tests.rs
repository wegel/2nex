//! Tests for zub store behavior.

use std::fs;
use std::io;
use std::path::Path;

use tempfile::TempDir;

use super::{commit_tree, remote_source, RemoteSource, Store};

#[test]
fn resolve_ref_pulls_from_configured_local_remote() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let source_repo = temp_dir.path().join("source");
    let dest_repo = temp_dir.path().join("dest");
    let tree = temp_dir.path().join("tree");
    let ref_name = "systems/demo/0.0.1";
    let metadata = [("nex.system.checksum".to_string(), checksum('d'))];

    if !init_store(&source_repo)? || !init_store(&dest_repo)? {
        return Ok(());
    }

    fs::create_dir_all(tree.join("usr/bin"))?;
    fs::write(tree.join("usr/bin/demo"), "demo")?;
    if !commit_test_tree(&source_repo, ref_name, &tree, &metadata)? {
        return Ok(());
    }
    write_configured_local_remote(&dest_repo, &source_repo)?;

    let store = Store::open(&dest_repo)?;
    let commit_hash = store.resolve_ref(ref_name)?;

    assert_eq!(commit_hash.len(), 64);
    assert_eq!(
        store.get_metadata(ref_name, "nex.system.checksum")?,
        Some(checksum('d'))
    );
    assert!(store.exists(ref_name));
    Ok(())
}

#[test]
fn checkout_can_be_modified_without_changing_store_objects() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    let tree = temp_dir.path().join("tree");
    let first_checkout = temp_dir.path().join("first-checkout");
    let second_checkout = temp_dir.path().join("second-checkout");
    let ref_name = "pkg/demo/1.0/files";

    if !init_store(&repo_path)? {
        return Ok(());
    }

    fs::create_dir_all(&tree)?;
    fs::write(tree.join("database"), "stored data")?;
    if !commit_test_tree(&repo_path, ref_name, &tree, &[])? {
        return Ok(());
    }

    let store = Store::open(&repo_path)?;
    store.checkout(ref_name, &first_checkout, false)?;
    fs::write(first_checkout.join("database"), "local change")?;

    store.checkout(ref_name, &second_checkout, false)?;
    assert_eq!(
        fs::read_to_string(second_checkout.join("database"))?,
        "stored data"
    );
    Ok(())
}

#[test]
fn remote_source_keeps_ssh_and_local_urls_distinct() {
    match remote_source("ssh://builder.example/var/zub") {
        RemoteSource::Ssh { remote, path } => {
            assert_eq!(remote, "builder.example");
            assert_eq!(path, Path::new("/var/zub"));
        }
        RemoteSource::Local(path) => panic!("expected SSH remote, got {}", path.display()),
    }

    match remote_source("builder@example:/srv/zub") {
        RemoteSource::Ssh { remote, path } => {
            assert_eq!(remote, "builder@example");
            assert_eq!(path, Path::new("/srv/zub"));
        }
        RemoteSource::Local(path) => panic!("expected SSH remote, got {}", path.display()),
    }

    match remote_source("/var/nex/repo") {
        RemoteSource::Local(path) => assert_eq!(path, Path::new("/var/nex/repo")),
        RemoteSource::Ssh { remote, path } => {
            panic!("expected local remote, got {remote}:{}", path.display());
        }
    }
}

fn init_store(repo_path: &Path) -> io::Result<bool> {
    match Store::init(repo_path) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => {
            eprintln!("skipping store test: host cannot map uid 0");
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn commit_test_tree(
    repo_path: &Path,
    ref_name: &str,
    tree_dir: &Path,
    metadata: &[(String, String)],
) -> io::Result<bool> {
    match commit_tree(
        &repo_path.display().to_string(),
        ref_name,
        tree_dir,
        metadata,
    ) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => {
            eprintln!("skipping store test: host cannot map uid 0");
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn write_configured_local_remote(dest_repo: &Path, source_repo: &Path) -> io::Result<()> {
    fs::write(
        dest_repo.join("config.toml"),
        format!(
            "\
[namespace]
uid_map = []
gid_map = []

[[remotes]]
name = \"local\"
url = \"{}\"
",
            source_repo.display(),
        ),
    )
}

fn host_lacks_root_user_namespace_mapping(error: &io::Error) -> bool {
    error.to_string().contains("uid 0 not mapped in namespace")
}

fn checksum(fill: char) -> String {
    std::iter::repeat(fill).take(64).collect()
}
