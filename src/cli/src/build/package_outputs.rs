//! Checksum and output commit steps for package builds.

use std::io;
use std::path::Path;

use crate::commands::build::BuildOpts;
use crate::manifest::{update_manifest_checksum_field, BuildEnvironment, Manifest, ManifestKind};
use crate::outputs::calculate_output_checksum;

use super::commits::{
    commit_raw_files, create_and_commit_bundles, create_semantic_files_ref,
    refresh_package_metadata, verify_and_commit_outputs,
};

#[cfg(test)]
#[path = "package_outputs_tests.rs"]
mod package_outputs_tests;

/// Calculate the package output checksum and commit the raw files snapshot.
pub fn commit_raw_output(
    opts: &BuildOpts,
    manifest: &Manifest,
    base_dir: &str,
    build_env: &BuildEnvironment,
) -> io::Result<String> {
    let output_dir = Path::new(base_dir).join(&build_env.paths.out);
    let checksum = calculate_output_checksum(&output_dir)?;
    println!("Build output checksum: {}", checksum);

    commit_raw_files(
        manifest,
        base_dir,
        &opts.repo_path,
        &build_env.paths,
        &checksum,
    )?;
    Ok(checksum)
}

/// Decide whether this package should publish outputs after checksum validation.
pub fn should_commit_package_outputs(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    checksum: &str,
) -> io::Result<bool> {
    match manifest.package.checksum.clone() {
        Some(expected_checksum) => {
            handle_expected_checksum(opts, manifest, checksum, &expected_checksum)
        }
        None => handle_missing_checksum(opts, manifest, checksum),
    }
}

/// Commit package outputs, bundles, and semantic files ref.
pub fn commit_package_outputs(
    opts: &BuildOpts,
    manifest: &Manifest,
    base_dir: &str,
    build_env: &BuildEnvironment,
    checksum: &str,
) -> io::Result<()> {
    let manifest_path = Path::new(&opts.manifest_file);
    verify_and_commit_outputs(
        manifest,
        base_dir,
        &opts.repo_path,
        manifest_path,
        &build_env.paths,
    )?;
    create_and_commit_bundles(manifest, base_dir, &opts.repo_path, manifest_path)?;
    println!("Build, packaging, and commit completed for all outputs.");

    create_semantic_files_ref(manifest, &opts.repo_path, manifest_path, checksum)?;
    if opts.update_checksum {
        refresh_package_metadata(&opts.repo_path, manifest, manifest_path)?;
    }
    Ok(())
}

/// Compute runtime dependency metadata and refresh store refs when requested.
pub fn maybe_compute_runtime_deps(opts: &BuildOpts, manifest: &mut Manifest) -> io::Result<()> {
    if !opts.compute_deps {
        return Ok(());
    }

    let manifest_path = Path::new(&opts.manifest_file);
    crate::commands::compute_deps::compute_deps_for_manifest(
        manifest,
        &opts.repo_path,
        manifest_path,
        &opts.manifest_dirs,
        opts.writable_manifest_root.as_deref(),
        opts.runtime_deps_verbose,
        false,
    )?;
    refresh_package_metadata(&opts.repo_path, manifest, manifest_path)
}

fn handle_expected_checksum(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    checksum: &str,
    expected_checksum: &str,
) -> io::Result<bool> {
    if checksum == expected_checksum {
        println!("Checksum verified successfully.");
        return Ok(true);
    }
    if opts.check {
        println!(
            "Note: checksum differs from manifest (expected {}, got {}). Proceeding with reproducibility check.",
            expected_checksum, checksum
        );
        return Ok(true);
    }
    if opts.update_checksum {
        println!(
            "Checksum mismatch (expected {}, calculated {}). Updating manifest.",
            expected_checksum, checksum
        );
        update_package_checksum(opts, manifest, checksum)?;
        return Ok(true);
    }

    eprintln!(
        "Checksum mismatch. Expected: {}, Calculated: {}",
        expected_checksum, checksum
    );
    eprintln!("Raw files committed to {}/files for debugging.", checksum);
    eprintln!(
        "Compare with: zub diff {}/files {}/files",
        expected_checksum, checksum
    );
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Checksum mismatch. Expected: {}, Calculated: {}",
            expected_checksum, checksum
        ),
    ))
}

fn handle_missing_checksum(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    checksum: &str,
) -> io::Result<bool> {
    if opts.check {
        return Ok(true);
    }
    if opts.update_checksum {
        println!(
            "Manifest {} does not record a checksum. Storing {}.",
            opts.manifest_file, checksum
        );
        update_package_checksum(opts, manifest, checksum)?;
    }
    Ok(true)
}

pub(super) fn update_check_checksum_after_reproducibility(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    checksum: &str,
) -> io::Result<()> {
    if !opts.check || !opts.update_checksum {
        return Ok(());
    }
    if manifest.package.checksum.as_deref() == Some(checksum) {
        return Ok(());
    }
    update_package_checksum(opts, manifest, checksum)
}

pub(super) fn ensure_check_checksum_allows_package_publish(
    opts: &BuildOpts,
    manifest: &Manifest,
    checksum: &str,
) -> io::Result<()> {
    if !opts.check || opts.update_checksum {
        return Ok(());
    }

    match manifest.package.checksum.as_deref() {
        Some(expected) if expected == checksum => Ok(()),
        Some(expected) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "package checksum differs from manifest after reproducibility check: expected {}, got {}. Re-run with --update-checksum before publishing refs.",
                expected, checksum
            ),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "package manifest does not record a checksum. Re-run with --update-checksum before publishing refs.",
        )),
    }
}

fn update_package_checksum(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    checksum: &str,
) -> io::Result<()> {
    update_manifest_checksum_field(&opts.manifest_file, ManifestKind::Package, checksum)?;
    manifest.package.checksum = Some(checksum.to_string());
    Ok(())
}
