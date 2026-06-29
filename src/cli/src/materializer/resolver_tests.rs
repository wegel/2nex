use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::manifest::types::{Build, FileEntry, Manifest, OutputSpec, Package};
use crate::manifest::ManifestIndex;
use crate::store::{commit_tree, Store};

use super::super::resolver_entries::file_entries_to_process;
use super::{self_files_commit, RuntimeClosure, RuntimeResolver};

fn manifest_with_lib_output() -> Manifest {
    let mut outputs = HashMap::new();
    outputs.insert(
        "lib".to_string(),
        OutputSpec {
            files: vec![
                FileEntry {
                    path: "/usr/lib/libX11.so.6".to_string(),
                    needs: vec!["/usr/lib/libxcb.so.1".to_string()],
                },
                FileEntry {
                    path: "/usr/lib/libX11-xcb.so.1".to_string(),
                    needs: Vec::new(),
                },
            ],
        },
    );

    Manifest {
        package: Package {
            name: "libX11".to_string(),
            slug: "libx11".to_string(),
            version: "1.8.10".to_string(),
            namespace: "libs/x11".to_string(),
            checksum: Some("abc".to_string()),
            stable_checksum: None,
            seed: false,
        },
        dependencies: Vec::new(),
        sources: Vec::new(),
        build: Build {
            environment: String::new(),
            script: String::new(),
            profile: Vec::new(),
        },
        outputs,
        bundles: HashMap::new(),
        resolution: HashMap::new(),
    }
}

#[test]
fn files_commit_processes_needed_manifest_entries_once() {
    let manifest = manifest_with_lib_output();
    let commit = "x86_64/pkg/libs/x11/libx11/1.8.10/abc/files";
    let mut closure = RuntimeClosure::default();
    closure.add_file_dep(commit, "/usr/lib/libX11.so.6", "test".to_string());

    let mut processed_file_paths = HashMap::new();
    let entries = file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
    assert_eq!(
        entries.entries,
        vec![(
            "/usr/lib/libX11.so.6".to_string(),
            vec!["/usr/lib/libxcb.so.1".to_string()]
        )]
    );
    assert!(entries.missing_files.is_empty());

    let entries = file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
    assert!(entries.entries.is_empty());
    assert!(entries.missing_files.is_empty());

    closure.add_file_dep(
        commit,
        "/usr/lib/libX11-xcb.so.1",
        "another need".to_string(),
    );
    let entries = file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
    assert_eq!(
        entries.entries,
        vec![("/usr/lib/libX11-xcb.so.1".to_string(), Vec::new())]
    );
    assert!(entries.missing_files.is_empty());
}

#[test]
fn files_commit_reports_requested_files_missing_manifest_metadata() {
    let manifest = manifest_with_lib_output();
    let commit = "x86_64/pkg/libs/x11/libx11/1.8.10/abc/files";
    let mut closure = RuntimeClosure::default();
    closure.add_file_dep(commit, "/usr/lib/libmissing.so.1", "test".to_string());
    let mut processed_file_paths = HashMap::new();

    let entries = file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);

    assert!(entries.entries.is_empty());
    assert_eq!(entries.missing_files, vec!["/usr/lib/libmissing.so.1"]);
}

#[test]
fn output_commit_derives_self_files_ref() {
    let manifest = manifest_with_lib_output();
    let commit = "x86_64/pkg/libs/x11/libx11/1.8.10/outputs/bin";

    let files_ref = self_files_commit(commit, &manifest);

    assert_eq!(
        files_ref.as_deref(),
        Some("x86_64/pkg/libs/x11/libx11/1.8.10/abc/files")
    );
}

#[test]
fn files_commit_uses_current_self_files_ref() {
    let manifest = manifest_with_lib_output();
    let commit = "x86_64/pkg/libs/x11/libx11/1.8.10/abc/files";

    let files_ref = self_files_commit(commit, &manifest);

    assert_eq!(files_ref.as_deref(), Some(commit));
}

fn host_lacks_root_user_namespace_mapping(error: &io::Error) -> bool {
    error.to_string().contains("uid 0 not mapped in namespace")
}

fn manifest_with_self_runtime_output() -> Manifest {
    let mut manifest = manifest_with_lib_output();
    manifest.outputs.insert(
        "bin".to_string(),
        OutputSpec {
            files: vec![FileEntry {
                path: "/usr/bin/demo".to_string(),
                needs: vec!["/usr/lib/libX11.so.6".to_string()],
            }],
        },
    );
    manifest
        .outputs
        .get_mut("lib")
        .unwrap()
        .files
        .push(FileEntry {
            path: "/usr/lib/libxcb.so.1".to_string(),
            needs: Vec::new(),
        });
    manifest
        .resolution
        .insert("/usr/lib/libX11.so.6".to_string(), "self".to_string());
    manifest
        .resolution
        .insert("/usr/lib/libxcb.so.1".to_string(), "self".to_string());
    manifest
}

fn commit_self_runtime_files_ref(
    temp_dir: &tempfile::TempDir,
    repo_path: &Path,
    files_ref: &str,
) -> io::Result<bool> {
    let tree_dir = temp_dir.path().join("files-tree");
    std::fs::create_dir_all(tree_dir.join("usr/lib"))?;
    std::fs::write(tree_dir.join("usr/lib/libX11.so.6"), b"x11")?;
    std::fs::write(tree_dir.join("usr/lib/libxcb.so.1"), b"xcb")?;
    let repo_path = repo_path.display().to_string();
    match commit_tree(&repo_path, files_ref, &tree_dir, &[]) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

#[test]
fn output_commit_queues_derived_self_files_ref() -> io::Result<()> {
    let temp_dir = tempfile::TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    if let Err(error) = Store::init(&repo_path) {
        if host_lacks_root_user_namespace_mapping(&error) {
            eprintln!("skipping self files resolver test: host cannot map uid 0");
            return Ok(());
        }
        return Err(error);
    }

    let files_ref = "x86_64/pkg/libs/x11/libx11/1.8.10/abc/files";
    if !commit_self_runtime_files_ref(&temp_dir, &repo_path, files_ref)? {
        eprintln!("skipping self files resolver test: host cannot map uid 0");
        return Ok(());
    }

    let mut index = ManifestIndex::new();
    index.add_manifest(manifest_with_self_runtime_output());
    let store = Store::open(&repo_path)?;
    let mut resolver = RuntimeResolver::new(&index, store);
    resolver.seed_requests(&[super::MaterializeRequest::Output {
        commit: "x86_64/pkg/libs/x11/libx11/1.8.10/outputs/bin".to_string(),
    }]);
    resolver.run();

    let needed_files = resolver
        .closure
        .get_files(files_ref)
        .cloned()
        .unwrap_or_default();
    assert!(needed_files.contains("/usr/lib/libX11.so.6"));
    assert!(needed_files.contains("/usr/lib/libxcb.so.1"));
    assert!(resolver.closure.unresolved.is_empty());
    Ok(())
}
