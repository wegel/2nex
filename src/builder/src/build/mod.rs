use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

use sha2::{Digest, Sha256};

use crate::commands::build::BuildOpts;
use crate::manifest::*;
use crate::outputs::*;
use crate::progress::{self, BuildProgressConfig};
use crate::store::{
    checkout_into, checkout_into_with_fallbacks, commit_tree, ensure_branch_exists,
    find_commit_by_manifest_hash, rewrite_branch_metadata,
};

pub fn append_checksum_file(package: &Package, checksum: &str, file_path: &Path) -> io::Result<()> {
    // Step 1: Calculate the maximum width of the first column
    let mut max_first_column_width = 0;
    if file_path.exists() {
        let file = File::open(file_path)?;
        for line in io::BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 1 {
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
) -> io::Result<()> {
    let base_path = Path::new(base_dir);
    if base_path.exists() {
        fs::remove_dir_all(base_path)?;
    }
    let out_dir = base_path.join("2nex/out");
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
        checkout_into(repo_path, &branch_name, &out_dir, true, false)?;
    }

    Ok(())
}

pub fn setup_composite_rootfs(
    base_dir: &str,
    repo_path: &str,
    fallback_repos: &[String],
    dependency_commits: &[String],
) -> io::Result<()> {
    println!("Setting up composite rootfs at {}", base_dir);
    if Path::new(base_dir).exists() {
        fs::remove_dir_all(base_dir)?;
    }
    fs::create_dir_all(base_dir)?;

    let work_dir = Path::new(base_dir).join("2nex/work");
    let out_dir = Path::new(base_dir).join("2nex/out");
    let tmp_dir = Path::new(base_dir).join("tmp");

    for dir in &[&work_dir, &out_dir, &tmp_dir] {
        println!("Creating directory: {}", dir.display());
        if dir.exists() {
            println!("Removing existing {}", dir.display());
            fs::remove_dir_all(dir)?;
        }
        fs::create_dir_all(dir)?;
    }

    for commit in dependency_commits {
        checkout_into_with_fallbacks(repo_path, fallback_repos, commit, Path::new(base_dir), true)?;
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
) -> io::Result<()> {
    for commit in commits {
        checkout_into_with_fallbacks(repo_path, fallback_repos, commit, Path::new(base_dir), true)?;
    }
    Ok(())
}

pub fn handle_inputs(
    sources: &[Source],
    download_dir: &str,
    build_dir: &str,
    is_bootstrap: bool,
) -> io::Result<HashMap<String, String>> {
    println!("Handling inputs");

    let mut input_env_vars = HashMap::new();
    let current_dir = env::current_dir().expect("Failed to get current directory");

    for (i, source) in sources.iter().enumerate() {
        let input = fetch_and_verify_input(source, download_dir)?;
        let inputs_dir = Path::new(build_dir).join("inputs");
        fs::create_dir_all(&inputs_dir)?;
        let input_path = inputs_dir.join(input.file_name().unwrap());
        fs::copy(input.clone(), &input_path)?;

        let path_str = if is_bootstrap {
            current_dir.join(&input_path).to_str().unwrap().to_string()
        } else {
            let relative_path = Path::new("./inputs").join(input_path.file_name().unwrap());
            relative_path.to_str().unwrap().to_string()
        };

        input_env_vars.insert(format!("SOURCE{}", i), path_str.clone());
        let var_name = format!("SOURCE_{}", source.name);
        input_env_vars.insert(var_name, path_str);
    }

    Ok(input_env_vars)
}

/// result from running a build script with progress tracking
pub struct BuildScriptResult {
    /// new profile if one was recorded (each element is "bytes:time_ms")
    pub new_profile: Option<Vec<String>>,
}

pub fn run_build_script(
    build_script: &str,
    build_dir: &str,
    env_vars: &HashMap<String, String>,
    bootstrap: bool,
) -> io::Result<()> {
    // legacy mode: no progress tracking
    run_build_script_with_progress(build_script, build_dir, env_vars, bootstrap, None)?;
    Ok(())
}

pub fn run_build_script_with_progress(
    build_script: &str,
    build_dir: &str,
    env_vars: &HashMap<String, String>,
    bootstrap: bool,
    progress_config: Option<&BuildProgressConfig>,
) -> io::Result<BuildScriptResult> {
    println!("Running build script in an isolated environment using unshare");

    let mut env = HashMap::new();
    env.insert("HOME".to_string(), "/homeless/deterministic".to_string());
    env.insert("LC_ALL".to_string(), "C".to_string());
    env.insert("TZ".to_string(), "UTC".to_string());
    env.insert("LANG".to_string(), "en_US.UTF-8".to_string());
    env.insert("SOURCE_DATE_EPOCH".to_string(), "1704067200".to_string());
    env.insert(
        "GLIBC_TUNABLES".to_string(),
        "glibc.cpu.hwcaps=-RNDRAND".to_string(),
    );
    let num_cpus = num_cpus::get();
    env.insert("MAKEFLAGS".to_string(), format!("-j{num_cpus}"));

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
    let tmpdir_path = build_dir_abs.join("2nex").join("tmp");
    std::fs::create_dir_all(&tmpdir_path)?;

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
    let launch_script = if bootstrap {
        let bootstrap_sysroot_path = build_dir_abs.join("bootstrap");
        let bootstrap_tools_path = bootstrap_sysroot_path.join("tools");
        let bootstrap_tools_path_display = bootstrap_tools_path.display();
        let workdir_path = build_dir_abs.join("2nex").join("work");
        let outdir_path = build_dir_abs.join("2nex").join("out");

        println!("Bootstrap mode.");

        // Unset all environment variables
        for (key, _) in std::env::vars() {
            std::env::remove_var(key);
        }

        env.insert(
            "PATH".to_string(),
            format!("{bootstrap_tools_path_display}/bin:/usr/bin").to_string(),
        );
        env.insert("TARGET".to_string(), "x86_64-2nex-linux-gnu".to_string());
        env.insert(
            "WORK_DIR".to_string(),
            workdir_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "OUT_DIR".to_string(),
            outdir_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "BOOTSTRAP_TOOLS".to_string(),
            bootstrap_tools_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "BOOTSTRAP_SYSROOT".to_string(),
            bootstrap_sysroot_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "CFLAGS".to_string(),
            "-march=x86-64 -mtune=generic -O2 -frandom-seed=424242".to_string(),
        );
        env.insert(
            "CXXFLAGS".to_string(),
            "-march=x86-64 -mtune=generic -O2 -frandom-seed=424242".to_string(),
        );
        env.insert(
            "LDFLAGS".to_string(),
            "-L/bootstrap/usr/lib -Wl,-O1,--sort-common,--as-needed,-z,now".to_string(),
        );

        format!(
            r#"
            {build_script}"#
        )
    } else {
        env.insert(
            "PATH".to_string(),
            "/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
        );
        env.insert("RUSTC_BOOTSTRAP".to_string(), "1".to_string());
        env.insert("CFLAGS".to_string(), "-march=x86-64 -mtune=generic -O2 -pipe -fno-plt -fexceptions -Wp,-D_FORTIFY_SOURCE=2 -Wformat -Werror=format-security -fstack-clash-protection -fcf-protection -fPIC -fno-common -fno-omit-frame-pointer -frandom-seed=424242".to_string());
        env.insert("CXXFLAGS".to_string(), "-march=x86-64 -mtune=generic -O2 -pipe -fno-plt -fexceptions -Wp,-D_FORTIFY_SOURCE=2 -Wformat -Werror=format-security -fstack-clash-protection -fcf-protection -Wp,-D_GLIBCXX_ASSERTIONS -fPIC -fno-common -fno-omit-frame-pointer -frandom-seed=424242".to_string());
        env.insert(
            "LDFLAGS".to_string(),
            "-Wl,-O1,--sort-common,--as-needed,-z,relro,-z,now".to_string(),
        );
        env.insert("LTOFLAGS".to_string(), "-flto=auto".to_string());
        env.insert("RUSTFLAGS".to_string(), "-C codegen-units=1 -C embed-bitcode=yes -C debuginfo=0 -C link-args=-fuse-ld=lld -C target-feature=+crt-static -C link-args=-frandom-seed=424242".to_string());
        env.insert("DEBUG_CFLAGS".to_string(), "-g".to_string());
        env.insert("DEBUG_CXXFLAGS".to_string(), "-g".to_string());
        env.insert("DEBUG_RUSTFLAGS".to_string(), "-C debuginfo=2".to_string());
        env.insert("TARGET".to_string(), "x86_64-pc-linux-gnu".to_string());
        env.insert("WORK_DIR".to_string(), "/2nex/work".to_string());
        env.insert("OUT_DIR".to_string(), "/2nex/out".to_string());

        let temp_file_path = tmpdir_path.join("build_script.sh");
        let mut temp_file = std::fs::File::create(&temp_file_path)?;

        // Write the build_script content to the temporary file
        temp_file.write_all(b"#!/usr/bin/bash -eu\n")?;
        temp_file.write_all(build_script.as_bytes())?;

        format!(
            r#"
            if ! read -r current_hostname < /proc/sys/kernel/hostname; then
                echo 'Warning: Could not read current hostname; forcing to 2nex-builder' >&2
                current_hostname=""
            fi

            if [ "$current_hostname" != "2nex-builder" ]; then
                # Can't write /proc/sys/kernel/hostname inside a user namespace,
                # so set it here via the host's hostname binary before chrooting.
                echo 'Setting hostname to 2nex-builder'
                if ! hostname 2nex-builder; then
                    echo 'Warning: Failed to run hostname command' >&2
                fi
            fi

            mkdir -p {build_dir}/dev
            for D in null zero random urandom tty console full; do
                touch {build_dir}/dev/$D
                mount --bind /dev/$D {build_dir}/dev/$D
            done

            mkdir -p {build_dir}/dev/pts
            mount -t devpts devpts {build_dir}/dev/pts
            ln -sf /dev/pts/ptmx {build_dir}/dev/ptmx

            # remove symlinks for usrmerge compatibility (only remove if symlink, not dir)
            if [ -L {build_dir}/lib ]; then
                rm -f {build_dir}/lib
            fi

            if [ -L {build_dir}/lib64 ]; then
                rm -f {build_dir}/lib64
            fi

            if [ -L {build_dir}/sbin ]; then
                rm -f {build_dir}/sbin
            fi

            if [ -L {build_dir}/usr/lib64 ]; then
                rm -f {build_dir}/usr/lib64
            fi

            if [ -L {build_dir}/usr/sbin ]; then
                rm -f {build_dir}/usr/sbin
            fi

            mkdir -p {build_dir}/usr

            if [ ! -e {build_dir}/bin ]; then
                ln -sf /usr/bin {build_dir}/bin
            fi

            if [ ! -e {build_dir}/lib ]; then
                ln -sf /usr/lib {build_dir}/lib
            fi

            if [ ! -e {build_dir}/sbin ]; then
                ln -sf /usr/bin {build_dir}/sbin
            fi

            if [ ! -e {build_dir}/lib64 ]; then
                ln -sf /usr/lib {build_dir}/lib64
            fi

            if [ ! -e {build_dir}/usr/lib64 ]; then
                ln -sf lib {build_dir}/usr/lib64
            fi

            if [ ! -e {build_dir}/usr/sbin ]; then
                ln -sf bin {build_dir}/usr/sbin
            fi

            # create same FHS symlinks in /target for system builds
            if [ -d {build_dir}/target ]; then
                mkdir -p {build_dir}/target/usr

                if [ ! -e {build_dir}/target/bin ]; then
                    ln -sf /usr/bin {build_dir}/target/bin
                fi

                if [ ! -e {build_dir}/target/lib ]; then
                    ln -sf /usr/lib {build_dir}/target/lib
                fi

                if [ ! -e {build_dir}/target/sbin ]; then
                    ln -sf /usr/bin {build_dir}/target/sbin
                fi

                if [ ! -e {build_dir}/target/lib64 ]; then
                    ln -sf /usr/lib {build_dir}/target/lib64
                fi

                if [ ! -e {build_dir}/target/usr/lib64 ]; then
                    ln -sf lib {build_dir}/target/usr/lib64
                fi

                if [ ! -e {build_dir}/target/usr/sbin ]; then
                    ln -sf bin {build_dir}/target/usr/sbin
                fi
            fi

            chmod +x {build_dir}/2nex/tmp/build_script.sh
            unshare --root={build_dir} /2nex/tmp/build_script.sh
            "#,
            build_dir = build_dir_str
        )
    };
    command_args.push(&launch_script);

    for (key, value) in env_vars {
        env.insert(key.to_string(), value.to_string());
    }

    let mut command = Command::new("unshare");
    command.args(&command_args).envs(&env);

    // configure stdout/stderr based on progress config
    // when no profile exists, show all output since we can't show meaningful progress
    let show_output = progress_config
        .map(|c| c.verbose || c.profile.is_empty())
        .unwrap_or(true);

    if progress_config.is_some() {
        command.stdout(Stdio::piped());
        if show_output {
            // show output mode: stderr goes to terminal
            command.stderr(Stdio::inherit());
        } else {
            // quiet mode: capture stderr for clean progress bar
            command.stderr(Stdio::piped());
        }
    }

    println!(
        "Executing build script under unshare with env vars: {:?}",
        env
    );
    let mut child = command.spawn()?;

    // handle progress tracking if configured
    let (new_profile, captured_stderr) = if let Some(config) = progress_config {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "failed to capture stdout"))?;

        // capture stderr in a separate thread if in quiet mode (has profile, not verbose)
        let stderr_handle = if !show_output {
            let stderr = child.stderr.take();
            stderr.map(|stderr| {
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    let mut reader = std::io::BufReader::new(stderr);
                    let _ = std::io::Read::read_to_end(&mut reader, &mut buf);
                    buf
                })
            })
        } else {
            None
        };

        let result = progress::run_with_progress(stdout, config)?;

        // collect stderr if we captured it
        let stderr_output = stderr_handle.and_then(|h| h.join().ok());

        (result.new_profile, stderr_output)
    } else {
        (None, None)
    };

    // use the result of wait() to determine if the build script succeeded or not
    let result = child.wait()?;
    if result.success() {
        Ok(BuildScriptResult { new_profile })
    } else {
        // show captured stderr on failure
        if let Some(stderr) = captured_stderr {
            if !stderr.is_empty() {
                eprintln!("\n--- build stderr ---");
                let _ = std::io::Write::write_all(&mut std::io::stderr(), &stderr);
                eprintln!("--- end stderr ---\n");
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "Build script failed"))
    }
}

pub fn verify_and_commit_outputs(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    manifest_path: &Path,
) -> io::Result<()> {
    println!("Verifying and committing outputs to store branches");

    // compute manifest hash once
    let manifest_hash = crate::compute_manifest_hash(manifest_path)?;

    let output_specs = &manifest.outputs;
    let out_dir = Path::new(base_dir).join("2nex/out");

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

    let outputs = categorize_files(&out_dir);
    println!("Suggested manifest outputs:");
    print_outputs(&outputs);

    for (output_type, spec) in output_specs {
        if output_type == "discard" {
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
        commit_tree(repo_path, &branch_name, &commit_output_dir, &metadata)?;
    }

    let unaccounted_files: Vec<String> = all_out_files
        .into_iter()
        .filter(|f| !accounted_files.contains(f))
        .collect();
    if !unaccounted_files.is_empty() {
        println!(
            "The following files in /2nex/out are not accounted for in the manifest outputs: {:?}",
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

    let manifest_hash = crate::compute_manifest_hash(manifest_path)?;
    let bundles = &manifest.bundles;

    for (bundle_name, bundle) in bundles {
        commit_bundle(repo_path, bundle_name, bundle, manifest, &manifest_hash)?;
    }

    Ok(())
}

// ============================================================================
// public build API - entry points for building packages
// ============================================================================

/// build a single package or system from a manifest file.
/// this is the main entry point for the build command.
pub fn build_single(opts: &BuildOpts) -> io::Result<()> {
    let manifest_data = load_manifest(&opts.manifest_file)?;

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

    match manifest_data {
        ManifestData::Package(mut manifest) => {
            if opts.refresh_metadata {
                refresh_package_metadata(&opts.repo_path, &manifest, Path::new(&opts.manifest_file))
            } else {
                println!("Building package: {}", manifest.package.slug);
                let build_dir = opts.build_dir.clone().unwrap_or_else(|| {
                    format!(
                        "./build_rootfs_{}_{}",
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
                    "./build_rootfs_{}_system",
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

    // resolve dependencies to specific commit IDs when they have manifest_ref
    let dependency_commits = resolve_dependency_commits(&manifest.dependencies, &opts.repo_path)?;

    setup_composite_rootfs(
        base_dir,
        &opts.repo_path,
        &opts.fallback_repos,
        &dependency_commits,
    )?;
    let input_env_vars = handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;

    let package_name = &manifest.package.name;
    let package_version = &manifest.package.version;
    let package_namespace = &manifest.package.namespace;

    println!(
        "Building {} {} in namespace {}",
        package_name, package_version, package_namespace
    );

    let mut env_vars = HashMap::new();
    env_vars.extend(input_env_vars);
    let build_script = manifest.build.script.clone();

    // run build with progress tracking if enabled
    let build_result = if opts.no_progress {
        run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;
        BuildScriptResult { new_profile: None }
    } else {
        let mut progress_config = BuildProgressConfig::new(&manifest.package.slug);
        progress_config.verbose = opts.verbose;
        // only record if explicitly requested - never auto-modify manifest
        progress_config.record_profile = opts.record_profile;
        progress_config.profile = manifest.build.profile.clone();
        run_build_script_with_progress(
            &build_script,
            base_dir,
            &env_vars,
            opts.bootstrap,
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
        let out_dir = Path::new(base_dir).join("2nex/out");
        let categorized = categorize_files(&out_dir);
        crate::manifest::update::write_auto_outputs_to_manifest(&opts.manifest_file, &categorized)?;

        // reload the manifest to pick up the new outputs
        let reloaded = load_manifest(&opts.manifest_file)?;
        if let ManifestData::Package(reloaded_manifest) = reloaded {
            *manifest = reloaded_manifest;
        }
    }

    verify_and_commit_outputs(
        manifest,
        base_dir,
        &opts.repo_path,
        Path::new(&opts.manifest_file),
    )?;

    create_and_commit_bundles(
        manifest,
        base_dir,
        &opts.repo_path,
        Path::new(&opts.manifest_file),
    )?;

    println!("Build, packaging, and commit completed for all outputs.");

    let output_dir = Path::new(base_dir).join("2nex/out");
    let checksum = calculate_output_checksum(&output_dir)?;
    println!("Build output checksum: {}", checksum);

    match manifest.package.checksum.as_ref() {
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
                    // refresh store metadata with new manifest hash
                    refresh_package_metadata(
                        &opts.repo_path,
                        manifest,
                        Path::new(&opts.manifest_file),
                    )?;
                } else if !opts.check {
                    // only exit on mismatch if we're not validating reproducibility
                    // (reproducibility check compares two builds, not against stored checksum)
                    eprintln!(
                        "Checksum mismatch. Expected: {}, Calculated: {}",
                        expected_checksum, checksum
                    );
                    std::process::exit(-2);
                } else {
                    println!(
                        "Note: checksum differs from manifest (expected {}, got {}). Proceeding with reproducibility check.",
                        expected_checksum, checksum
                    );
                }
            } else {
                println!("Checksum verified successfully.");
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
                // refresh store metadata with new manifest hash
                refresh_package_metadata(
                    &opts.repo_path,
                    manifest,
                    Path::new(&opts.manifest_file),
                )?;
            }
        }
    }

    // create {hash}/files commit (union of all outputs) for dependency resolution
    create_files_commit_for_package(manifest, &opts.repo_path, Path::new(&opts.manifest_file))?;

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
        )?;
        handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;
        run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;
        verify_and_commit_outputs(
            manifest,
            base_dir,
            &opts.repo_path,
            Path::new(&opts.manifest_file),
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
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Build is not reproducible.",
            ));
        }
    }

    append_checksum_file(&manifest.package, &checksum, &Path::new("checksums.txt"))?;

    Ok(())
}

/// refresh metadata on all output and bundle branches for a package.
pub fn refresh_package_metadata(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
) -> io::Result<()> {
    println!(
        "Refreshing store metadata for {}/{} ({})",
        manifest.package.slug, manifest.package.version, manifest.package.namespace
    );
    let manifest_hash = compute_manifest_hash(manifest_path)?;
    refresh_output_branches(repo_path, manifest, &manifest_hash)?;
    refresh_bundle_branches(repo_path, manifest, &manifest_hash)?;
    println!("Finished refreshing metadata for {}", manifest.package.slug);
    Ok(())
}

/// create {hash}/files commit as in-store union of all outputs.
fn create_files_commit_for_package(
    manifest: &Manifest,
    repo_path: &str,
    manifest_path: &Path,
) -> io::Result<()> {
    // determine address hash: checksum if stable, else manifest git blob SHA
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);

    let address_hash = if has_stable_checksum {
        manifest.package.checksum.clone().unwrap()
    } else {
        crate::utils::hash_file_content(manifest_path)?
    };

    let files_ref = format!("{}/files", address_hash);

    // build output refs
    let output_refs: Vec<String> = manifest
        .outputs
        .keys()
        .filter(|k| *k != "discard")
        .map(|name| {
            format!(
                "x86_64/{}/{}/{}/outputs/{}",
                manifest.package.namespace_path(),
                manifest.package.slug,
                manifest.package.version,
                name
            )
        })
        .collect();

    if output_refs.is_empty() {
        return Ok(());
    }

    println!("Creating files commit: {}", files_ref);

    let repo = zub::Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let ref_strs: Vec<&str> = output_refs.iter().map(|s| s.as_str()).collect();
    zub::ops::union_trees(
        &repo,
        &ref_strs,
        &files_ref,
        zub::ops::UnionOptions {
            on_conflict: zub::ops::ConflictResolution::Last,
            ..Default::default()
        },
    )
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // attach metadata
    let metadata = vec![
        ("nex.address_hash".to_string(), address_hash.clone()),
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
    rewrite_branch_metadata(repo_path, &files_ref, &metadata)?;

    // create semantic files ref (x86_64/pkg/{namespace}/{slug}/{version}/files)
    let semantic_files_ref = format!(
        "x86_64/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version
    );

    // resolve the commit hash from the checksum-based ref
    let commit_hash = zub::resolve_ref(&repo, &files_ref)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // write the semantic ref pointing to the same commit
    zub::write_ref(&repo, &semantic_files_ref, &commit_hash)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // attach manifest hash to semantic ref for staleness checks
    let manifest_hash = compute_manifest_hash(manifest_path)?;
    rewrite_branch_metadata(
        repo_path,
        &semantic_files_ref,
        &[
            ("nex.manifest.hash".to_string(), manifest_hash),
            ("nex.address_hash".to_string(), address_hash),
        ],
    )?;

    println!("Created semantic ref: {}", semantic_files_ref);

    Ok(())
}

fn refresh_output_branches(
    repo_path: &str,
    manifest: &Manifest,
    manifest_hash: &str,
) -> io::Result<()> {
    for (category, spec) in &manifest.outputs {
        if category == "discard" {
            continue;
        }
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            category
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = output_branch_metadata(manifest, spec, manifest_hash)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}

fn refresh_bundle_branches(
    repo_path: &str,
    manifest: &Manifest,
    manifest_hash: &str,
) -> io::Result<()> {
    for (bundle_name, bundle) in &manifest.bundles {
        let branch_name = format!(
            "x86_64/{}/{}/{}/bundles/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            bundle_name
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = bundle_branch_metadata(manifest, bundle, manifest_hash)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}

/// compute SHA256 hash of manifest file.
pub fn compute_manifest_hash(manifest_path: &Path) -> io::Result<String> {
    let contents = fs::read(manifest_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    Ok(format!("{:x}", hasher.finalize()))
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
