use std::collections::BTreeMap;
use std::fs;
use std::io;

use crate::commands::build::BuildOpts;
use crate::manifest::{
    AssemblyFile, Build, BuildEnvironment, BuildPaths, ExecutionConfig, ManifestIndex, SystemBase,
    SystemManifest, SystemMeta,
};
use crate::store::{commit_tree, Store};

use super::{build_system_once, ensure_check_checksum_allows_system_publish, SystemBuildInputs};

fn opts(check: bool, update_checksum: bool) -> BuildOpts {
    BuildOpts {
        repo_path: ".nex/repo".to_string(),
        manifest_file: "asm/test.yaml".to_string(),
        manifest_dirs: vec!["pkg".into()],
        writable_manifest_root: None,
        check,
        update_checksum,
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

fn manifest(checksum: Option<&str>) -> SystemManifest {
    SystemManifest {
        schema: Some(1),
        system: SystemMeta {
            name: "test".to_string(),
            slug: "test".to_string(),
            version: "1.0".to_string(),
            architecture: None,
            boot_method: None,
            description: None,
            checksum: checksum.map(ToOwned::to_owned),
            stable_checksum: None,
            nex_structure: false,
        },
        base: None,
        packages: Vec::new(),
        providers: BTreeMap::new(),
        dependencies: Vec::new(),
        sources: Vec::new(),
        files: Vec::new(),
        build: Build {
            environment: "env/test.yaml".to_string(),
            script: "true".to_string(),
            profile: Vec::new(),
        },
    }
}

#[test]
fn check_without_update_rejects_stale_system_checksum() {
    let error = ensure_check_checksum_allows_system_publish(
        &opts(true, false),
        &manifest(Some("old")),
        "new",
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("--update-checksum"));
}

#[test]
fn check_with_update_allows_stale_system_checksum() {
    ensure_check_checksum_allows_system_publish(&opts(true, true), &manifest(Some("old")), "new")
        .expect("test setup should succeed");
}

/// Scenario: a child has no packages of its own and adds one file above a realized base.
/// The assembly build must retain the base file while it applies the child's file entry.
#[test]
fn child_build_keeps_the_realized_base_tree() -> io::Result<()> {
    let temp = tempfile::tempdir()?;
    let store_path = temp.path().join("store");
    if let Err(error) = Store::init(&store_path) {
        eprintln!("skipping Zub test because this filesystem cannot initialize a store: {error}");
        return Ok(());
    }
    let base_tree = temp.path().join("base-tree");
    fs::create_dir_all(base_tree.join("usr/share"))?;
    fs::write(base_tree.join("usr/share/base-marker"), "base\n")?;
    commit_tree(
        store_path.to_string_lossy().as_ref(),
        "systems/base/1",
        &base_tree,
        &[],
    )?;
    let store = Store::open(&store_path)?;
    let base_commit = store.resolve_ref("systems/base/1")?;
    let mut options = opts(false, false);
    options.repo_path = store_path.to_string_lossy().into_owned();
    options.manifest_dirs.clear();
    let mut child = manifest(None);
    child.base = Some(SystemBase {
        commit: "systems/base/1".to_string(),
        manifest: "base/base.yaml".into(),
    });
    child.files.push(AssemblyFile {
        path: "/usr/share/child-marker".into(),
        mode: None,
        content: Some("child\n".to_string()),
        source: None,
        symlink: None,
        directory: false,
        replace: false,
        base_dir: None,
        upstream_dir: None,
    });
    child.build.script.clear();
    let inputs = SystemBuildInputs {
        manifest_index: ManifestIndex::default(),
        dependency_commits: Vec::new(),
        package_commits: Vec::new(),
        original_package_commits: Vec::new(),
        build_env: host_environment(),
        use_absolute_paths: true,
        base_commit: Some(base_commit),
    };
    let build_root = temp.path().join("build");

    build_system_once(
        &options,
        &child,
        build_root.to_string_lossy().as_ref(),
        temp.path().join("downloads").to_string_lossy().as_ref(),
        &inputs,
        false,
    )?;

    assert_eq!(
        fs::read_to_string(build_root.join("target/usr/share/base-marker"))?,
        "base\n"
    );
    assert_eq!(
        fs::read_to_string(build_root.join("target/usr/share/child-marker"))?,
        "child\n"
    );
    Ok(())
}

fn host_environment() -> BuildEnvironment {
    BuildEnvironment {
        name: "test".to_string(),
        description: String::new(),
        execution: ExecutionConfig { chroot: false },
        env: Default::default(),
        preamble: String::new(),
        paths: BuildPaths {
            work: "work".to_string(),
            out: "out".to_string(),
            inputs: "inputs".to_string(),
        },
    }
}
