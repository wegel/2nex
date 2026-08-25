use std::fs;
use std::io;
use std::path::PathBuf;

use super::{layer_base_commit, resolve_base_commit};
use crate::commands::build::BuildOpts;
use crate::manifest::SystemBase;
use crate::store::{commit_tree, Store};

/// Scenario: another build moves the base's semantic ref after the child starts.
/// The child must still layer the exact Zub commit that it resolved at the start.
#[test]
fn resolved_base_commit_does_not_move_during_the_child_build() -> io::Result<()> {
    let temp = tempfile::tempdir()?;
    let store_path = temp.path().join("store");
    if let Err(error) = Store::init(&store_path) {
        eprintln!("skipping Zub test because this filesystem cannot initialize a store: {error}");
        return Ok(());
    }
    let first_tree = temp.path().join("first");
    write_marker(&first_tree, "first\n")?;
    commit_tree(
        store_path.to_string_lossy().as_ref(),
        "systems/base/1",
        &first_tree,
        &[],
    )?;
    let opts = opts(&store_path);
    let resolved = resolve_base_commit(Some(&base()), &opts)?.expect("base commit");

    let second_tree = temp.path().join("second");
    write_marker(&second_tree, "second\n")?;
    commit_tree(
        store_path.to_string_lossy().as_ref(),
        "systems/base/1",
        &second_tree,
        &[],
    )?;
    let build_root = temp.path().join("build");
    layer_base_commit(
        Some(&resolved),
        &opts,
        build_root.to_string_lossy().as_ref(),
    )?;

    assert_eq!(
        fs::read_to_string(build_root.join("target/usr/share/base-marker"))?,
        "first\n"
    );
    Ok(())
}

fn base() -> SystemBase {
    SystemBase {
        commit: "systems/base/1".to_string(),
        manifest: PathBuf::from("base/base.yaml"),
    }
}

fn write_marker(root: &std::path::Path, contents: &str) -> io::Result<()> {
    fs::create_dir_all(root.join("usr/share"))?;
    fs::write(root.join("usr/share/base-marker"), contents)
}

fn opts(store_path: &std::path::Path) -> BuildOpts {
    BuildOpts {
        repo_path: store_path.to_string_lossy().into_owned(),
        manifest_file: "asm/child.yaml".to_string(),
        manifest_dirs: Vec::new(),
        writable_manifest_root: None,
        check: false,
        update_checksum: false,
        compute_deps: false,
        runtime_deps_verbose: false,
        refresh_metadata: false,
        force: false,
        build_dir: None,
        generate_outputs: false,
        fallback_repos: Vec::new(),
        verbose: false,
        record_profile: false,
        no_progress: true,
        dry_run: false,
        add_checksums: false,
        show_dep_paths: false,
        trace_dependency: None,
        multi_progress: None,
        reuse_rootfs: false,
    }
}
