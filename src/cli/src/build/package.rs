//! Package and system build entry points for the CLI.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use crate::commands::build::BuildOpts;
use crate::manifest::{
    detect_manifest_kind, load_manifest, load_system_manifest_resolved, BuildEnvironment, Manifest,
    ManifestData, ManifestKind,
};
use crate::outputs::categorize_files_with_existing_outputs;
use crate::progress::BuildProgressConfig;

use super::commits::{append_checksum_file, refresh_package_metadata};
use super::dependencies::resolve_dependency_commits;
use super::env::load_environment;
use super::inputs::handle_inputs;
use super::package_outputs::{
    commit_package_outputs, commit_raw_output, maybe_compute_runtime_deps,
    should_commit_package_outputs,
};
use super::reproducibility::{maybe_check_reproducibility, ReproducibilityCheck};
use super::rootfs::setup_composite_rootfs;
use super::script::run_build_script_with_env;
use super::status::check_if_built;

/// Build the package or system named by the CLI options.
pub fn build_single(opts: &BuildOpts) -> io::Result<()> {
    fs::create_dir_all(".nex/tmp")?;

    let manifest_data = load_manifest_data(opts)?;
    validate_refresh_metadata_flags(opts, &manifest_data)?;
    if skip_current_package(opts, &manifest_data)? {
        return Ok(());
    }

    match manifest_data {
        ManifestData::Package(mut manifest) => build_package_from_opts(opts, &mut manifest),
        ManifestData::System(manifest) => {
            println!("Building system: {}", manifest.system.slug);
            let build_dir = opts
                .build_dir
                .clone()
                .unwrap_or_else(|| default_system_build_dir(&manifest.system.slug));
            crate::system::build_system_manifest_with_dir(opts, &manifest, &build_dir)
        }
    }
}

/// Build a package manifest with a specific build directory.
pub fn build_package_manifest_with_dir(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    base_dir: &str,
) -> io::Result<()> {
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    let build_env = load_environment(&opts.repo_path, &manifest.build.environment)?;
    let dependency_commits = resolve_dependency_commits(&manifest.dependencies, &opts.repo_path)?;
    prepare_package_root(
        opts,
        base_dir,
        &build_env,
        &dependency_commits,
        opts.reuse_rootfs,
    )?;

    let canonical_prefix = canonical_prefix(&build_env);
    let input_env_vars = handle_inputs(
        &manifest.sources,
        download_dir,
        base_dir,
        !build_env.execution.chroot,
        &build_env.paths,
        canonical_prefix,
    )?;

    let build_script = manifest.build.script.clone();
    run_primary_build(
        opts,
        manifest,
        base_dir,
        &build_env,
        &build_script,
        &input_env_vars,
    )?;
    maybe_generate_outputs(opts, manifest, base_dir, &build_env)?;

    let checksum = commit_raw_output(opts, manifest, base_dir, &build_env)?;
    if should_commit_package_outputs(opts, manifest, &checksum)? {
        commit_package_outputs(opts, manifest, base_dir, &build_env, &checksum)?;
    }

    maybe_compute_runtime_deps(opts, manifest)?;
    maybe_check_reproducibility(ReproducibilityCheck {
        opts,
        manifest,
        base_dir,
        download_dir,
        build_env: &build_env,
        dependency_commits: &dependency_commits,
        canonical_prefix,
        build_script: &build_script,
        checksum: &checksum,
    })?;

    append_checksum_file(&manifest.package, &checksum, Path::new("checksums.txt"))?;
    Ok(())
}

fn load_manifest_data(opts: &BuildOpts) -> io::Result<ManifestData> {
    let manifest_str = fs::read_to_string(&opts.manifest_file)?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&manifest_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if detect_manifest_kind(&doc) == ManifestKind::System {
        let resolved = load_system_manifest_resolved(Path::new(&opts.manifest_file))?;
        Ok(ManifestData::System(resolved))
    } else {
        load_manifest(&opts.manifest_file)
    }
}

fn validate_refresh_metadata_flags(
    opts: &BuildOpts,
    manifest_data: &ManifestData,
) -> io::Result<()> {
    if !opts.refresh_metadata {
        return Ok(());
    }
    if matches!(manifest_data, ManifestData::System(_)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--refresh-metadata only applies to package manifests",
        ));
    }
    if opts.check {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--refresh-metadata cannot be combined with --validate-reproducibility",
        ));
    }
    Ok(())
}

fn skip_current_package(opts: &BuildOpts, manifest_data: &ManifestData) -> io::Result<bool> {
    let ManifestData::Package(manifest) = manifest_data else {
        return Ok(false);
    };
    if opts.force || opts.check || opts.refresh_metadata {
        return Ok(false);
    }

    let manifest_path = Path::new(&opts.manifest_file);
    if let Ok(Some(_commit)) = check_if_built(&opts.repo_path, manifest, manifest_path) {
        println!(
            "Package {}/{} already built, skipping (use --force to rebuild)",
            manifest.package.namespace, manifest.package.slug
        );
        return Ok(true);
    }
    Ok(false)
}

fn build_package_from_opts(opts: &BuildOpts, manifest: &mut Manifest) -> io::Result<()> {
    if opts.refresh_metadata {
        return refresh_package_metadata(&opts.repo_path, manifest, Path::new(&opts.manifest_file));
    }

    println!(
        "Building package: {}/{}",
        manifest.package.namespace, manifest.package.slug
    );
    let build_dir = opts
        .build_dir
        .clone()
        .unwrap_or_else(|| default_package_build_dir(manifest));
    build_package_manifest_with_dir(opts, manifest, &build_dir)
}

fn default_package_build_dir(manifest: &Manifest) -> String {
    format!(
        ".nex/tmp/build_rootfs_{}_{}",
        manifest.package.slug.replace("/", "_"),
        manifest.package.namespace.replace("/", "_")
    )
}

fn default_system_build_dir(slug: &str) -> String {
    format!(".nex/tmp/build_rootfs_{}_system", slug.replace("/", "_"))
}

fn prepare_package_root(
    opts: &BuildOpts,
    base_dir: &str,
    build_env: &BuildEnvironment,
    dependency_commits: &[String],
    reuse_rootfs: bool,
) -> io::Result<()> {
    setup_composite_rootfs(crate::build::RootfsSetup {
        base_dir,
        repo_path: &opts.repo_path,
        fallback_repos: &opts.fallback_repos,
        dependency_commits,
        paths: &build_env.paths,
        verbose: opts.verbose,
        reuse_rootfs,
        hydrate_runtime_deps: build_env.execution.chroot,
    })
}

fn canonical_prefix(build_env: &BuildEnvironment) -> Option<&'static str> {
    if build_env.execution.chroot {
        None
    } else {
        Some("/tmp/bootstrap")
    }
}

fn run_primary_build(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    base_dir: &str,
    build_env: &BuildEnvironment,
    build_script: &str,
    input_env_vars: &HashMap<String, String>,
) -> io::Result<()> {
    println!(
        "Building {} {} in namespace {} using '{}' environment",
        manifest.package.name, manifest.package.version, manifest.package.namespace, build_env.name
    );

    let build_result = if opts.no_progress {
        run_build_script_with_env(build_script, base_dir, input_env_vars, build_env, None)?
    } else {
        run_build_script_with_env(
            build_script,
            base_dir,
            input_env_vars,
            build_env,
            Some(&progress_config(opts, manifest)),
        )?
    };

    if opts.record_profile {
        if let Some(new_profile) = build_result.new_profile {
            crate::manifest::update::update_build_profile(&opts.manifest_file, &new_profile)?;
            manifest.build.profile = new_profile;
        }
    }
    Ok(())
}

fn progress_config(opts: &BuildOpts, manifest: &Manifest) -> BuildProgressConfig {
    let mut config = BuildProgressConfig::new(&format!(
        "{}/{}",
        manifest.package.namespace, manifest.package.slug
    ));
    config.verbose = opts.verbose;
    config.record_profile = opts.record_profile;
    config.profile = manifest.build.profile.clone();
    config.multi_progress = opts.multi_progress.clone();
    config
}

fn maybe_generate_outputs(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    base_dir: &str,
    build_env: &BuildEnvironment,
) -> io::Result<()> {
    if !opts.generate_outputs {
        return Ok(());
    }

    let out_dir = Path::new(base_dir).join(&build_env.paths.out);
    let categorized = categorize_files_with_existing_outputs(&out_dir, &manifest.outputs);
    crate::manifest::update::write_auto_outputs_to_manifest(&opts.manifest_file, &categorized)?;

    if let ManifestData::Package(reloaded_manifest) = load_manifest(&opts.manifest_file)? {
        *manifest = reloaded_manifest;
    }
    Ok(())
}
