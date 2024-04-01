use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use hostname;
use num_cpus;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use walkdir::WalkDir;

use content_disposition::parse_content_disposition;
use reqwest::header::{CONTENT_DISPOSITION, LOCATION};
use std::fmt;
use std::process;
use url::Url;

#[derive(Parser)]
#[clap(version = "1.0", author = "Your Name")]
struct Opts {
    #[clap(value_name = "REPO", help = "Path to the OSTree repository")]
    repo_path: String,
    #[clap(value_name = "MANIFEST", help = "Path to the manifest file")]
    manifest_file: String,
    #[clap(long, help = "Validate build reproducibility")]
    validate_reproducibility: bool,
    #[clap(
        long,
        help = "Run the build script on the host's filesystem (for bootstrapping)"
    )]
    bootstrap: bool,
}

#[derive(Deserialize)]
struct Manifest {
    package: Package,
    dependencies: Vec<Dependency>,
    sources: Vec<Source>,
    build: Build,
    outputs: HashMap<String, Vec<String>>,
    bundles: HashMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    slug: String,
    version: String,
    flavor: String,
    checksum: Option<String>,
    stable_checksum: Option<bool>,
}

#[derive(Deserialize)]
struct Dependency {
    commit: String,
}

#[derive(Deserialize, Debug)]
struct Source {
    name: String,
    url: Option<String>,
    file: Option<String>,
    sha256: String,
}

#[derive(Deserialize)]
struct Build {
    script: String,
}

fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();

    let manifest = load_manifest(&opts.manifest_file)?;
    let manifest_dir = Path::new(&opts.manifest_file).parent().unwrap();
    let base_dir = "./build_rootfs";
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    setup_composite_rootfs(&manifest, base_dir, &opts.repo_path)?;
    let input_env_vars = handle_inputs(
        &manifest,
        manifest_dir,
        download_dir,
        base_dir,
        opts.bootstrap,
    )?;

    let package_name = &manifest.package.name;
    let package_version = &manifest.package.version;
    let package_flavor = &manifest.package.flavor;

    println!(
        "Building {} {} for flavor {}",
        package_name, package_version, package_flavor
    );

    let mut env_vars = HashMap::new();
    env_vars.extend(input_env_vars);
    let build_script = &manifest.build.script;

    run_build_script(build_script, base_dir, &env_vars, opts.bootstrap)?;

    verify_and_commit_outputs(&manifest, base_dir, &opts.repo_path)?;

    create_and_commit_bundles(&manifest, base_dir, &opts.repo_path)?;
    println!("Build, packaging, and commit to OSTree completed for all outputs.");

    let output_dir = Path::new(base_dir).join("2nex/out");
    let checksum = calculate_output_checksum(&output_dir)?;
    println!("Build output checksum: {}", checksum);

    // Verify checksum if it's provided in the manifest
    if let Some(expected_checksum) = &manifest.package.checksum {
        if checksum != *expected_checksum {
            eprintln!(
                "Checksum mismatch. Expected: {}, Calculated: {}",
                expected_checksum, checksum
            );
            process::exit(-2);
        } else {
            println!("Checksum verified successfully.");
        }
    }

    if opts.validate_reproducibility {
        println!("Validating build reproducibility by building the package a second time.");
        fs::remove_dir_all(base_dir)?;
        setup_composite_rootfs(&manifest, base_dir, &opts.repo_path)?;
        handle_inputs(
            &manifest,
            manifest_dir,
            download_dir,
            base_dir,
            opts.bootstrap,
        )?;
        run_build_script(build_script, base_dir, &env_vars, opts.bootstrap)?;
        verify_and_commit_outputs(&manifest, base_dir, &opts.repo_path)?;
        create_and_commit_bundles(&manifest, base_dir, &opts.repo_path)?;

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

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.flavor, self.slug)
    }
}

fn append_checksum_file(package: &Package, checksum: &str, file_path: &Path) -> io::Result<()> {
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

fn load_manifest(file_path: &str) -> io::Result<Manifest> {
    let manifest_str = fs::read_to_string(file_path)?;
    let manifest: Manifest = serde_yaml::from_str(&manifest_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(manifest)
}

fn setup_composite_rootfs(manifest: &Manifest, base_dir: &str, repo_path: &str) -> io::Result<()> {
    println!("Setting up composite rootfs at {}", base_dir);
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

    for dep in &manifest.dependencies {
        checkout_ostree_dependency(repo_path, &dep.commit, base_dir)?;
    }

    Ok(())
}

fn handle_inputs(
    manifest: &Manifest,
    _manifest_dir: &Path,
    download_dir: &str,
    build_dir: &str,
    is_bootstrap: bool,
) -> io::Result<HashMap<String, String>> {
    println!("Handling inputs");

    let mut input_env_vars = HashMap::new();
    let current_dir = env::current_dir().expect("Failed to get current directory");

    for (i, source) in manifest.sources.iter().enumerate() {
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

fn run_build_script(
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

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let tmpdir_path = current_dir.join("build_rootfs").join("2nex").join("tmp");
    std::fs::create_dir_all(&tmpdir_path)?;

    let mut command_args = unshare_command.clone();
    let launch_script = if bootstrap {
        let bootstrap_sysroot_path = current_dir.join("build_rootfs").join("bootstrap");
        let bootstrap_tools_path = bootstrap_sysroot_path.join("tools");
        let bootstrap_tools_path_display = bootstrap_tools_path.display();
        let workdir_path = current_dir.join("build_rootfs").join("2nex").join("work");
        let outdir_path = current_dir.join("build_rootfs").join("2nex").join("out");

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
        match hostname::get() {
            Ok(current_hostname) => {
                if current_hostname.to_string_lossy() != "2nex-builder" {
                    // Add hostname-setting commands only if the hostname is not already "2nex-builder"
                    temp_file.write_all(b"echo 'Setting hostname to 2nex-builder'\n")?;
                    temp_file.write_all(b"hostname 2nex-builder\n")?;
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Couldn't get current hostname: {}. Forcing hostname.",
                    e
                );
                temp_file.write_all(b"echo 'Setting hostname to 2nex-builder'\n")?;
                temp_file.write_all(b"hostname 2nex-builder\n")?;
            }
        }
        temp_file.write_all(build_script.as_bytes())?;

        format!(
            r#"
            mkdir -p {build_dir}/dev
            for D in null zero random urandom tty console full; do
                touch {build_dir}/dev/$D
                mount --bind /dev/$D {build_dir}/dev/$D
            done

            mkdir -p {build_dir}/dev/pts
            mount -t devpts devpts {build_dir}/dev/pts
            ln -sf /dev/pts/ptmx {build_dir}/dev/ptmx

            if [ -e {build_dir}/bin ]; then
                rmdir {build_dir}/lib
            fi
            
            if [ -e {build_dir}/lib64 ]; then
                rmdir {build_dir}/lib64
            fi
            
            if [ -e {build_dir}/sbin ]; then
                rmdir {build_dir}/usr/sbin
            fi
            
            if [ -e {build_dir}/lib64 ]; then
                rmdir {build_dir}/usr/lib64
            fi

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

            chmod +x {build_dir}/2nex/tmp/build_script.sh
            unshare --root={build_dir} /2nex/tmp/build_script.sh
            "#,
            build_dir = build_dir
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

/// Verifies and commits outputs to OSTree branches based on the manifest.
///
/// This function takes the manifest, base directory, and repository path as input.
/// It verifies and commits the outputs specified in the manifest to the corresponding
/// OSTree branches. The function categorizes the output files, checks their existence,
/// moves them to the appropriate output directories, and commits them to the OSTree
/// repository.
fn verify_and_commit_outputs(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
) -> io::Result<()> {
    println!("Verifying and committing outputs to OSTree branches");

    let output_types = &manifest.outputs;
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

    for (output_type, files) in output_types {
        if output_type == "discard" {
            continue;
        }

        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, output_type
        );

        for file_path in files {
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
            let output_dir = out_dir
                .join(output_type)
                .join(target_dir_structure.strip_prefix(&out_dir).unwrap());
            fs::create_dir_all(&output_dir)?;

            fs::rename(
                &source_path,
                output_dir.join(source_path.file_name().unwrap()),
            )?;

            accounted_files.push(file_path.trim_start_matches('/').to_string());
        }

        let commit_output_dir = out_dir.join(output_type);
        commit_to_ostree(&commit_output_dir, &branch_name, repo_path)?;
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

fn create_and_commit_bundles(
    manifest: &Manifest,
    _base_dir: &str,
    repo_path: &str,
) -> io::Result<()> {
    println!("Processing bundles");

    let bundles = &manifest.bundles;

    for (bundle_name, includes) in bundles {
        commit_bundle(repo_path, bundle_name, includes, manifest)?;
    }

    Ok(())
}

fn checkout_ostree_dependency(
    repo_path: &str,
    commit_id: &str,
    target_dir: &str,
) -> io::Result<()> {
    println!(
        "Checking out OSTree dependency {} into {}",
        commit_id, target_dir
    );

    let checkout_command = &[
        "ostree", "checkout", "--repo", repo_path, "--union", commit_id, target_dir,
    ];

    let output = Command::new("unshare")
        .args(&["--user", "--map-root-user", "--"])
        .args(checkout_command)
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to checkout OSTree dependency: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(())
}

fn commit_to_ostree(output_dir: &Path, branch_name: &str, repo_path: &str) -> io::Result<()> {
    println!(
        "Committing {} to OSTree branch {}",
        output_dir.display(),
        branch_name
    );

    let commit_command = &[
        "ostree",
        "commit",
        "--repo",
        repo_path,
        "--branch",
        branch_name,
        "--no-xattrs",
        "--no-bindings",
        output_dir.to_str().unwrap(),
    ];

    let output = Command::new("unshare")
        .args(&["--map-root-user", "--user", "--"])
        .args(commit_command)
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to commit to OSTree: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(())
}

fn commit_bundle(
    repo_path: &str,
    bundle_name: &str,
    includes: &[String],
    manifest: &Manifest,
) -> io::Result<()> {
    println!("Creating bundle: {}", bundle_name);

    let temp_dir = TempDir::new()?;
    let temp_dir_path = temp_dir.path();

    for output in includes {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, output
        );
        checkout_ostree_dependency(repo_path, &branch_name, temp_dir_path.to_str().unwrap())?;
    }

    let bundle_branch = format!(
        "x86_64/{}/{}/{}/bundles/{}",
        manifest.package.slug, manifest.package.version, manifest.package.flavor, bundle_name
    );
    commit_to_ostree(temp_dir_path, &bundle_branch, repo_path)?;

    Ok(())
}

fn fetch_and_verify_input(input_spec: &Source, download_dir: &str) -> io::Result<PathBuf> {
    println!("Fetching and verifying input: {:?}", input_spec);

    if let Some(url) = &input_spec.url {
        println!("Fetching input from URL: {}", url);

        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let mut current_url = url.clone();
        let mut final_url = url.clone();
        let mut response = None;

        // Follow redirects manually
        for _ in 0..20 {
            // Limit to 20 redirects to prevent infinite loops
            println!("Trying URL: {}", current_url);
            let resp = client
                .get(&current_url)
                .send()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            let status = resp.status();
            println!("Response status: {}", status);

            if status.is_redirection() {
                if let Some(location) = resp.headers().get(LOCATION) {
                    let new_url = location
                        .to_str()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    println!("Redirecting to: {}", new_url);
                    current_url = if new_url.starts_with("http") {
                        new_url.to_string()
                    } else {
                        Url::parse(&current_url)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                            .join(new_url)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                            .to_string()
                    };
                    final_url = current_url.clone();
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Redirect without Location header",
                    ));
                }
            } else if status.is_success() {
                response = Some(resp);
                break;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Unexpected status code: {}", status),
                ));
            }
        }

        let mut response =
            response.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Too many redirects"))?;

        println!("Final URL after redirects: {}", final_url);

        let file_name = response
            .headers()
            .get(CONTENT_DISPOSITION)
            .and_then(|header| header.to_str().ok())
            .map(parse_content_disposition)
            .and_then(|disp| disp.filename().map(|(name, _)| name))
            .or_else(|| {
                response
                    .headers()
                    .get(CONTENT_DISPOSITION)
                    .and_then(|header| header.to_str().ok())
                    .map(parse_content_disposition)
                    .and_then(|disp| disp.filename_full())
            })
            .unwrap_or_else(|| {
                Url::parse(&final_url)
                    .ok()
                    .and_then(|url| {
                        url.path_segments()
                            .and_then(|segments| segments.last().map(String::from))
                    })
                    .unwrap_or_else(|| "downloaded_file".to_string())
            });

        let dst_path = Path::new(download_dir).join(file_name);

        if !dst_path.exists() {
            println!("Downloading {} from {}", dst_path.display(), final_url);
            let mut file = fs::File::create(&dst_path)?;
            io::copy(&mut response, &mut file)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        let mut file = fs::File::open(&dst_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let sha256_hash = hex::encode(Sha256::digest(&contents));
        if sha256_hash != input_spec.sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SHA256 hash mismatch for downloaded input: {}", sha256_hash),
            ));
        }

        Ok(dst_path)
    } else if let Some(file_path) = &input_spec.file {
        println!("Fetching input from local file: {}", file_path);

        let file_path = Path::new("./inputs_cache").join(file_path);
        if !file_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Local file not found: {}", file_path.display()),
            ));
        }

        let mut file = fs::File::open(&file_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let sha256_hash = hex::encode(Sha256::digest(&contents));
        if sha256_hash != input_spec.sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SHA256 hash mismatch for local input: {}", sha256_hash),
            ));
        }

        Ok(file_path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No URL or file path specified for input.",
        ))
    }
}

fn calculate_output_checksum(output_dir: &Path) -> io::Result<String> {
    let mut file_paths: Vec<PathBuf> = Vec::new();

    // Collect all file paths
    for entry in WalkDir::new(output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            file_paths.push(entry.path().to_path_buf());
        }
    }

    // Sort file paths to ensure consistent ordering
    file_paths.sort();

    let mut hasher = Sha256::new();

    for path in file_paths {
        // Update hasher with relative path
        let relative_path = path.strip_prefix(output_dir).unwrap();
        hasher.update(relative_path.to_string_lossy().as_bytes());
        hasher.update(b"\0"); // Use null byte as separator

        // Read and hash file contents
        let mut file = fs::File::open(&path)?;
        let mut buffer = [0; 4096];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.update(b"\0"); // Use null byte as separator between files
    }

    Ok(hex::encode(hasher.finalize()))
}

fn categorize_files(rootfs_dir: &Path) -> HashMap<String, Vec<String>> {
    let mut outputs = HashMap::new();

    for entry in WalkDir::new(rootfs_dir) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() || entry.file_type().is_symlink() {
            let relative_path = entry.path().strip_prefix(rootfs_dir).unwrap();
            let relative_path_str = format!("/{}", relative_path.to_str().unwrap());

            let category = determine_category(&relative_path_str);
            outputs
                .entry(category)
                .or_insert_with(Vec::new)
                .push(relative_path_str);
        }
    }

    // Sort each vector of paths in the HashMap
    for paths in outputs.values_mut() {
        paths.sort();
    }

    outputs
}

fn determine_category(file_path: &str) -> String {
    if file_path.ends_with(".so") || file_path.contains(".so.") {
        "lib".to_string()
    } else if file_path.contains("/include/") {
        "dev".to_string()
    } else if file_path.ends_with(".pc") || file_path.contains("/pkgconfig/") {
        "dev".to_string()
    } else if file_path.ends_with(".la") {
        "dev".to_string()
    } else if file_path.ends_with(".a") {
        "static".to_string()
    } else if file_path.contains("/share/man/") {
        "man".to_string()
    } else if file_path.contains("/share/info/") {
        "info".to_string()
    } else if file_path.contains("/share/doc") {
        "doc".to_string()
    } else if file_path.contains("/locale/") {
        "locale".to_string()
    } else if file_path.contains("/bin/") {
        "bin".to_string()
    } else if file_path.contains("/libexec/") {
        "bin".to_string()
    } else if file_path.contains("/lib/") || file_path.contains("/lib64/") {
        "lib".to_string()
    } else if file_path.contains("/conf/")
        || file_path.contains("/etc/")
        || file_path.ends_with(".conf")
    {
        "conf".to_string()
    } else {
        "misc".to_string()
    }
}

fn print_outputs(outputs: &HashMap<String, Vec<String>>) {
    println!("outputs:");
    for (category, files) in outputs {
        println!("  {}:", category);
        for file in files {
            println!("    - {}", file);
        }
    }
}
