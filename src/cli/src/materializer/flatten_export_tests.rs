use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use crate::store::{commit_tree, Store};

use super::super::flatten_export::flatten_library_preserving_path;

#[test]
fn flatten_library_exports_chained_relative_symlink_targets() -> io::Result<()> {
    let temp_dir = tempfile::TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    if !init_test_store(&repo_path)? {
        return Ok(());
    }

    let tree_dir = temp_dir.path().join("tree");
    std::fs::create_dir_all(tree_dir.join("usr/lib/real"))?;
    std::fs::write(tree_dir.join("usr/lib/real/libdemo.so"), "demo")?;
    symlink("real/libdemo.so", tree_dir.join("usr/lib/libdemo-real.so"))?;
    symlink("libdemo-real.so", tree_dir.join("usr/lib/libdemo.so"))?;

    let files_ref = "x86_64/pkg/libs/demo/1.0/abcdef/files";
    if !commit_test_tree(&repo_path, files_ref, &tree_dir)? {
        return Ok(());
    }

    let pkg_dir = temp_dir.path().join("pkg");
    let flattened = flatten_library_preserving_path(
        &repo_path.display().to_string(),
        files_ref,
        "/usr/lib/libdemo.so",
        &pkg_dir,
        &[],
    )?;

    assert!(flattened);
    assert_eq!(
        std::fs::read_link(pkg_dir.join("usr/lib/libdemo.so"))?,
        std::path::PathBuf::from("libdemo-real.so")
    );
    assert_eq!(
        std::fs::read_link(pkg_dir.join("usr/lib/libdemo-real.so"))?,
        std::path::PathBuf::from("real/libdemo.so")
    );
    assert_eq!(
        std::fs::read_to_string(pkg_dir.join("usr/lib/real/libdemo.so"))?,
        "demo"
    );
    Ok(())
}

#[test]
fn flatten_library_rejects_relative_symlink_target_that_escapes_package() -> io::Result<()> {
    let temp_dir = tempfile::TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    if !init_test_store(&repo_path)? {
        return Ok(());
    }

    let tree_dir = temp_dir.path().join("tree");
    std::fs::create_dir_all(tree_dir.join("usr/lib"))?;
    symlink(
        "../../../outside/libdemo.so",
        tree_dir.join("usr/lib/libdemo.so"),
    )?;

    let files_ref = "x86_64/pkg/libs/demo/1.0/abcdef/files";
    if !commit_test_tree(&repo_path, files_ref, &tree_dir)? {
        return Ok(());
    }

    let error = flatten_library_preserving_path(
        &repo_path.display().to_string(),
        files_ref,
        "/usr/lib/libdemo.so",
        &temp_dir.path().join("pkg"),
        &[],
    )
    .expect_err("escaping symlink target should be rejected");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("escapes checkout root"));
    Ok(())
}

fn init_test_store(repo_path: &Path) -> io::Result<bool> {
    match Store::init(repo_path) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => {
            eprintln!("skipping flatten export test: host cannot map uid 0");
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn host_lacks_root_user_namespace_mapping(error: &io::Error) -> bool {
    error.to_string().contains("uid 0 not mapped in namespace")
}

fn commit_test_tree(repo_path: &Path, files_ref: &str, tree_dir: &Path) -> io::Result<bool> {
    match commit_tree(&repo_path.display().to_string(), files_ref, tree_dir, &[]) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => {
            eprintln!("skipping flatten export test: host cannot map uid 0");
            Ok(false)
        }
        Err(error) => Err(error),
    }
}
