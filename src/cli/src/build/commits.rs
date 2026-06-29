//! Store commit helpers for package build outputs.

use walkdir::WalkDir;

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::manifest::{BuildPaths, FileEntry, Manifest, OutputSpec, Package};
use crate::outputs::{commit_bundle, output_branch_metadata};
use crate::store::{
    checkout_into, commit_tree, create_artifact, ensure_branch_exists, rewrite_branch_metadata,
};

use super::status::{compute_manifest_hash, output_artifact_path, output_branch_name};
use super::OUTPUT_DISCARD;

/// Append a package checksum to the repository checksum ledger.
pub fn append_checksum_file(package: &Package, checksum: &str, file_path: &Path) -> io::Result<()> {
    let max_first_column_width = max_checksum_name_width(file_path)?;
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    if max_first_column_width > 0 {
        writeln!(file)?;
    }

    let padded_first_column = format!(
        "{:<width$}",
        package.to_string(),
        width = max_first_column_width + 3
    );
    writeln!(file, "{} {}", padded_first_column, checksum)?;

    Ok(())
}

/// Move manifest outputs into category directories and commit each output ref.
pub fn verify_and_commit_outputs(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    manifest_path: &Path,
    paths: &BuildPaths,
) -> io::Result<()> {
    println!("Verifying and committing outputs to store branches");

    let manifest_hash = compute_manifest_hash(manifest_path)?;
    let out_dir = Path::new(base_dir).join(&paths.out);
    let all_out_files = collect_output_files(&out_dir);
    let mut accounted_files = Vec::new();

    for (output_type, spec) in committable_outputs(manifest) {
        move_output_files(&out_dir, output_type, spec, &mut accounted_files)?;
        commit_output_branch(
            manifest,
            repo_path,
            &manifest_hash,
            output_type,
            spec,
            &out_dir,
        )?;
    }

    report_unaccounted_files(all_out_files, &accounted_files);
    Ok(())
}

/// Commit package bundles after their source outputs have been committed.
pub fn create_and_commit_bundles(
    manifest: &Manifest,
    _base_dir: &str,
    repo_path: &str,
    manifest_path: &Path,
) -> io::Result<()> {
    println!("Processing bundles");

    let manifest_hash = compute_manifest_hash(manifest_path)?;
    for (bundle_name, bundle) in &manifest.bundles {
        commit_bundle(repo_path, bundle_name, bundle, manifest, &manifest_hash)?;
    }

    Ok(())
}

/// Commit raw build output files to a checksum-addressed files ref.
pub fn commit_raw_files(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    paths: &BuildPaths,
    checksum: &str,
) -> io::Result<()> {
    let out_dir = Path::new(base_dir).join(&paths.out);
    if !out_dir.exists() {
        return Ok(());
    }

    let files_ref = checksum_files_ref(manifest, checksum);
    println!("Committing raw build output to {}", files_ref);

    commit_tree(
        repo_path,
        &files_ref,
        &out_dir,
        &raw_files_metadata(manifest, checksum),
    )?;

    Ok(())
}

/// Refresh output and bundle refs from the checksum-verified files ref.
pub fn refresh_package_metadata(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
) -> io::Result<()> {
    println!(
        "Refreshing outputs/bundles for {}/{} ({})",
        manifest.package.slug, manifest.package.version, manifest.package.namespace
    );

    let files_ref = semantic_files_ref(manifest);
    ensure_branch_exists(repo_path, &files_ref)?;

    let temp_dir = create_refresh_temp_dir(manifest)?;
    let base_dir = temp_dir.path().to_path_buf();
    let out_dir = base_dir.join("out");
    fs::create_dir_all(&out_dir)?;

    println!("Checking out {} to temp directory", files_ref);
    checkout_into(repo_path, &files_ref, &out_dir, false)?;
    commit_refreshed_metadata(repo_path, manifest, manifest_path, &base_dir)?;

    println!(
        "Finished refreshing outputs/bundles for {}",
        manifest.package.slug
    );
    Ok(())
}

/// Point the semantic files ref at the checksum-addressed files commit.
pub(super) fn create_semantic_files_ref(
    manifest: &Manifest,
    repo_path: &str,
    manifest_path: &Path,
    checksum: &str,
) -> io::Result<()> {
    let files_ref = checksum_files_ref(manifest, checksum);
    let semantic_files_ref = semantic_files_ref(manifest);

    println!("Creating semantic ref: {}", semantic_files_ref);

    let repo =
        zub::Repo::open(Path::new(repo_path)).map_err(|e| io::Error::other(e.to_string()))?;
    let commit_hash =
        zub::resolve_ref(&repo, &files_ref).map_err(|e| io::Error::other(e.to_string()))?;

    zub::write_ref(&repo, &semantic_files_ref, &commit_hash)
        .map_err(|e| io::Error::other(e.to_string()))?;

    let manifest_hash = compute_manifest_hash(manifest_path)?;
    rewrite_branch_metadata(
        repo_path,
        &semantic_files_ref,
        &[
            ("nex.manifest.hash".to_string(), manifest_hash),
            ("nex.address_hash".to_string(), checksum.to_string()),
        ],
    )?;

    Ok(())
}

fn max_checksum_name_width(file_path: &Path) -> io::Result<usize> {
    if !file_path.exists() {
        return Ok(0);
    }

    let file = File::open(file_path)?;
    let mut max_width = 0;
    for line in io::BufReader::new(file).lines() {
        let line = line?;
        if let Some(first_column) = line.split_whitespace().next() {
            max_width = max_width.max(first_column.len());
        }
    }
    Ok(max_width)
}

fn collect_output_files(out_dir: &Path) -> Vec<String> {
    WalkDir::new(out_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() || entry.file_type().is_symlink())
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(out_dir)
                .ok()?
                .to_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn committable_outputs(manifest: &Manifest) -> impl Iterator<Item = (&String, &OutputSpec)> {
    manifest.outputs.iter().filter(|(output_type, spec)| {
        output_type.as_str() != OUTPUT_DISCARD && !spec.files.is_empty()
    })
}

fn move_output_files(
    out_dir: &Path,
    output_type: &str,
    spec: &OutputSpec,
    accounted_files: &mut Vec<String>,
) -> io::Result<()> {
    for file_entry in &spec.files {
        move_manifest_file(out_dir, output_type, file_entry)?;
        accounted_files.push(file_entry.path.trim_start_matches('/').to_string());
    }
    Ok(())
}

fn move_manifest_file(out_dir: &Path, output_type: &str, file_entry: &FileEntry) -> io::Result<()> {
    let source_path = out_dir.join(file_entry.path.trim_start_matches('/'));
    ensure_manifest_output_exists(&source_path)?;

    let output_dir = categorized_parent_dir(out_dir, output_type, &source_path)?;
    let temp_path = temporarily_move_blocking_file(&output_dir)?;
    fs::create_dir_all(&output_dir)?;

    let target = output_dir.join(source_path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest output path has no file name",
        )
    })?);
    match temp_path {
        Some(tmp) => fs::rename(tmp, target),
        None => fs::rename(source_path, target),
    }
}

fn ensure_manifest_output_exists(source_path: &Path) -> io::Result<()> {
    if source_path.is_symlink() || source_path.exists() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "File {} listed in manifest outputs does not exist.",
            source_path.display()
        ),
    ))
}

fn categorized_parent_dir(
    out_dir: &Path,
    output_type: &str,
    source_path: &Path,
) -> io::Result<PathBuf> {
    let source_parent = source_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest output path has no parent",
        )
    })?;
    let relative_parent = source_parent
        .strip_prefix(out_dir)
        .map_err(io::Error::other)?;

    if relative_parent.as_os_str().is_empty() {
        Ok(out_dir.join(output_type))
    } else {
        Ok(out_dir.join(output_type).join(relative_parent))
    }
}

fn temporarily_move_blocking_file(output_dir: &Path) -> io::Result<Option<PathBuf>> {
    if !output_dir.exists() || output_dir.is_dir() {
        return Ok(None);
    }

    let tmp = output_dir.with_extension("tmp_rename");
    fs::rename(output_dir, &tmp)?;
    Ok(Some(tmp))
}

fn commit_output_branch(
    manifest: &Manifest,
    repo_path: &str,
    manifest_hash: &str,
    output_type: &str,
    spec: &OutputSpec,
    out_dir: &Path,
) -> io::Result<()> {
    let branch_name = output_branch_name(manifest, output_type);
    let commit_output_dir = out_dir.join(output_type);
    let metadata = output_branch_metadata(manifest, spec, manifest_hash)?;
    let tree_hash = commit_tree(repo_path, &branch_name, &commit_output_dir, &metadata)?;
    let artifact_output = format!("outputs/{}", output_type);
    let artifact_path = output_artifact_path(manifest, manifest_hash, output_type);

    create_artifact(
        repo_path,
        &tree_hash,
        manifest_hash,
        &artifact_output,
        &artifact_path,
    )?;
    Ok(())
}

fn report_unaccounted_files(all_out_files: Vec<String>, accounted_files: &[String]) {
    let unaccounted_files: Vec<String> = all_out_files
        .into_iter()
        .filter(|file| !accounted_files.contains(file))
        .collect();

    if !unaccounted_files.is_empty() {
        println!(
            "The following files in /nex/out are not accounted for in the manifest outputs: {:?}",
            unaccounted_files
        );
    }
}

fn raw_files_metadata(manifest: &Manifest, checksum: &str) -> Vec<(String, String)> {
    vec![
        ("nex.checksum".to_string(), checksum.to_string()),
        (
            "nex.package".to_string(),
            format!(
                "{}/{}/{}",
                manifest.package.namespace_path(),
                manifest.package.slug,
                manifest.package.version
            ),
        ),
    ]
}

fn create_refresh_temp_dir(manifest: &Manifest) -> io::Result<tempfile::TempDir> {
    let base_parent = Path::new(".nex/tmp");
    fs::create_dir_all(base_parent)?;
    let prefix = format!(
        "refresh_metadata_{}_{}_",
        manifest.package.slug.replace("/", "_"),
        manifest.package.namespace.replace("/", "_")
    );

    tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir_in(base_parent)
        .map_err(|e| io::Error::other(format!("tempdir failed: {}", e)))
}

fn commit_refreshed_metadata(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
    base_dir: &Path,
) -> io::Result<()> {
    let paths = BuildPaths {
        work: "work".to_string(),
        out: "out".to_string(),
        inputs: "inputs".to_string(),
    };
    let base_dir = base_dir
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "base dir must be UTF-8"))?;

    verify_and_commit_outputs(manifest, base_dir, repo_path, manifest_path, &paths)?;
    create_and_commit_bundles(manifest, base_dir, repo_path, manifest_path)
}

fn checksum_files_ref(manifest: &Manifest, checksum: &str) -> String {
    format!(
        "x86_64/{}/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        checksum
    )
}

fn semantic_files_ref(manifest: &Manifest) -> String {
    format!(
        "x86_64/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version
    )
}
