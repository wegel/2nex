use std::collections::{BTreeSet, HashMap};

use crate::manifest::types::{Build, FileEntry, Manifest, OutputSpec, Package};

use super::{file_entries_to_process, queue_self_file_dependency, RuntimeClosure};

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
        entries,
        vec![(
            "/usr/lib/libX11.so.6".to_string(),
            vec!["/usr/lib/libxcb.so.1".to_string()]
        )]
    );

    let entries = file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
    assert!(entries.is_empty());

    closure.add_file_dep(
        commit,
        "/usr/lib/libX11-xcb.so.1",
        "another need".to_string(),
    );
    let entries = file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
    assert_eq!(
        entries,
        vec![("/usr/lib/libX11-xcb.so.1".to_string(), Vec::new())]
    );
}

#[test]
fn checksum_files_commit_queues_self_file_for_later_processing() {
    let commit = "x86_64/pkg/libs/graphics/mesa/24.2.7/abc/files";
    let mut closure = RuntimeClosure::default();
    let mut pending = Vec::new();

    queue_self_file_dependency(
        commit,
        "/usr/lib/libgallium-24.2.7.so",
        "test".to_string(),
        &mut closure,
        &mut pending,
    );

    assert_eq!(pending, vec![commit.to_string()]);
    assert_eq!(
        closure.get_files(commit).cloned().unwrap_or_default(),
        BTreeSet::from(["/usr/lib/libgallium-24.2.7.so".to_string()])
    );

    queue_self_file_dependency(
        commit,
        "/usr/lib/libgallium-24.2.7.so",
        "duplicate".to_string(),
        &mut closure,
        &mut pending,
    );

    assert_eq!(pending, vec![commit.to_string()]);
}
