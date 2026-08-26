//! Output partitioning and publication as Zub refs.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use zub::ops::{ConflictResolution, UnionOptions};
use zub::{Hash, Repo};

use crate::manifest::PackageManifest;
use crate::metadata;
use crate::reference::{PackageRef, RefKind};
use crate::schema::invalid;

static BUILD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn publish(
    repo: &Repo,
    manifest: &PackageManifest,
    output_root: &Path,
    checksum: &str,
    recipe: &str,
) -> io::Result<()> {
    validate_outputs(manifest, output_root)?;

    let sequence = BUILD_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_prefix = format!("nex/tmp/simple-builder/{}/{sequence}", std::process::id());
    let mut pending = BTreeMap::new();
    let result = (|| {
        let files = format!("{temporary_prefix}/files");
        let metadata = metadata::build(checksum, recipe);
        let hash = zub::ops::commit_with_metadata(
            repo,
            output_root,
            &files,
            Some(""),
            Some(metadata::AUTHOR),
            &metadata,
        )
        .map_err(io::Error::other)?;
        pending.insert(files_ref(manifest), hash);
        commit_outputs(
            repo,
            manifest,
            checksum,
            recipe,
            &temporary_prefix,
            &mut pending,
        )?;
        commit_bundles(
            repo,
            manifest,
            checksum,
            recipe,
            &temporary_prefix,
            &mut pending,
        )?;
        for (reference, hash) in &pending {
            zub::write_ref(repo, reference, hash).map_err(io::Error::other)?;
        }
        Ok(())
    })();
    cleanup_temporary_refs(repo, manifest, &temporary_prefix);
    result
}

fn validate_outputs(manifest: &PackageManifest, output_root: &Path) -> io::Result<()> {
    let mut staged = BTreeSet::new();
    for specification in manifest.outputs.values() {
        for entry in &specification.files {
            let relative = Path::new(&entry.path)
                .strip_prefix("/")
                .expect("validated absolute output path");
            if !staged.insert(relative.to_path_buf()) {
                return Err(invalid(format!(
                    "output path appears more than once: {}",
                    entry.path
                )));
            }
            let source = output_root.join(relative);
            source.symlink_metadata().map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "declared output does not exist: {}: {error}",
                        source.display()
                    ),
                )
            })?;
        }
    }
    let mut previous: Option<&PathBuf> = None;
    for path in &staged {
        if let Some(parent) = previous {
            if path.starts_with(parent) {
                return Err(invalid(format!(
                    "output paths overlap: {} and {}",
                    parent.display(),
                    path.display()
                )));
            }
        }
        previous = Some(path);
    }

    Ok(())
}

fn commit_outputs(
    repo: &Repo,
    manifest: &PackageManifest,
    checksum: &str,
    recipe: &str,
    temporary_prefix: &str,
    pending: &mut BTreeMap<String, Hash>,
) -> io::Result<()> {
    let metadata = metadata::build(checksum, recipe);
    for (output, specification) in manifest
        .outputs
        .iter()
        .filter(|(name, _)| name.as_str() != "discard")
    {
        let final_ref = output_ref(manifest, output);
        let temporary_ref = format!("{temporary_prefix}/outputs/{output}");
        let paths = specification
            .files
            .iter()
            .map(|entry| PathBuf::from(&entry.path))
            .collect::<Vec<_>>();
        let tree = zub::ops::select_tree(repo, &format!("{temporary_prefix}/files"), &paths)
            .map_err(io::Error::other)?;
        let hash = zub::ops::commit_tree_with_metadata(
            repo,
            &tree,
            &temporary_ref,
            Some(""),
            Some(metadata::AUTHOR),
            &metadata,
        )
        .map_err(io::Error::other)?;
        pending.insert(final_ref, hash);
    }
    Ok(())
}

fn commit_bundles(
    repo: &Repo,
    manifest: &PackageManifest,
    checksum: &str,
    recipe: &str,
    temporary_prefix: &str,
    pending: &mut BTreeMap<String, Hash>,
) -> io::Result<()> {
    let metadata = metadata::build(checksum, recipe);
    for (name, bundle) in &manifest.bundles {
        let inputs = bundle
            .iter()
            .map(|output| format!("{temporary_prefix}/outputs/{output}"))
            .collect::<Vec<_>>();
        let input_refs = inputs.iter().map(String::as_str).collect::<Vec<_>>();
        let temporary_ref = format!("{temporary_prefix}/bundles/{name}");
        let union = zub::ops::union_trees(
            repo,
            &input_refs,
            &temporary_ref,
            UnionOptions {
                author: Some(metadata::AUTHOR.to_owned()),
                on_conflict: ConflictResolution::Error,
                ..UnionOptions::default()
            },
        )
        .map_err(io::Error::other)?;
        let tree = zub::read_commit(repo, &union)
            .map_err(io::Error::other)?
            .tree;
        let hash = zub::ops::commit_tree_with_metadata(
            repo,
            &tree,
            &temporary_ref,
            Some(""),
            Some(metadata::AUTHOR),
            &metadata,
        )
        .map_err(io::Error::other)?;
        pending.insert(bundle_ref(manifest, name), hash);
    }
    Ok(())
}

fn cleanup_temporary_refs(repo: &Repo, manifest: &PackageManifest, prefix: &str) {
    let _ = zub::delete_ref(repo, &format!("{prefix}/files"));
    for output in manifest
        .outputs
        .keys()
        .filter(|name| name.as_str() != "discard")
    {
        let _ = zub::delete_ref(repo, &format!("{prefix}/outputs/{output}"));
    }
    for bundle in manifest.bundles.keys() {
        let _ = zub::delete_ref(repo, &format!("{prefix}/bundles/{bundle}"));
    }
}

pub(crate) fn published_refs(manifest: &PackageManifest) -> Vec<String> {
    let mut refs = std::iter::once(files_ref(manifest))
        .chain(
            manifest
                .bundles
                .keys()
                .map(|name| bundle_ref(manifest, name)),
        )
        .chain(
            manifest
                .outputs
                .keys()
                .filter(|name| name.as_str() != "discard")
                .map(|name| output_ref(manifest, name)),
        )
        .collect::<Vec<_>>();
    refs.sort();
    refs
}

pub(crate) fn files_ref(manifest: &PackageManifest) -> String {
    PackageRef::from_manifest(manifest, RefKind::Files).to_string()
}

pub(crate) fn output_ref(manifest: &PackageManifest, output: &str) -> String {
    PackageRef::from_manifest(manifest, RefKind::Output(output.to_owned())).to_string()
}

pub(crate) fn bundle_ref(manifest: &PackageManifest, bundle: &str) -> String {
    PackageRef::from_manifest(manifest, RefKind::Bundle(bundle.to_owned())).to_string()
}
