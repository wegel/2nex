use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use crate::manifest::*;
use crate::ostree::*;
use crate::outputs::*;
use crate::runtime::scanner::RuntimeScanResult;

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
        checkout_ostree_into(repo_path, &branch_name, &out_dir, true, false)?;
    }

    Ok(())
}

pub fn setup_composite_rootfs(
    base_dir: &str,
    repo_path: &str,
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
        checkout_ostree_into(repo_path, commit, Path::new(base_dir), true, false)?;
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
    commits: &[String],
) -> io::Result<()> {
    for commit in commits {
        checkout_ostree_into(repo_path, commit, Path::new(base_dir), true, false)?;
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

pub fn run_build_script(
    build_script: &str,
    build_dir: &str,
    env_vars: &HashMap<String, String>,
    bootstrap: bool,
) -> io::Result<()> {
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

    println!(
        "Executing build script under unshare with env vars: {:?}",
        env
    );
    let mut child = command.spawn()?;

    // use the result of wait() to determine if the build script succeeded or not
    let result = child.wait()?;
    if result.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Build script failed"))
    }
}

pub fn verify_and_commit_outputs(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    runtime_suggestions: Option<&RuntimeScanResult>,
    verbose_reasons: bool,
    manifest_path: &Path,
) -> io::Result<()> {
    println!("Verifying and committing outputs to OSTree branches");

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
    print_outputs(&outputs, runtime_suggestions, verbose_reasons);

    for (output_type, spec) in output_specs {
        if output_type == "discard" {
            continue;
        }

        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            output_type
        );

        for file_path in &spec.files {
            let source_path = out_dir.join(file_path.trim_start_matches('/'));
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

            accounted_files.push(file_path.trim_start_matches('/').to_string());
        }

        let commit_output_dir = out_dir.join(output_type);

        let metadata = output_branch_metadata(manifest, spec, &manifest_hash)?;
        commit_to_ostree(repo_path, &branch_name, &commit_output_dir, &metadata)?;
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
