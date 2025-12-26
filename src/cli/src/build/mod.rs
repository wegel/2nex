pub mod orchestration;

use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

use sha2::{Digest, Sha256};

use crate::commands::build::BuildOpts;

/// Output category name for files that should be discarded (not committed to store)
pub const OUTPUT_DISCARD: &str = "discard";
use crate::manifest::types::{BuildEnvironment, BuildPaths};
use crate::manifest::*;
use crate::outputs::*;
use crate::progress::{self, BuildProgressConfig};
use crate::store::{
    checkout_into, checkout_into_with_fallbacks, commit_tree, create_artifact,
    ensure_branch_exists, find_commit_by_manifest_hash, lookup_artifact, rewrite_branch_metadata,
};

/// Check if a string looks like a git SHA1 (40 hex characters)
fn is_sha1(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Load a build environment from a git blob SHA1 or file path
pub fn load_environment(repo_path: &str, env_ref: &str) -> io::Result<BuildEnvironment> {
    let content = if is_sha1(env_ref) {
        // load from git blob
        let output = Command::new("git")
            .args(["-C", repo_path, "cat-file", "blob", env_ref])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Failed to load environment blob {}: {}",
                    env_ref,
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        // load from file path
        fs::read_to_string(env_ref).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to load environment file {}: {}", env_ref, e),
            )
        })?
    };

    let env: BuildEnvironment = serde_yaml::from_str(&content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse environment YAML: {}", e),
        )
    })?;

    Ok(env)
}

/// Expand template variables in a string
/// Supported: {num_cpus}, {build_dir}, {bootstrap_tools}, {bootstrap_sysroot}
pub fn expand_env_templates(
    value: &str,
    num_cpus: usize,
    build_dir: &str,
    bootstrap_tools: Option<&str>,
    bootstrap_sysroot: Option<&str>,
) -> String {
    let mut result = value.to_string();
    result = result.replace("{num_cpus}", &num_cpus.to_string());
    result = result.replace("{build_dir}", build_dir);
    if let Some(tools) = bootstrap_tools {
        result = result.replace("{bootstrap_tools}", tools);
    }
    if let Some(sysroot) = bootstrap_sysroot {
        result = result.replace("{bootstrap_sysroot}", sysroot);
    }
    result
}

pub fn append_checksum_file(package: &Package, checksum: &str, file_path: &Path) -> io::Result<()> {
    // Step 1: Calculate the maximum width of the first column
    let mut max_first_column_width = 0;
    if file_path.exists() {
        let file = File::open(file_path)?;
        for line in io::BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                max_first_column_width = max_first_column_width.max(parts[0].len());
            }
        }
    }

    // Step 2: Append the new entry to the file
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;
    if max_first_column_width > 0 {
        // If the file was not empty, add a newline before appending
        writeln!(file)?;
    }
    let padded_first_column = format!(
        "{:<width$}",
        package.to_string(),
        width = max_first_column_width + 3 // Adjust for padding
    );
    writeln!(file, "{} {}", padded_first_column, checksum)?;

    Ok(())
}

pub fn stage_existing_outputs(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    paths: &BuildPaths,
) -> io::Result<()> {
    let base_path = Path::new(base_dir);
    if base_path.exists() {
        fs::remove_dir_all(base_path)?;
    }
    let out_dir = base_path.join(&paths.out);
    fs::create_dir_all(&out_dir)?;

    for category in manifest.outputs.keys() {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            category
        );
        println!(
            "Checking out existing outputs from {} into {}",
            branch_name,
            out_dir.display()
        );
        checkout_into(repo_path, &branch_name, &out_dir, true)?;
    }

    Ok(())
}

pub fn setup_composite_rootfs(
    base_dir: &str,
    repo_path: &str,
    fallback_repos: &[String],
    dependency_commits: &[String],
    paths: &BuildPaths,
    verbose: bool,
) -> io::Result<()> {
    println!("Setting up composite rootfs at {}", base_dir);
    if Path::new(base_dir).exists() {
        fs::remove_dir_all(base_dir)?;
    }
    fs::create_dir_all(base_dir)?;

    let work_dir = Path::new(base_dir).join(&paths.work);
    let out_dir = Path::new(base_dir).join(&paths.out);
    let tmp_dir = Path::new(base_dir).join("tmp");

    for dir in &[&work_dir, &out_dir, &tmp_dir] {
        if verbose {
            println!("Creating directory: {}", dir.display());
        }
        if dir.exists() {
            if verbose {
                println!("Removing existing {}", dir.display());
            }
            fs::remove_dir_all(dir)?;
        }
        fs::create_dir_all(dir)?;
    }

    for commit in dependency_commits {
        checkout_into_with_fallbacks(repo_path, fallback_repos, commit, Path::new(base_dir), true, verbose)?;
    }

    // create FHS compatibility symlinks (only if there are actual dependencies to checkout)
    if !dependency_commits.is_empty() {
        let symlinks = vec![
            ("bin", "/usr/bin"),
            ("lib", "/usr/lib"),
            ("sbin", "/usr/bin"),
            ("lib64", "/usr/lib"),
            ("usr/lib64", "lib"),
            ("usr/sbin", "bin"),
        ];

        for (link_path, target) in symlinks {
            let full_link_path = Path::new(base_dir).join(link_path);
            if !full_link_path.exists() {
                if let Some(parent) = full_link_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                std::os::unix::fs::symlink(target, &full_link_path)?;
            }
        }
    }

    Ok(())
}

pub fn layer_commits_into_rootfs(
    base_dir: &str,
    repo_path: &str,
    fallback_repos: &[String],
    commits: &[String],
    verbose: bool,
) -> io::Result<()> {
    for commit in commits {
        checkout_into_with_fallbacks(repo_path, fallback_repos, commit, Path::new(base_dir), true, verbose)?;
    }
    Ok(())
}

pub fn handle_inputs(
    sources: &[Source],
    download_dir: &str,
    build_dir: &str,
    use_absolute_paths: bool,
    paths: &BuildPaths,
    canonical_prefix: Option<&str>,
) -> io::Result<HashMap<String, String>> {
    println!("Handling inputs");

    let mut input_env_vars = HashMap::new();

    for (i, source) in sources.iter().enumerate() {
        let input = fetch_and_verify_input(source, download_dir)?;
        let inputs_dir = Path::new(build_dir).join(&paths.inputs);
        fs::create_dir_all(&inputs_dir)?;
        let input_path = inputs_dir.join(input.file_name().unwrap());
        fs::copy(input.clone(), &input_path)?;

        // non-chroot builds use canonical prefix path for portability
        // chroot builds use relative paths
        let path_str = if use_absolute_paths {
            let prefix = canonical_prefix.unwrap_or("/tmp/bootstrap");
            format!(
                "{}/{}/{}",
                prefix,
                paths.inputs,
                input_path.file_name().unwrap().to_str().unwrap()
            )
        } else {
            format!(
                "./{}/{}",
                paths.inputs,
                input_path.file_name().unwrap().to_str().unwrap()
            )
        };

        input_env_vars.insert(format!("SOURCE{}", i), path_str.clone());
        // replace hyphens with underscores for valid bash variable names
        let safe_name = source.name.replace('-', "_");
        let var_name = format!("SOURCE_{}", safe_name);
        input_env_vars.insert(var_name, path_str);

        // mark dev sources with IS_DEV env var
        if source.dev.is_some() {
            let is_dev_var = format!("SOURCE_{}_IS_DEV", safe_name);
            input_env_vars.insert(is_dev_var, "1".to_string());
        }
    }

    Ok(input_env_vars)
}

/// result from running a build script with progress tracking
pub struct BuildScriptResult {
    /// new profile if one was recorded (each element is "bytes:time_ms")
    pub new_profile: Option<Vec<String>>,
}

/// Run a build script using the specified environment definition
pub fn run_build_script_with_env(
    build_script: &str,
    build_dir: &str,
    input_env_vars: &HashMap<String, String>,
    build_env: &BuildEnvironment,
    progress_config: Option<&BuildProgressConfig>,
) -> io::Result<BuildScriptResult> {
    println!(
        "Running build script in isolated environment using '{}' environment",
        build_env.name
    );

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let build_dir_path = Path::new(build_dir);
    let build_dir_abs = if build_dir_path.is_absolute() {
        build_dir_path.to_path_buf()
    } else {
        current_dir.join(build_dir_path)
    };
    let build_dir_str = build_dir_abs
        .to_str()
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "build_dir must be valid UTF-8")
        })?
        .to_string();
    let tmpdir_path = build_dir_abs.join("nex").join("tmp");
    std::fs::create_dir_all(&tmpdir_path)?;

    // compute template variable values
    let num_cpus = num_cpus::get();
    let bootstrap_sysroot_path = build_dir_abs.join("bootstrap");
    let bootstrap_tools_path = bootstrap_sysroot_path.join("tools");
    let bootstrap_tools_str = bootstrap_tools_path.to_str().unwrap();
    let bootstrap_sysroot_str = bootstrap_sysroot_path.to_str().unwrap();

    // build environment variables from the environment definition
    let mut env = HashMap::new();

    // for non-chroot mode, unset all host environment variables first
    if !build_env.execution.chroot {
        for (key, _) in std::env::vars() {
            std::env::remove_var(key);
        }
    }

    // apply env vars from the environment definition with template expansion
    for (key, value) in &build_env.env {
        let expanded = expand_env_templates(
            value,
            num_cpus,
            &build_dir_str,
            Some(bootstrap_tools_str),
            Some(bootstrap_sysroot_str),
        );
        env.insert(key.clone(), expanded);
    }

    // add input env vars (SOURCE_*, etc.) - these override environment defaults
    for (key, value) in input_env_vars {
        env.insert(key.clone(), value.clone());
    }

    let unshare_command = vec![
        "unshare",
        "--user",
        "--pid",
        "--mount",
        "--uts",
        "--fork",
        "--ipc",
        "--net",
        "--map-root-user",
        "/usr/bin/bash",
        "-o",
        "errexit",
        "-o",
        "nounset",
        "-c",
    ];

    let mut command_args = unshare_command.clone();

    // construct the launch script based on execution mode
    let launch_script = if build_env.execution.chroot {
        // chroot mode: write build script to file, run preamble, then chroot and execute
        let temp_file_path = tmpdir_path.join("build_script.sh");
        let mut temp_file = std::fs::File::create(&temp_file_path)?;
        temp_file.write_all(b"#!/usr/bin/bash -eu\n")?;
        temp_file.write_all(build_script.as_bytes())?;

        // expand preamble templates
        let preamble = expand_env_templates(
            &build_env.preamble,
            num_cpus,
            &build_dir_str,
            Some(bootstrap_tools_str),
            Some(bootstrap_sysroot_str),
        );

        format!(
            r#"
{preamble}

chmod +x {build_dir}/nex/tmp/build_script.sh
unshare --root={build_dir} /nex/tmp/build_script.sh 2>&1
"#,
            preamble = preamble,
            build_dir = build_dir_str
        )
    } else {
        // non-chroot mode: run script directly on host filesystem
        // preamble runs first (if any), then the build script
        let preamble = expand_env_templates(
            &build_env.preamble,
            num_cpus,
            &build_dir_str,
            Some(bootstrap_tools_str),
            Some(bootstrap_sysroot_str),
        );

        if preamble.trim().is_empty() {
            build_script.to_string()
        } else {
            format!("{preamble}\n{build_script}")
        }
    };
    command_args.push(&launch_script);

    let mut command = Command::new("unshare");
    command.args(&command_args).envs(&env);

    // always pipe stdout (which includes stderr via 2>&1 in launch_script)
    // we capture everything and only display on verbose or error
    if progress_config.is_some() {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
    }

    println!(
        "Executing build script under unshare with env vars: {:?}",
        env
    );
    let mut child = command.spawn()?;

    // handle progress tracking if configured
    let (new_profile, captured_output) = if let Some(config) = progress_config {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("failed to capture stdout"))?;

        // capture any escaped stderr in background
        let stderr_handle = child.stderr.take().map(|stderr| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut reader = std::io::BufReader::new(stderr);
                let _ = std::io::Read::read_to_end(&mut reader, &mut buf);
                buf
            })
        });

        let result = progress::run_with_progress(stdout, config)?;

        // combine captured output from progress tracking with any escaped stderr
        let mut output = result.captured_output.unwrap_or_default();
        if let Some(stderr) = stderr_handle.and_then(|h| h.join().ok()) {
            output.extend(stderr);
        }

        (
            result.new_profile,
            if output.is_empty() {
                None
            } else {
                Some(output)
            },
        )
    } else {
        (None, None)
    };

    // use the result of wait() to determine if the build script succeeded or not
    let result = child.wait()?;
    if result.success() {
        Ok(BuildScriptResult { new_profile })
    } else {
        // show captured output on failure
        if let Some(output) = captured_output {
            if !output.is_empty() {
                eprintln!("\n--- build output ---");
                let _ = std::io::Write::write_all(&mut std::io::stderr(), &output);
                eprintln!("--- end output ---\n");
            }
        }
        Err(io::Error::other("Build script failed"))
    }
}

pub fn verify_and_commit_outputs(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    manifest_path: &Path,
    paths: &BuildPaths,
) -> io::Result<()> {
    println!("Verifying and committing outputs to store branches");

    // compute manifest hash once
    let manifest_hash = compute_manifest_hash(manifest_path)?;

    let output_specs = &manifest.outputs;
    let out_dir = Path::new(base_dir).join(&paths.out);

    let all_out_files: Vec<String> = WalkDir::new(&out_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() || e.file_type().is_symlink())
        .map(|e| {
            e.path()
                .strip_prefix(&out_dir)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();

    let mut accounted_files = Vec::new();

    for (output_type, spec) in output_specs {
        if output_type == OUTPUT_DISCARD {
            continue;
        }

        // skip outputs with no files
        if spec.files.is_empty() {
            continue;
        }

        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            output_type
        );

        for file_entry in &spec.files {
            let source_path = out_dir.join(file_entry.path.trim_start_matches('/'));
            if !source_path.is_symlink() && !source_path.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "File {} listed in manifest outputs does not exist.",
                        source_path.display()
                    ),
                ));
            }

            let target_dir_structure = source_path.parent().unwrap();
            let relative_parent = target_dir_structure.strip_prefix(&out_dir).unwrap();
            let output_dir = if relative_parent.as_os_str().is_empty() {
                // file is directly in out_dir (e.g., /init)
                out_dir.join(output_type)
            } else {
                out_dir.join(output_type).join(relative_parent)
            };

            // if output_dir path exists as a file (not dir), temporarily move it
            let temp_path = if output_dir.exists() && !output_dir.is_dir() {
                let tmp = output_dir.with_extension("tmp_rename");
                fs::rename(&output_dir, &tmp)?;
                Some(tmp)
            } else {
                None
            };

            fs::create_dir_all(&output_dir)?;

            // if we temporarily moved a file, move it back to its final location
            if let Some(tmp) = temp_path {
                fs::rename(&tmp, output_dir.join(source_path.file_name().unwrap()))?;
            } else {
                fs::rename(
                    &source_path,
                    output_dir.join(source_path.file_name().unwrap()),
                )?;
            }

            accounted_files.push(file_entry.path.trim_start_matches('/').to_string());
        }

        let commit_output_dir = out_dir.join(output_type);

        let metadata = output_branch_metadata(manifest, spec, &manifest_hash)?;
        let tree_hash = commit_tree(repo_path, &branch_name, &commit_output_dir, &metadata)?;

        // create artifact for this output
        let artifact_output = format!("outputs/{}", output_type);
        let artifact_path = format!(
            "x86_64/{}/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            manifest_hash,
            output_type
        );
        create_artifact(repo_path, &tree_hash, &manifest_hash, &artifact_output, &artifact_path)?;
    }

    let unaccounted_files: Vec<String> = all_out_files
        .into_iter()
        .filter(|f| !accounted_files.contains(f))
        .collect();
    if !unaccounted_files.is_empty() {
        println!(
            "The following files in /nex/out are not accounted for in the manifest outputs: {:?}",
            unaccounted_files
        );
    }

    Ok(())
}

pub fn create_and_commit_bundles(
    manifest: &Manifest,
    _base_dir: &str,
    repo_path: &str,
    manifest_path: &Path,
) -> io::Result<()> {
    println!("Processing bundles");

    let manifest_hash = compute_manifest_hash(manifest_path)?;
    let bundles = &manifest.bundles;

    for (bundle_name, bundle) in bundles {
        commit_bundle(repo_path, bundle_name, bundle, manifest, &manifest_hash)?;
    }

    Ok(())
}

/// commit raw build output files to `x86_64/{pkg}/{checksum}/files`.
/// called before outputs/bundles are created, so files can be compared
/// between builds when checksums don't match.
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

    let files_ref = format!(
        "x86_64/{}/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        checksum
    );
    println!("Committing raw build output to {}", files_ref);

    let metadata = vec![
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
    ];

    commit_tree(repo_path, &files_ref, &out_dir, &metadata)?;

    Ok(())
}

/// create semantic `x86_64/{pkg}/files` ref pointing to the checksum-based files commit.
/// only called after successful checksum verification.
fn create_semantic_files_ref(
    manifest: &Manifest,
    repo_path: &str,
    manifest_path: &Path,
    checksum: &str,
) -> io::Result<()> {
    let files_ref = format!(
        "x86_64/{}/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        checksum
    );
    let semantic_files_ref = format!(
        "x86_64/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version
    );

    println!("Creating semantic ref: {}", semantic_files_ref);

    let repo =
        zub::Repo::open(Path::new(repo_path)).map_err(|e| io::Error::other(e.to_string()))?;

    // resolve the commit hash from the checksum-based ref
    let commit_hash =
        zub::resolve_ref(&repo, &files_ref).map_err(|e| io::Error::other(e.to_string()))?;

    // write the semantic ref pointing to the same commit
    zub::write_ref(&repo, &semantic_files_ref, &commit_hash)
        .map_err(|e| io::Error::other(e.to_string()))?;

    // attach manifest hash to semantic ref for staleness checks
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

// ============================================================================
// public build API - entry points for building packages
// ============================================================================

/// build a single package or system from a manifest file.
/// this is the main entry point for the build command.
pub fn build_single(opts: &BuildOpts) -> io::Result<()> {
    // ensure tmp directory exists
    fs::create_dir_all(".nex/tmp")?;

    // detect manifest kind first to use inheritance for system manifests
    let manifest_str = fs::read_to_string(&opts.manifest_file)?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&manifest_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let manifest_data = if detect_manifest_kind(&doc) == ManifestKind::System {
        // use inheritance resolution for system manifests
        let resolved = load_system_manifest_resolved(Path::new(&opts.manifest_file))?;
        ManifestData::System(resolved)
    } else {
        load_manifest(&opts.manifest_file)?
    };

    // validate flags for refresh_metadata
    if opts.refresh_metadata {
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
    }

    // early staleness check for package manifests (unless force or check)
    if let ManifestData::Package(ref manifest) = manifest_data {
        if !opts.force && !opts.check && !opts.refresh_metadata {
            let manifest_path = Path::new(&opts.manifest_file);
            if let Ok(Some(_commit)) = check_if_built(&opts.repo_path, manifest, manifest_path) {
                println!(
                    "Package {}/{} already built, skipping (use --force to rebuild)",
                    manifest.package.namespace, manifest.package.slug
                );
                return Ok(());
            }
        }
    }

    match manifest_data {
        ManifestData::Package(mut manifest) => {
            if opts.refresh_metadata {
                refresh_package_metadata(&opts.repo_path, &manifest, Path::new(&opts.manifest_file))
            } else {
                println!(
                    "Building package: {}/{}",
                    manifest.package.namespace, manifest.package.slug
                );
                let build_dir = opts.build_dir.clone().unwrap_or_else(|| {
                    format!(
                        ".nex/tmp/build_rootfs_{}_{}",
                        manifest.package.slug.replace("/", "_"),
                        manifest.package.namespace.replace("/", "_")
                    )
                });
                build_package_manifest_with_dir(opts, &mut manifest, &build_dir)
            }
        }
        ManifestData::System(manifest) => {
            println!("Building system: {}", manifest.system.slug);
            let build_dir = opts.build_dir.clone().unwrap_or_else(|| {
                format!(
                    ".nex/tmp/build_rootfs_{}_system",
                    manifest.system.slug.replace("/", "_")
                )
            });
            crate::system::build_system_manifest_with_dir(opts, &manifest, &build_dir)
        }
    }
}

/// build a package manifest with a specific build directory.
pub fn build_package_manifest_with_dir(
    opts: &BuildOpts,
    manifest: &mut Manifest,
    base_dir: &str,
) -> io::Result<()> {
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    // load build environment from manifest
    let build_env = load_environment(&opts.repo_path, &manifest.build.environment)?;

    // resolve dependencies to specific commit IDs when they have manifest_ref
    let dependency_commits = resolve_dependency_commits(&manifest.dependencies, &opts.repo_path)?;

    setup_composite_rootfs(
        base_dir,
        &opts.repo_path,
        &opts.fallback_repos,
        &dependency_commits,
        &build_env.paths,
        opts.verbose,
    )?;

    // use_absolute_paths = !chroot (when not using chroot, we need absolute paths)
    // for non-chroot builds, use canonical /tmp/bootstrap prefix for portability
    let canonical_prefix = if !build_env.execution.chroot {
        Some("/tmp/bootstrap")
    } else {
        None
    };
    let input_env_vars = handle_inputs(
        &manifest.sources,
        download_dir,
        base_dir,
        !build_env.execution.chroot,
        &build_env.paths,
        canonical_prefix,
    )?;

    let package_name = &manifest.package.name;
    let package_version = &manifest.package.version;
    let package_namespace = &manifest.package.namespace;

    println!(
        "Building {} {} in namespace {} using '{}' environment",
        package_name, package_version, package_namespace, build_env.name
    );

    let build_script = manifest.build.script.clone();

    // run build with progress tracking if enabled
    let build_result = if opts.no_progress {
        run_build_script_with_env(&build_script, base_dir, &input_env_vars, &build_env, None)?
    } else {
        let mut progress_config = BuildProgressConfig::new(&format!(
            "{}/{}",
            manifest.package.namespace, manifest.package.slug
        ));
        progress_config.verbose = opts.verbose;
        // only record if explicitly requested - never auto-modify manifest
        progress_config.record_profile = opts.record_profile;
        progress_config.profile = manifest.build.profile.clone();
        progress_config.multi_progress = opts.multi_progress.clone();
        run_build_script_with_env(
            &build_script,
            base_dir,
            &input_env_vars,
            &build_env,
            Some(&progress_config),
        )?
    };

    // only update manifest if --record-profile was explicitly requested
    if opts.record_profile {
        if let Some(new_profile) = build_result.new_profile {
            crate::manifest::update::update_build_profile(&opts.manifest_file, &new_profile)?;
            manifest.build.profile = new_profile;
        }
    }

    // if generate_outputs is enabled, write auto-detected outputs to manifest
    if opts.generate_outputs {
        let out_dir = Path::new(base_dir).join(&build_env.paths.out);
        let categorized = categorize_files(&out_dir);
        crate::manifest::update::write_auto_outputs_to_manifest(&opts.manifest_file, &categorized)?;

        // reload the manifest to pick up the new outputs
        let reloaded = load_manifest(&opts.manifest_file)?;
        if let ManifestData::Package(reloaded_manifest) = reloaded {
            *manifest = reloaded_manifest;
        }
    }

    // calculate checksum BEFORE committing anything
    let output_dir = Path::new(base_dir).join(&build_env.paths.out);
    let checksum = calculate_output_checksum(&output_dir)?;
    println!("Build output checksum: {}", checksum);

    // always commit raw files first (before moving files to outputs)
    // this allows comparing builds when checksums don't match
    commit_raw_files(
        manifest,
        base_dir,
        &opts.repo_path,
        &build_env.paths,
        &checksum,
    )?;

    // determine if we should commit outputs/bundles based on checksum verification
    let should_commit = match manifest.package.checksum.as_ref() {
        Some(expected_checksum) => {
            if checksum != *expected_checksum {
                if opts.update_checksum {
                    println!(
                        "Checksum mismatch (expected {}, calculated {}). Updating manifest.",
                        expected_checksum, checksum
                    );
                    update_manifest_checksum_field(
                        &opts.manifest_file,
                        ManifestKind::Package,
                        &checksum,
                    )?;
                    manifest.package.checksum = Some(checksum.clone());
                    true // commit with updated checksum
                } else if opts.check {
                    println!(
                        "Note: checksum differs from manifest (expected {}, got {}). Proceeding with reproducibility check.",
                        expected_checksum, checksum
                    );
                    true // commit for reproducibility check
                } else {
                    // checksum mismatch: raw files already committed for debugging
                    eprintln!(
                        "Checksum mismatch. Expected: {}, Calculated: {}",
                        expected_checksum, checksum
                    );
                    eprintln!(
                        "Raw files committed to {}/files for debugging.",
                        checksum
                    );
                    eprintln!(
                        "Compare with: zub diff {}/files {}/files",
                        expected_checksum, checksum
                    );

                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Checksum mismatch. Expected: {}, Calculated: {}",
                            expected_checksum, checksum
                        ),
                    ));
                }
            } else {
                println!("Checksum verified successfully.");
                true
            }
        }
        None => {
            if opts.update_checksum {
                println!(
                    "Manifest {} does not record a checksum. Storing {}.",
                    opts.manifest_file, checksum
                );
                update_manifest_checksum_field(
                    &opts.manifest_file,
                    ManifestKind::Package,
                    &checksum,
                )?;
                manifest.package.checksum = Some(checksum.clone());
            }
            true // no expected checksum, always commit
        }
    };

    if should_commit {
        verify_and_commit_outputs(
            manifest,
            base_dir,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
            &build_env.paths,
        )?;

        create_and_commit_bundles(
            manifest,
            base_dir,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
        )?;

        println!("Build, packaging, and commit completed for all outputs.");

        // create semantic {pkg}/files ref pointing to the checksum-based files commit
        // (must happen before refresh_package_metadata which needs this ref)
        create_semantic_files_ref(manifest, &opts.repo_path, Path::new(&opts.manifest_file), &checksum)?;

        // refresh store metadata if checksum was updated
        if opts.update_checksum {
            refresh_package_metadata(
                &opts.repo_path,
                manifest,
                Path::new(&opts.manifest_file),
            )?;
        }
    }

    // compute runtime dependencies (opt-in, modifies manifest)
    if opts.compute_deps {
        crate::commands::compute_deps::compute_deps_for_manifest(
            manifest,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
            opts.runtime_deps_verbose,
            false, // never dry_run during build
        )?;
        // refresh store metadata since compute_deps modified the manifest
        refresh_package_metadata(&opts.repo_path, manifest, Path::new(&opts.manifest_file))?;
    }

    if opts.check {
        println!("Validating build reproducibility by building the package a second time.");
        fs::remove_dir_all(base_dir)?;
        setup_composite_rootfs(
            base_dir,
            &opts.repo_path,
            &opts.fallback_repos,
            &dependency_commits,
            &build_env.paths,
            opts.verbose,
        )?;
        let input_env_vars_2 = handle_inputs(
            &manifest.sources,
            download_dir,
            base_dir,
            !build_env.execution.chroot,
            &build_env.paths,
            canonical_prefix,
        )?;
        run_build_script_with_env(&build_script, base_dir, &input_env_vars_2, &build_env, None)?;
        verify_and_commit_outputs(
            manifest,
            base_dir,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
            &build_env.paths,
        )?;
        create_and_commit_bundles(
            manifest,
            base_dir,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
        )?;

        let second_checksum = calculate_output_checksum(&output_dir)?;
        println!("Second build output checksum: {}", second_checksum);

        if checksum == second_checksum {
            println!("Build is reproducible. Checksums match.");
        } else {
            println!("Build is not reproducible. Checksums do not match.");
            return Err(io::Error::other("Build is not reproducible."));
        }
    }

    append_checksum_file(&manifest.package, &checksum, Path::new("checksums.txt"))?;

    Ok(())
}

/// refresh outputs and bundles by checking out the files ref and re-splitting.
pub fn refresh_package_metadata(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
) -> io::Result<()> {
    println!(
        "Refreshing outputs/bundles for {}/{} ({})",
        manifest.package.slug, manifest.package.version, manifest.package.namespace
    );

    // find the files ref
    let files_ref = format!(
        "x86_64/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version
    );

    // verify files ref exists
    ensure_branch_exists(repo_path, &files_ref)?;

    // create temp directory in .nex/tmp (same filesystem as repo for hardlinks)
    let base_parent = Path::new(".nex/tmp");
    fs::create_dir_all(base_parent)?;
    let prefix = format!(
        "refresh_metadata_{}_{}_",
        manifest.package.slug.replace("/", "_"),
        manifest.package.namespace.replace("/", "_")
    );
    let temp_dir = tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir_in(base_parent)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("tempdir failed: {}", e)))?;
    let base_dir = temp_dir.path().to_path_buf();
    let out_dir = base_dir.join("out");
    fs::create_dir_all(&out_dir)?;

    println!("Checking out {} to temp directory", files_ref);
    checkout_into(repo_path, &files_ref, &out_dir, false)?;

    // create paths struct (out is relative to base_dir)
    let paths = BuildPaths {
        work: "work".to_string(),
        out: "out".to_string(),
        inputs: "inputs".to_string(),
    };

    // split files into outputs and commit
    verify_and_commit_outputs(manifest, base_dir.to_str().unwrap(), repo_path, manifest_path, &paths)?;

    // create bundles from outputs
    create_and_commit_bundles(manifest, base_dir.to_str().unwrap(), repo_path, manifest_path)?;

    // cleanup temp directory
    drop(temp_dir);

    println!("Finished refreshing outputs/bundles for {}", manifest.package.slug);
    Ok(())
}

/// compute SHA256 hash of manifest file.
pub fn compute_manifest_hash(manifest_path: &Path) -> io::Result<String> {
    let contents = fs::read(manifest_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    Ok(format!("{:x}", hasher.finalize()))
}

/// check if all outputs of a manifest are already built in the store with current manifest hash.
/// returns Some(commit_id) if found, None if not built or hash mismatch.
pub fn check_if_built(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
) -> io::Result<Option<String>> {
    let arch = "x86_64"; // TODO: make configurable
    let slug = &manifest.package.slug;
    let version = &manifest.package.version;
    let namespace = manifest.package.namespace_path();

    // compute current manifest hash
    let current_hash = compute_manifest_hash(manifest_path)?;

    // check all outputs - we'll use the first output to find the commit
    let mut found_commit: Option<String> = None;

    for output_name in manifest.outputs.keys() {
        let branch = format!(
            "{}/{}/{}/{}/outputs/{}",
            arch, namespace, slug, version, output_name
        );

        // try artifact lookup first (O(1))
        let artifact_path = format!(
            "{}/{}/{}/{}/{}/outputs/{}",
            arch, namespace, slug, version, current_hash, output_name
        );
        if let Ok(Some(_tree)) = lookup_artifact(repo_path, &artifact_path) {
            if found_commit.is_none() {
                found_commit = Some("artifact".to_string());
            }
            continue;
        }

        // check if branch exists
        if ensure_branch_exists(repo_path, &branch).is_err() {
            return Ok(None);
        }

        // fall back to commit history search
        match find_commit_by_manifest_hash(repo_path, &branch, &current_hash)? {
            Some(commit_id) => {
                if found_commit.is_none() {
                    found_commit = Some(commit_id);
                }
            }
            None => {
                // no commit found with matching hash
                return Ok(None);
            }
        }
    }

    Ok(found_commit)
}

/// resolve dependencies to specific commit IDs when they have manifest_ref.
fn resolve_dependency_commits(
    dependencies: &[Dependency],
    repo_path: &str,
) -> io::Result<Vec<String>> {
    let git_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut resolved = Vec::new();

    for dep in dependencies {
        if let Some(ref blob_sha) = dep.manifest_ref {
            // fetch blob content and compute its hash
            match crate::utils::fetch_git_blob(&git_root, blob_sha) {
                Ok(content) => {
                    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

                    // search store history for matching build
                    match find_commit_by_manifest_hash(repo_path, &dep.commit, &content_hash) {
                        Ok(Some(commit_id)) => {
                            // use the specific commit ID instead of branch name
                            resolved.push(commit_id);
                            continue;
                        }
                        Ok(None) => {
                            // fall back to branch name
                            eprintln!(
                                "Warning: no matching commit found for {} with manifest_ref {}",
                                dep.commit, blob_sha
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: failed to search history for {}: {}",
                                dep.commit, e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to fetch blob {} for {}: {}",
                        blob_sha, dep.commit, e
                    );
                }
            }
        }
        // no manifest_ref or resolution failed - use branch name
        resolved.push(dep.commit.clone());
    }

    Ok(resolved)
}
