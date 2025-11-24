use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::process;

use crate::build::*;
use crate::deps::*;
use crate::manifest::*;
use crate::ostree::*;
use crate::outputs::calculate_output_checksum;

// Use Opts from parent
use crate::Opts;

pub fn build_system_manifest(opts: &Opts, manifest: &SystemManifest) -> io::Result<()> {
    build_system_manifest_with_dir(opts, manifest, "./build_rootfs")
}

pub fn build_system_manifest_with_dir(
    opts: &Opts,
    manifest: &SystemManifest,
    base_dir: &str,
) -> io::Result<()> {
    if opts.update_outputs_requires || opts.update_outputs_requires_only {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "System manifests do not define outputs, so --update-outputs-requires flags are invalid.",
        ));
    }
    if opts.runtime_deps_verbose || opts.skip_runtime_deps {
        println!("Note: runtime dependency scanning is not available for system manifests yet.");
    }

    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    let dependency_commits = resolve_dependency_closure(&manifest.dependencies, &opts.repo_path)?;
    let package_dependency_specs = dependencies_from_system_packages(&manifest.packages);
    let package_commits = resolve_dependency_closure(&package_dependency_specs, &opts.repo_path)?;

    setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
    layer_commits_into_rootfs(base_dir, &opts.repo_path, &package_commits)?;
    materialize_system_packages(base_dir, &opts.repo_path, &package_commits)?;

    let env_vars = build_system_env_vars(manifest, download_dir, base_dir, opts.bootstrap)?;

    println!(
        "Building system {} {}",
        manifest.system.slug, manifest.system.version
    );

    run_build_script(&manifest.build.script, base_dir, &env_vars, opts.bootstrap)?;

    let target_dir = Path::new(base_dir).join("target");
    let checksum = calculate_output_checksum(&target_dir)?;
    println!("System build checksum: {}", checksum);

    match manifest.system.checksum.as_ref() {
        Some(expected_checksum) => {
            if checksum != *expected_checksum {
                if opts.update_checksum {
                    println!(
                        "System checksum mismatch (expected {}, calculated {}). Updating manifest.",
                        expected_checksum, checksum
                    );
                    update_manifest_checksum_field(
                        &opts.manifest_file,
                        ManifestKind::System,
                        &checksum,
                    )?;
                } else {
                    eprintln!(
                        "Checksum mismatch. Expected: {}, Calculated: {}",
                        expected_checksum, checksum
                    );
                    process::exit(-2);
                }
            } else {
                println!("Checksum verified successfully.");
            }
        }
        None => {
            if opts.update_checksum {
                println!(
                    "System manifest {} does not record a checksum. Storing {}.",
                    opts.manifest_file, checksum
                );
                update_manifest_checksum_field(
                    &opts.manifest_file,
                    ManifestKind::System,
                    &checksum,
                )?;
            }
        }
    }

    commit_system_rootfs(
        manifest,
        base_dir,
        &opts.repo_path,
        &package_commits,
        &dependency_commits,
        &checksum,
    )?;

    println!(
        "System commit stored at systems/{}/{}",
        manifest.system.slug, manifest.system.version
    );

    if opts.validate_reproducibility {
        println!("Validating build reproducibility by building the system a second time.");
        fs::remove_dir_all(base_dir)?;

        setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
        layer_commits_into_rootfs(base_dir, &opts.repo_path, &package_commits)?;
        materialize_system_packages(base_dir, &opts.repo_path, &package_commits)?;

        let env_vars = build_system_env_vars(manifest, download_dir, base_dir, opts.bootstrap)?;
        run_build_script(&manifest.build.script, base_dir, &env_vars, opts.bootstrap)?;

        let second_checksum = calculate_output_checksum(&target_dir)?;
        println!("Second build checksum: {}", second_checksum);

        if checksum == second_checksum {
            println!("Build is reproducible. Checksums match.");
        } else {
            println!("Build is not reproducible. Checksums do not match.");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Build is not reproducible.",
            ));
        }

        commit_system_rootfs(
            manifest,
            base_dir,
            &opts.repo_path,
            &package_commits,
            &dependency_commits,
            &second_checksum,
        )?;
    }

    Ok(())
}

pub fn build_system_env_vars(
    manifest: &SystemManifest,
    download_dir: &str,
    base_dir: &str,
    bootstrap: bool,
) -> io::Result<HashMap<String, String>> {
    let mut env_vars = handle_inputs(&manifest.sources, download_dir, base_dir, bootstrap)?;
    env_vars.insert("SYSTEM_NAME".to_string(), manifest.system.name.clone());
    env_vars.insert("SYSTEM_SLUG".to_string(), manifest.system.slug.clone());
    env_vars.insert(
        "SYSTEM_VERSION".to_string(),
        manifest.system.version.clone(),
    );
    env_vars.insert("TARGET_DIR".to_string(), "/target".to_string());
    env_vars.insert("SYSTEM_TARGET".to_string(), "/target".to_string());
    if let Some(arch) = &manifest.system.architecture {
        env_vars.insert("SYSTEM_ARCH".to_string(), arch.clone());
    }
    if let Some(boot) = &manifest.system.boot_method {
        env_vars.insert("SYSTEM_BOOT_METHOD".to_string(), boot.clone());
    }
    if let Some(desc) = &manifest.system.description {
        env_vars.insert("SYSTEM_DESCRIPTION".to_string(), desc.clone());
    }
    Ok(env_vars)
}

pub fn dependencies_from_system_packages(packages: &[SystemPackage]) -> Vec<Dependency> {
    packages
        .iter()
        .map(|pkg| Dependency {
            commit: pkg.commit.clone(),
            name: pkg.name.clone(),
            manifest_ref: None,
        })
        .collect()
}

pub fn materialize_system_packages(
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
) -> io::Result<()> {
    let target_dir = Path::new(base_dir).join("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    for commit in package_commits {
        checkout_ostree_into(repo_path, commit, &target_dir, true, false)?;
    }

    Ok(())
}

pub fn commit_system_rootfs(
    manifest: &SystemManifest,
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
    dependency_commits: &[String],
    checksum: &str,
) -> io::Result<()> {
    let target_dir = Path::new(base_dir).join("target");
    if !target_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "System target directory missing after build",
        ));
    }

    let branch_name = format!(
        "systems/{}/{}",
        manifest.system.slug, manifest.system.version
    );
    let mut metadata = Vec::new();
    metadata.push(("nex.system.name".to_string(), manifest.system.name.clone()));
    metadata.push(("nex.system.slug".to_string(), manifest.system.slug.clone()));
    metadata.push((
        "nex.system.version".to_string(),
        manifest.system.version.clone(),
    ));
    metadata.push(("nex.build.checksum".to_string(), checksum.to_string()));
    if let Some(desc) = &manifest.system.description {
        metadata.push(("nex.system.description".to_string(), desc.clone()));
    }
    if let Some(arch) = &manifest.system.architecture {
        metadata.push(("nex.system.arch".to_string(), arch.clone()));
    }
    if let Some(boot) = &manifest.system.boot_method {
        metadata.push(("nex.system.boot_method".to_string(), boot.clone()));
    }
    if let Some(encoded) = encode_metadata_list(package_commits)? {
        metadata.push(("nex.system.packages".to_string(), encoded));
    }
    if let Some(encoded) = encode_metadata_list(dependency_commits)? {
        metadata.push(("nex.system.dependencies".to_string(), encoded));
    }

    commit_to_ostree(repo_path, &branch_name, &target_dir, &metadata)
}
