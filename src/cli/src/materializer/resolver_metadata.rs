//! Runtime metadata checks for manifest-backed resolver inputs.

use crate::manifest::types::Manifest;

pub(super) fn missing_runtime_metadata(commit: &str, manifest: &Manifest) -> Vec<String> {
    if super::resolver::is_checksum_files_commit_ref(commit) {
        return Vec::new();
    }

    let commit_parts: Vec<&str> = commit.split('/').collect();
    let commit_type = commit_parts
        .get(commit_parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("");
    let commit_name = commit_parts.last().copied().unwrap_or("");

    if commit_type == "bundles" {
        return missing_bundle_outputs(commit_name, manifest);
    }
    if commit_type == "outputs" && !manifest.outputs.contains_key(commit_name) {
        return vec![format!("output '{}' in {}", commit_name, commit)];
    }
    Vec::new()
}

fn missing_bundle_outputs(bundle_name: &str, manifest: &Manifest) -> Vec<String> {
    let Some(bundle) = manifest.bundles.get(bundle_name) else {
        return vec![format!("bundle '{}'", bundle_name)];
    };

    bundle
        .includes
        .iter()
        .filter(|output_name| !manifest.outputs.contains_key(*output_name))
        .map(|output_name| format!("output '{}' in bundle '{}'", output_name, bundle_name))
        .collect()
}

#[cfg(test)]
#[path = "resolver_metadata_tests.rs"]
mod resolver_metadata_tests;
