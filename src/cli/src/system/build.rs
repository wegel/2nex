//! Top-level system assembly build coordinator.

use std::fs;
use std::io;
use std::path::Path;
use std::process;

use crate::build::{
    layer_commits_into_rootfs, load_environment, run_build_script_with_env, setup_composite_rootfs,
    RootfsSetup,
};
use crate::deps::resolve_dependency_closure;
use crate::manifest::{
    update_manifest_checksum_field, ManifestIndex, ManifestKind, SystemManifest,
};
use crate::outputs::calculate_output_checksum;
use crate::BuildOpts;

use super::commit::commit_system_rootfs;
use super::dependencies::dependencies_from_system_packages;
use super::env::build_system_env_vars;
use super::flat::materialize_system_packages;
use super::nex::materialize_nex_structure;
use super::overlays::apply_overlays;

struct SystemBuildInputs {
    manifest_index: ManifestIndex,
    dependency_commits: Vec<String>,
    package_commits: Vec<String>,
    original_package_commits: Vec<String>,
    build_env: crate::manifest::BuildEnvironment,
    use_absolute_paths: bool,
}

/// Build a system manifest using the default system build root.
pub fn build_system_manifest(opts: &BuildOpts, manifest: &SystemManifest) -> io::Result<()> {
    build_system_manifest_with_dir(opts, manifest, ".nex/tmp/build_rootfs")
}

/// Build a system manifest using an explicit build root directory.
pub fn build_system_manifest_with_dir(
    opts: &BuildOpts,
    manifest: &SystemManifest,
    base_dir: &str,
) -> io::Result<()> {
    print_unsupported_compute_deps_note(opts);
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    let inputs = prepare_system_build_inputs(opts, manifest)?;
    build_system_once(
        opts,
        manifest,
        base_dir,
        download_dir,
        &inputs,
        opts.reuse_rootfs,
    )?;
    let checksum = checksum_target(base_dir)?;
    println!("System build checksum: {}", checksum);

    handle_system_checksum(opts, manifest, &checksum)?;
    commit_system_build(opts, manifest, base_dir, &inputs, &checksum)?;
    println!(
        "System commit stored at systems/{}/{}",
        manifest.system.slug, manifest.system.version
    );

    if opts.check {
        check_system_reproducibility(opts, manifest, base_dir, download_dir, &inputs, &checksum)?;
    }

    Ok(())
}

fn prepare_system_build_inputs(
    opts: &BuildOpts,
    manifest: &SystemManifest,
) -> io::Result<SystemBuildInputs> {
    let manifest_index = ManifestIndex::load("pkg")?;
    let dependency_commits = resolve_dependency_closure(&manifest.dependencies, &manifest_index)?;
    let package_dependency_specs = dependencies_from_system_packages(&manifest.packages);
    let package_commits = resolve_dependency_closure(&package_dependency_specs, &manifest_index)?;
    let original_package_commits = manifest
        .packages
        .iter()
        .map(|package| package.commit.clone())
        .collect();
    let build_env = load_environment(&opts.repo_path, &manifest.build.environment)?;
    let use_absolute_paths = !build_env.execution.chroot;

    Ok(SystemBuildInputs {
        manifest_index,
        dependency_commits,
        package_commits,
        original_package_commits,
        build_env,
        use_absolute_paths,
    })
}

fn build_system_once(
    opts: &BuildOpts,
    manifest: &SystemManifest,
    base_dir: &str,
    download_dir: &str,
    inputs: &SystemBuildInputs,
    reuse_rootfs: bool,
) -> io::Result<()> {
    prepare_system_rootfs(opts, base_dir, inputs, reuse_rootfs)?;
    materialize_system_package_set(opts, manifest, base_dir, inputs)?;
    apply_overlays(&manifest.overlays, base_dir)?;
    run_system_script(manifest, base_dir, download_dir, inputs)
}

fn prepare_system_rootfs(
    opts: &BuildOpts,
    base_dir: &str,
    inputs: &SystemBuildInputs,
    reuse_rootfs: bool,
) -> io::Result<()> {
    setup_composite_rootfs(RootfsSetup {
        base_dir,
        repo_path: &opts.repo_path,
        fallback_repos: &opts.fallback_repos,
        dependency_commits: &inputs.dependency_commits,
        paths: &inputs.build_env.paths,
        verbose: opts.verbose,
        reuse_rootfs,
        hydrate_runtime_deps: false,
    })?;
    layer_commits_into_rootfs(
        base_dir,
        &opts.repo_path,
        &opts.fallback_repos,
        &inputs.package_commits,
        opts.verbose,
    )
}

fn materialize_system_package_set(
    opts: &BuildOpts,
    manifest: &SystemManifest,
    base_dir: &str,
    inputs: &SystemBuildInputs,
) -> io::Result<()> {
    if manifest.system.nex_structure {
        materialize_nex_structure(base_dir, &opts.repo_path, &inputs.original_package_commits)
    } else {
        materialize_system_packages(
            base_dir,
            &opts.repo_path,
            &inputs.original_package_commits,
            &inputs.manifest_index,
        )
    }
}

fn run_system_script(
    manifest: &SystemManifest,
    base_dir: &str,
    download_dir: &str,
    inputs: &SystemBuildInputs,
) -> io::Result<()> {
    let env_vars = build_system_env_vars(
        manifest,
        download_dir,
        base_dir,
        inputs.use_absolute_paths,
        &inputs.build_env.paths,
    )?;

    println!(
        "Building system {} {}",
        manifest.system.slug, manifest.system.version
    );
    run_build_script_with_env(
        &manifest.build.script,
        base_dir,
        &env_vars,
        &inputs.build_env,
        None,
    )?;
    Ok(())
}

fn checksum_target(base_dir: &str) -> io::Result<String> {
    calculate_output_checksum(&Path::new(base_dir).join("target"))
}

fn handle_system_checksum(
    opts: &BuildOpts,
    manifest: &SystemManifest,
    checksum: &str,
) -> io::Result<()> {
    match manifest.system.checksum.as_ref() {
        Some(expected_checksum) => handle_recorded_checksum(opts, expected_checksum, checksum),
        None => handle_missing_checksum(opts, checksum),
    }
}

fn handle_recorded_checksum(
    opts: &BuildOpts,
    expected_checksum: &str,
    checksum: &str,
) -> io::Result<()> {
    if checksum == expected_checksum {
        println!("Checksum verified successfully.");
        return Ok(());
    }
    if opts.update_checksum {
        println!(
            "System checksum mismatch (expected {}, calculated {}). Updating manifest.",
            expected_checksum, checksum
        );
        return update_manifest_checksum_field(&opts.manifest_file, ManifestKind::System, checksum);
    }
    eprintln!(
        "Checksum mismatch. Expected: {}, Calculated: {}",
        expected_checksum, checksum
    );
    process::exit(-2);
}

fn handle_missing_checksum(opts: &BuildOpts, checksum: &str) -> io::Result<()> {
    if opts.update_checksum {
        println!(
            "System manifest {} does not record a checksum. Storing {}.",
            opts.manifest_file, checksum
        );
        update_manifest_checksum_field(&opts.manifest_file, ManifestKind::System, checksum)?;
    }
    Ok(())
}

fn commit_system_build(
    opts: &BuildOpts,
    manifest: &SystemManifest,
    base_dir: &str,
    inputs: &SystemBuildInputs,
    checksum: &str,
) -> io::Result<()> {
    commit_system_rootfs(
        manifest,
        base_dir,
        &opts.repo_path,
        &inputs.package_commits,
        &inputs.dependency_commits,
        checksum,
    )
}

fn check_system_reproducibility(
    opts: &BuildOpts,
    manifest: &SystemManifest,
    base_dir: &str,
    download_dir: &str,
    inputs: &SystemBuildInputs,
    checksum: &str,
) -> io::Result<()> {
    println!("Validating build reproducibility by building the system a second time.");
    fs::remove_dir_all(base_dir)?;
    build_system_once(opts, manifest, base_dir, download_dir, inputs, false)?;
    let second_checksum = checksum_target(base_dir)?;
    println!("Second build checksum: {}", second_checksum);
    verify_reproducible_checksum(checksum, &second_checksum)?;
    commit_system_build(opts, manifest, base_dir, inputs, &second_checksum)
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

fn print_unsupported_compute_deps_note(opts: &BuildOpts) {
    if opts.runtime_deps_verbose || opts.compute_deps {
        println!("Note: --compute-deps is not available for system manifests (deps come from package manifests).");
    }
}
