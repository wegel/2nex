//! Bundle output refs into store branches and artifact refs.

use std::io;
use std::path::Path;

use crate::manifest::{Bundle, Manifest, OutputSpec};
use crate::store::{
    create_artifact, encode_metadata_list, get_branch_tree, rewrite_branch_metadata,
};

/// Return metadata attached to an output branch.
pub fn output_branch_metadata(
    manifest: &Manifest,
    _spec: &OutputSpec,
    manifest_hash: &str,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = vec![("nex.manifest.hash".to_string(), manifest_hash.to_string())];
    if let Some(checksum) = &manifest.package.checksum {
        metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
    }
    Ok(metadata)
}

/// Return metadata attached to a bundle branch.
pub fn bundle_branch_metadata(
    manifest: &Manifest,
    _bundle: &Bundle,
    manifest_hash: &str,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = vec![("nex.manifest.hash".to_string(), manifest_hash.to_string())];
    if let Some(checksum) = &manifest.package.checksum {
        metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
    }
    Ok(metadata)
}

/// Commit a bundle by merging its output branches and recording an artifact ref.
pub fn commit_bundle(
    repo_path: &str,
    bundle_name: &str,
    bundle: &Bundle,
    manifest: &Manifest,
    manifest_hash: &str,
) -> io::Result<()> {
    let output_commits = output_commit_refs(bundle, manifest);
    let bundle_branch = bundle_branch_ref(bundle_name, manifest);
    let metadata = bundle_metadata(manifest, bundle, manifest_hash, &output_commits)?;

    union_bundle_tree(repo_path, &output_commits, &bundle_branch)?;
    rewrite_branch_metadata(repo_path, &bundle_branch, &metadata)?;
    create_bundle_artifact(
        repo_path,
        bundle_name,
        manifest,
        manifest_hash,
        &bundle_branch,
    )
}

fn output_commit_refs(bundle: &Bundle, manifest: &Manifest) -> Vec<String> {
    bundle
        .includes
        .iter()
        .map(|output| {
            format!(
                "x86_64/{}/{}/{}/outputs/{}",
                manifest.package.namespace_path(),
                manifest.package.slug,
                manifest.package.version,
                output
            )
        })
        .collect()
}

fn bundle_branch_ref(bundle_name: &str, manifest: &Manifest) -> String {
    format!(
        "x86_64/{}/{}/{}/bundles/{}",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        bundle_name
    )
}

fn bundle_metadata(
    manifest: &Manifest,
    bundle: &Bundle,
    manifest_hash: &str,
    output_commits: &[String],
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = bundle_branch_metadata(manifest, bundle, manifest_hash)?;
    if let Some(encoded) = encode_metadata_list(output_commits)? {
        metadata.push(("nex.bundle.outputs".to_string(), encoded));
    }
    Ok(metadata)
}

fn union_bundle_tree(
    repo_path: &str,
    output_commits: &[String],
    bundle_branch: &str,
) -> io::Result<()> {
    let repo =
        zub::Repo::open(Path::new(repo_path)).map_err(|e| io::Error::other(e.to_string()))?;
    let refs: Vec<&str> = output_commits
        .iter()
        .map(|commit| commit.as_str())
        .collect();

    zub::ops::union_trees(
        &repo,
        &refs,
        bundle_branch,
        zub::ops::UnionOptions {
            on_conflict: zub::ops::ConflictResolution::Last,
            ..Default::default()
        },
    )
    .map(|_| ())
    .map_err(|e| io::Error::other(e.to_string()))
}

fn create_bundle_artifact(
    repo_path: &str,
    bundle_name: &str,
    manifest: &Manifest,
    manifest_hash: &str,
    bundle_branch: &str,
) -> io::Result<()> {
    let tree_hash = get_branch_tree(repo_path, bundle_branch)?;
    let artifact_output = format!("bundles/{}", bundle_name);
    let artifact_path = format!(
        "x86_64/{}/{}/{}/{}/bundles/{}",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        manifest_hash,
        bundle_name
    );
    create_artifact(
        repo_path,
        &tree_hash,
        manifest_hash,
        &artifact_output,
        &artifact_path,
    )
    .map(|_| ())
}
