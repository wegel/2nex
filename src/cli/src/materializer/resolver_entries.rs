//! Select manifest file entries for runtime dependency processing.

use std::collections::{BTreeSet, HashMap};

use crate::manifest::types::Manifest;

use super::types::RuntimeClosure;

#[derive(Default)]
pub(super) struct FileEntriesToProcess {
    pub(super) entries: Vec<(String, Vec<String>)>,
    pub(super) missing_files: Vec<String>,
}

pub(super) fn file_entries_to_process(
    commit: &str,
    manifest: &Manifest,
    closure: &RuntimeClosure,
    processed_file_paths: &mut HashMap<String, BTreeSet<String>>,
) -> FileEntriesToProcess {
    if is_checksum_files_commit_ref(commit) {
        return checksum_file_entries_to_process(commit, manifest, closure, processed_file_paths);
    }

    let output_names = output_names_for_commit(commit, manifest);
    let entries = output_names
        .iter()
        .filter_map(|output_name| manifest.outputs.get(output_name))
        .flat_map(|output| output.files.iter())
        .map(|file_entry| (file_entry.path.clone(), file_entry.needs.clone()))
        .collect();
    FileEntriesToProcess {
        entries,
        missing_files: Vec::new(),
    }
}

pub(super) fn is_checksum_files_commit_ref(commit: &str) -> bool {
    let mut parts = commit.rsplit('/');
    let Some(last) = parts.next() else {
        return false;
    };
    let Some(previous) = parts.next() else {
        return false;
    };
    last == "files"
        && previous != "outputs"
        && previous != "bundles"
        && commit.split('/').count() >= 7
}

fn checksum_file_entries_to_process(
    commit: &str,
    manifest: &Manifest,
    closure: &RuntimeClosure,
    processed_file_paths: &mut HashMap<String, BTreeSet<String>>,
) -> FileEntriesToProcess {
    let Some(requested_files) = closure.get_files(commit) else {
        return FileEntriesToProcess::default();
    };
    let processed_files = processed_file_paths.entry(commit.to_string()).or_default();
    let pending_files: BTreeSet<String> = requested_files
        .difference(processed_files)
        .cloned()
        .collect();
    if pending_files.is_empty() {
        return FileEntriesToProcess::default();
    }

    processed_files.extend(pending_files.iter().cloned());
    entries_for_pending_files(manifest, &pending_files)
}

fn entries_for_pending_files(
    manifest: &Manifest,
    pending_files: &BTreeSet<String>,
) -> FileEntriesToProcess {
    let entries = manifest
        .outputs
        .values()
        .flat_map(|output| output.files.iter())
        .filter(|file_entry| pending_files.contains(&file_entry.path))
        .map(|file_entry| (file_entry.path.clone(), file_entry.needs.clone()))
        .collect::<Vec<_>>();
    let found_files = entries
        .iter()
        .map(|(file_path, _)| file_path.clone())
        .collect::<BTreeSet<_>>();
    let missing_files = pending_files.difference(&found_files).cloned().collect();
    FileEntriesToProcess {
        entries,
        missing_files,
    }
}

fn output_names_for_commit(commit: &str, manifest: &Manifest) -> Vec<String> {
    let commit_parts: Vec<&str> = commit.split('/').collect();
    let commit_type = commit_parts
        .get(commit_parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("");
    let commit_name = commit_parts.last().copied().unwrap_or("");

    if commit_type == "bundles" {
        return manifest
            .bundles
            .get(commit_name)
            .map(|bundle| bundle.includes.clone())
            .unwrap_or_default();
    }
    vec![commit_name.to_string()]
}
