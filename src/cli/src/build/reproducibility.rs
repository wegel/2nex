//! Second-pass package build checks for reproducible output.

use std::fs;
use std::io;
use std::path::Path;

use crate::commands::build::BuildOpts;
use crate::manifest::{BuildEnvironment, Manifest};
use crate::outputs::calculate_output_checksum;

use super::commits::{create_and_commit_bundles, verify_and_commit_outputs};
use super::inputs::handle_inputs;
use super::rootfs::setup_composite_rootfs;
use super::script::run_build_script_with_env;

/// Inputs for the optional second package build.
pub struct ReproducibilityCheck<'a> {
    /// Original CLI build options.
    pub opts: &'a BuildOpts,
    /// Package manifest to rebuild.
    pub manifest: &'a Manifest,
    /// Build root directory.
    pub base_dir: &'a str,
    /// Source input cache directory.
    pub download_dir: &'a str,
    /// Loaded build environment.
    pub build_env: &'a BuildEnvironment,
    /// Dependency commits used to recreate the build root.
    pub dependency_commits: &'a [String],
    /// Prefix used when non-chroot source paths need canonical names.
    pub canonical_prefix: Option<&'a str>,
    /// Build script captured before any manifest rewrite.
    pub build_script: &'a str,
    /// First build output checksum.
    pub checksum: &'a str,
}

/// Run the second build pass when the CLI requested a reproducibility check.
pub fn maybe_check_reproducibility(check: ReproducibilityCheck<'_>) -> io::Result<()> {
    let ReproducibilityCheck {
        opts,
        manifest,
        base_dir,
        download_dir,
        build_env,
        dependency_commits,
        canonical_prefix,
        build_script,
        checksum,
    } = check;

    if !opts.check {
        return Ok(());
    }

    println!("Validating build reproducibility by building the package a second time.");
    fs::remove_dir_all(base_dir)?;
    prepare_second_build_root(opts, base_dir, build_env, dependency_commits)?;

    let input_env_vars = handle_inputs(
        &manifest.sources,
        download_dir,
        base_dir,
        !build_env.execution.chroot,
        &build_env.paths,
        canonical_prefix,
    )?;
    run_build_script_with_env(build_script, base_dir, &input_env_vars, build_env, None)?;
    verify_second_build(opts, manifest, base_dir, build_env, checksum)
}

fn prepare_second_build_root(
    opts: &BuildOpts,
    base_dir: &str,
    build_env: &BuildEnvironment,
    dependency_commits: &[String],
) -> io::Result<()> {
    setup_composite_rootfs(crate::build::RootfsSetup {
        base_dir,
        repo_path: &opts.repo_path,
        fallback_repos: &opts.fallback_repos,
        dependency_commits,
        paths: &build_env.paths,
        verbose: opts.verbose,
        reuse_rootfs: false,
        hydrate_runtime_deps: build_env.execution.chroot,
    })
}

fn verify_second_build(
    opts: &BuildOpts,
    manifest: &Manifest,
    base_dir: &str,
    build_env: &BuildEnvironment,
    checksum: &str,
) -> io::Result<()> {
    let output_dir = Path::new(base_dir).join(&build_env.paths.out);
    let second_checksum = calculate_output_checksum(&output_dir)?;
    println!("Second build output checksum: {}", second_checksum);
    verify_reproducible_checksum(checksum, &second_checksum)?;

    let manifest_path = Path::new(&opts.manifest_file);
    verify_and_commit_outputs(
        manifest,
        base_dir,
        &opts.repo_path,
        manifest_path,
        &build_env.paths,
    )?;
    create_and_commit_bundles(manifest, base_dir, &opts.repo_path, manifest_path)?;
    Ok(())
}

fn verify_reproducible_checksum(first_checksum: &str, second_checksum: &str) -> io::Result<()> {
    if first_checksum == second_checksum {
        println!("Build is reproducible. Checksums match.");
        Ok(())
    } else {
        println!("Build is not reproducible. Checksums do not match.");
        Err(io::Error::other("Build is not reproducible."))
    }
}
