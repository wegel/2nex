use std::collections::{hash_map::Entry, BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use goblin::Object;
use hostname;
use num_cpus;
use serde::de::Deserializer;
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
    #[clap(long, help = "Skip runtime dependency scanning")]
    skip_runtime_deps: bool,
    #[clap(
        long,
        help = "Include per-reference explanations in runtime dependency output"
    )]
    runtime_deps_verbose: bool,
    #[clap(
        long,
        help = "Treat missing files during runtime dependency scanning as warnings instead of errors"
    )]
    allow_missing_runtime_files: bool,
}

#[derive(Deserialize)]
struct Manifest {
    package: Package,
    dependencies: Vec<Dependency>,
    sources: Vec<Source>,
    build: Build,
    #[serde(deserialize_with = "deserialize_outputs")]
    outputs: HashMap<String, OutputSpec>,
    #[serde(deserialize_with = "deserialize_bundles")]
    bundles: HashMap<String, Bundle>,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Bundle {
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    suggests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BundleDef {
    Simple(Vec<String>),
    Detailed(Bundle),
}

impl From<BundleDef> for Bundle {
    fn from(def: BundleDef) -> Self {
        match def {
            BundleDef::Simple(includes) => Bundle {
                includes,
                requires: Vec::new(),
                suggests: Vec::new(),
            },
            BundleDef::Detailed(bundle) => bundle,
        }
    }
}

fn deserialize_bundles<'de, D>(deserializer: D) -> Result<HashMap<String, Bundle>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, BundleDef> = HashMap::deserialize(deserializer)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.into())).collect())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct OutputSpec {
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    suggests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OutputDef {
    Simple(Vec<String>),
    Detailed(OutputSpec),
}

impl From<OutputDef> for OutputSpec {
    fn from(def: OutputDef) -> Self {
        match def {
            OutputDef::Simple(files) => OutputSpec {
                files,
                requires: Vec::new(),
                suggests: Vec::new(),
            },
            OutputDef::Detailed(spec) => spec,
        }
    }
}

fn deserialize_outputs<'de, D>(deserializer: D) -> Result<HashMap<String, OutputSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, OutputDef> = HashMap::deserialize(deserializer)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.into())).collect())
}

fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();

    let manifest = load_manifest(&opts.manifest_file)?;
    let manifest_dir = Path::new(&opts.manifest_file).parent().unwrap();
    let base_dir = "./build_rootfs";
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    let dependency_commits = resolve_dependency_closure(&manifest.dependencies, &opts.repo_path)?;

    setup_composite_rootfs(&manifest, base_dir, &opts.repo_path, &dependency_commits)?;
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

    if !opts.skip_runtime_deps {
        scan_runtime_dependencies(
            &manifest,
            base_dir,
            &opts.repo_path,
            &dependency_commits,
            opts.runtime_deps_verbose,
            opts.allow_missing_runtime_files,
        )?;
    }

    let runtime_suggestions = if opts.skip_runtime_deps {
        None
    } else {
        Some(scan_runtime_dependencies(
            &manifest,
            base_dir,
            &opts.repo_path,
            &dependency_commits,
            opts.runtime_deps_verbose,
            opts.allow_missing_runtime_files,
        )?)
    };

    verify_and_commit_outputs(
        &manifest,
        base_dir,
        &opts.repo_path,
        runtime_suggestions.as_ref(),
        opts.runtime_deps_verbose,
    )?;

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
        setup_composite_rootfs(&manifest, base_dir, &opts.repo_path, &dependency_commits)?;
        handle_inputs(
            &manifest,
            manifest_dir,
            download_dir,
            base_dir,
            opts.bootstrap,
        )?;
        run_build_script(build_script, base_dir, &env_vars, opts.bootstrap)?;
        verify_and_commit_outputs(
            &manifest,
            base_dir,
            &opts.repo_path,
            runtime_suggestions.as_ref(),
            opts.runtime_deps_verbose,
        )?;
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

fn resolve_dependency_closure(
    dependencies: &[Dependency],
    repo_path: &str,
) -> io::Result<Vec<String>> {
    let mut resolved = Vec::new();
    let mut queue = VecDeque::new();
    for dep in dependencies {
        queue.push_back(dep.commit.clone());
    }

    let mut seen = HashSet::new();
    while let Some(commit) = queue.pop_front() {
        if !seen.insert(commit.clone()) {
            continue;
        }
        resolved.push(commit.clone());

        for key in ["nex.bundle.requires", "nex.output.requires"] {
            let required = read_metadata_list(repo_path, &commit, key)?;
            for req in required {
                if !req.is_empty() {
                    queue.push_back(req);
                }
            }
        }
    }

    Ok(resolved)
}

fn setup_composite_rootfs(
    _manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    dependency_commits: &[String],
) -> io::Result<()> {
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

    for commit in dependency_commits {
        checkout_ostree_into(repo_path, commit, Path::new(base_dir), true)?;
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
    runtime_suggestions: Option<&RuntimeScanResult>,
    verbose_reasons: bool,
) -> io::Result<()> {
    println!("Verifying and committing outputs to OSTree branches");

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
            manifest.package.slug, manifest.package.version, manifest.package.flavor, output_type
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

        let mut metadata = Vec::new();
        if let Some(checksum) = &manifest.package.checksum {
            metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
        }
        if let Some(encoded) = encode_metadata_list(&spec.requires)? {
            metadata.push(("nex.output.requires".to_string(), encoded));
        }
        if let Some(encoded) = encode_metadata_list(&spec.suggests)? {
            metadata.push(("nex.output.suggests".to_string(), encoded));
        }
        commit_to_ostree(&commit_output_dir, &branch_name, repo_path, &metadata)?;
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

    for (bundle_name, bundle) in bundles {
        commit_bundle(repo_path, bundle_name, bundle, manifest)?;
    }

    Ok(())
}

fn checkout_ostree_into(
    repo_path: &str,
    commit_id: &str,
    target_dir: &Path,
    union: bool,
) -> io::Result<()> {
    println!(
        "Checking out OSTree commit {} into {} (union: {})",
        commit_id,
        target_dir.display(),
        union
    );

    let mut command = Command::new("unshare");
    command.args(&["--user", "--map-root-user", "--"]);
    command.arg("ostree");
    command.arg("checkout");
    command.arg("--repo");
    command.arg(repo_path);
    if union {
        command.arg("--union");
    }
    command.arg(commit_id);
    command.arg(target_dir.to_str().unwrap());

    let output = command.output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to checkout OSTree commit: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(())
}

fn commit_to_ostree(
    output_dir: &Path,
    branch_name: &str,
    repo_path: &str,
    metadata: &[(String, String)],
) -> io::Result<()> {
    println!(
        "Committing {} to OSTree branch {}",
        output_dir.display(),
        branch_name
    );

    let mut command = Command::new("unshare");
    command.args(&["--map-root-user", "--user", "--"]);

    command.arg("ostree");
    command.arg("commit");
    command.arg("--repo").arg(repo_path);
    command.arg("--branch").arg(branch_name);
    command.arg("--no-xattrs");
    command.arg("--no-bindings");

    for (key, value) in metadata {
        let metadata_arg = format!("{}={}", key, value);
        command.arg("--add-metadata-string");
        command.arg(metadata_arg);
    }

    command.arg(output_dir.to_str().unwrap());

    let output = command.output()?;

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

fn encode_metadata_list(values: &[String]) -> io::Result<Option<String>> {
    if values.is_empty() {
        Ok(None)
    } else {
        serde_json::to_string(values)
            .map(Some)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

fn read_metadata_list(repo_path: &str, commit: &str, key: &str) -> io::Result<Vec<String>> {
    let mut command = Command::new("unshare");
    command.args(&["--user", "--map-root-user", "--"]);
    command.arg("ostree");
    command.arg("show");
    command.arg("--repo");
    command.arg(repo_path);
    command.arg(format!("--print-metadata-key={}", key));
    command.arg(commit);

    let output = command.output()?;
    if output.status.success() {
        parse_metadata_list_output(&output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No such metadata key") {
            Ok(Vec::new())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to read metadata {} from {}: {}",
                    key, commit, stderr
                ),
            ))
        }
    }
}

fn parse_metadata_list_output(raw: &[u8]) -> io::Result<Vec<String>> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let value = String::from_utf8(raw.to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(Vec::new())
    } else {
        serde_json::from_str(trimmed).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

fn commit_bundle(
    repo_path: &str,
    bundle_name: &str,
    bundle: &Bundle,
    manifest: &Manifest,
) -> io::Result<()> {
    println!("Creating bundle: {}", bundle_name);

    let temp_dir = TempDir::new()?;
    let temp_dir_path = temp_dir.path();

    for output in &bundle.includes {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, output
        );
        checkout_ostree_into(repo_path, &branch_name, temp_dir_path, true)?;
    }

    let bundle_branch = format!(
        "x86_64/{}/{}/{}/bundles/{}",
        manifest.package.slug, manifest.package.version, manifest.package.flavor, bundle_name
    );

    let mut metadata = Vec::new();
    if let Some(checksum) = &manifest.package.checksum {
        metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
    }
    if let Some(encoded) = encode_metadata_list(&bundle.requires)? {
        metadata.push(("nex.bundle.requires".to_string(), encoded));
    }
    if let Some(encoded) = encode_metadata_list(&bundle.suggests)? {
        metadata.push(("nex.bundle.suggests".to_string(), encoded));
    }

    commit_to_ostree(temp_dir_path, &bundle_branch, repo_path, &metadata)?;

    Ok(())
}

fn fetch_and_verify_input(input_spec: &Source, download_dir: &str) -> io::Result<PathBuf> {
    println!("Fetching and verifying input: {:?}", input_spec);

    // First check if we have a symbolic link with the hash name
    let hash_link_path = Path::new(download_dir).join(format!("sha256-{}", input_spec.sha256));
    if hash_link_path.exists() {
        // If the symbolic link exists, check that the target file also exists
        if let Ok(target_filename) = std::fs::read_link(&hash_link_path) {
            // Handle the relative path properly - the symlink points to a file in the same directory
            let full_target_path = Path::new(download_dir).join(&target_filename);
            if full_target_path.exists() {
                println!(
                    "Found existing file via hash link: {} -> {}",
                    hash_link_path.display(),
                    full_target_path.display()
                );

                // Always verify the hash even if found via symlink
                let mut file = fs::File::open(&full_target_path)?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;

                let calculated_hash = hex::encode(Sha256::digest(&contents));
                if calculated_hash == input_spec.sha256 {
                    println!("Hash verified for file found via symlink");
                    return Ok(full_target_path);
                } else {
                    println!(
                        "Hash mismatch for file found via symlink. Expected: {}, Got: {}",
                        input_spec.sha256, calculated_hash
                    );
                    println!("Removing invalid symlink: {}", hash_link_path.display());
                    std::fs::remove_file(&hash_link_path)?;
                    // Continue with normal download/verification process
                }
            } else {
                println!(
                    "Hash link target doesn't exist, removing stale link: {}",
                    hash_link_path.display()
                );
                std::fs::remove_file(&hash_link_path)?;
            }
        }
    }

    if let Some(url) = &input_spec.url {
        println!("Fetching input from URL: {}", url);

        // Extract a reasonable filename from the URL
        let url_path = url.split('/').last().unwrap_or("downloaded_file");
        let expected_filename = url_path.split('?').next().unwrap_or(url_path);
        let dst_path = Path::new(download_dir).join(expected_filename);

        // Download the file using curl
        if !dst_path.exists() {
            println!("Downloading {} using curl", dst_path.display());

            let status = std::process::Command::new("curl")
                .args([
                    "-L", // Follow redirects
                    "-f", // Fail on server errors
                    "-s", // Silent mode
                    "--output",
                    dst_path.to_str().unwrap(),
                    url,
                ])
                .status()?;

            if !status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("curl download failed with status: {}", status),
                ));
            }
        } else {
            println!("File already exists: {}", dst_path.display());
        }

        // Verify the downloaded file
        let mut file = fs::File::open(&dst_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        // Check hash before creating the symlink
        let sha256_hash = hex::encode(Sha256::digest(&contents));
        if sha256_hash != input_spec.sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SHA256 hash mismatch for downloaded input: {}", sha256_hash),
            ));
        }

        // Create a symbolic link from the hash to the file
        let hash_link_path = Path::new(download_dir).join(format!("sha256-{}", input_spec.sha256));
        if !hash_link_path.exists() {
            // Create relative path for the symlink to avoid including inputs_cache itself
            let filename = dst_path.file_name().unwrap();
            println!(
                "Creating hash symbolic link: {} -> {}",
                hash_link_path.display(),
                filename.to_string_lossy()
            );
            std::os::unix::fs::symlink(&filename, &hash_link_path)?;
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

        // Create a symbolic link from the hash to the file
        let hash_link_path = Path::new(download_dir).join(format!("sha256-{}", input_spec.sha256));
        if !hash_link_path.exists() {
            // Create relative path for the symlink to avoid including inputs_cache itself
            let filename = file_path.file_name().unwrap();
            println!(
                "Creating hash symbolic link: {} -> {}",
                hash_link_path.display(),
                filename.to_string_lossy()
            );
            std::os::unix::fs::symlink(&filename, &hash_link_path)?;
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

fn print_outputs(
    outputs: &HashMap<String, Vec<String>>,
    runtime_suggestions: Option<&RuntimeScanResult>,
    verbose_reasons: bool,
) {
    println!("outputs:");
    let mut categories: Vec<_> = outputs.keys().collect();
    categories.sort();
    for category in categories {
        let files = outputs.get(category).unwrap();
        println!("  {}:", category);
        println!("    files:");
        for file in files {
            println!("      - {}", file);
        }
        if let Some(suggestions) = runtime_suggestions {
            if let Some(commits) = suggestions.category_resolved(category) {
                println!("    requires:");
                for (commit, reasons) in commits {
                    println!("      - {}", commit);
                    if verbose_reasons {
                        for reason in reasons {
                            println!("        # {}", reason);
                        }
                    }
                }
            }
            if let Some(unresolved) = suggestions.category_unresolved(category) {
                println!("    unresolved:");
                for (req, reasons) in unresolved {
                    println!("      - {}", req);
                    if verbose_reasons {
                        for reason in reasons {
                            println!("        # {}", reason);
                        }
                    }
                }
            }
        }
    }
}

fn scan_runtime_dependencies(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    dependency_commits: &[String],
    verbose_reasons: bool,
    allow_missing_files: bool,
) -> io::Result<RuntimeScanResult> {
    let base_dir_path = Path::new(base_dir);
    let out_dir = base_dir_path.join("2nex/out");

    if !out_dir.exists() {
        println!(
            "No output directory found at {}. Skipping runtime dependency scan.",
            out_dir.display()
        );
        return Ok(RuntimeScanResult::default());
    }

    println!(
        "Scanning runtime dependencies for {} {}",
        manifest.package.name, manifest.package.version
    );

    let local_basenames = collect_local_basenames(&out_dir)?;
    let provider_index = build_provider_index(repo_path, dependency_commits)?;
    let mut result = RuntimeScanResult::default();

    for entry in WalkDir::new(&out_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(&out_dir)
            .unwrap_or(entry.path())
            .to_string_lossy();
        let display_path = format!("/{}", rel);

        scan_file_for_dependencies(
            entry.path(),
            &display_path,
            &out_dir,
            &local_basenames,
            &provider_index,
            &mut result,
            allow_missing_files,
        )?;
    }

    if verbose_reasons && result.resolved.is_empty() {
        println!(
            "Runtime dependency suggestions for {} {}",
            manifest.package.name, manifest.package.version
        );
        println!("  No external runtime dependencies detected.");
    }

    Ok(result)
}

fn collect_local_basenames(out_dir: &Path) -> io::Result<HashSet<String>> {
    let mut names = HashSet::new();
    for entry in WalkDir::new(out_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
            continue;
        }

        if let Some(name) = entry.file_name().to_str() {
            names.insert(name.to_string());
        }
    }
    Ok(names)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProviderMatch {
    commit: String,
    path: String,
}

#[derive(Default)]
struct ProviderIndex {
    by_basename: HashMap<String, Vec<ProviderMatch>>,
    by_full_path: HashMap<String, Vec<ProviderMatch>>,
}

impl ProviderIndex {
    fn add_entry(&mut self, commit: &str, path: String) {
        let provider = ProviderMatch {
            commit: commit.to_string(),
            path: path.clone(),
        };
        if let Some(name) = Path::new(&path).file_name().and_then(|n| n.to_str()) {
            self.by_basename
                .entry(name.to_string())
                .or_default()
                .push(provider.clone());
        }
        self.by_full_path.entry(path).or_default().push(provider);
    }
}

fn build_provider_index(repo_path: &str, dependencies: &[String]) -> io::Result<ProviderIndex> {
    let mut index = ProviderIndex::default();
    let mut outputs_cache: HashMap<String, HashSet<String>> = HashMap::new();

    for dep in dependencies {
        let prefix = manifest_prefix(dep).unwrap_or_else(|| dep.clone());
        let outputs = match outputs_cache.entry(prefix.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let set = list_output_refs(repo_path, &prefix)?;
                entry.insert(set)
            }
        };

        let mut command = Command::new("unshare");
        command.args(&["--user", "--map-root-user", "--"]);
        command.arg("ostree");
        command.arg("ls");
        command.arg("--repo");
        command.arg(repo_path);
        command.arg("--recursive");
        command.arg(dep);

        let output = command.output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to list OSTree commit {}: {}",
                    dep,
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        let listing = String::from_utf8(output.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        for line in listing.lines() {
            if line.is_empty() {
                continue;
            }
            let entry_type = line.chars().next().unwrap_or(' ');
            if entry_type != '-' && entry_type != 'l' {
                continue;
            }
            let path = match line.find(" /") {
                Some(idx) => {
                    let raw = &line[idx + 1..];
                    raw.split(" -> ").next().unwrap_or(raw).to_string()
                }
                None => continue,
            };
            let category = determine_category(&path);
            let canonical_branch = {
                let branch = format!("{}/outputs/{}", prefix, category);
                if outputs.contains(&branch) {
                    branch
                } else {
                    dep.clone()
                }
            };
            index.add_entry(&canonical_branch, path);
        }
    }

    Ok(index)
}

fn manifest_prefix(commit: &str) -> Option<String> {
    if let Some(idx) = commit.find("/bundles/") {
        Some(commit[..idx].to_string())
    } else if let Some(idx) = commit.find("/outputs/") {
        Some(commit[..idx].to_string())
    } else {
        None
    }
}

fn list_output_refs(repo_path: &str, prefix: &str) -> io::Result<HashSet<String>> {
    let search_prefix = format!("{}/outputs", prefix);
    let mut command = Command::new("unshare");
    command.args(&["--user", "--map-root-user", "--"]);
    command.arg("ostree");
    command.arg("refs");
    command.arg("--repo");
    command.arg(repo_path);
    command.arg("--list");
    command.arg(&search_prefix);

    let output = command.output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to list output refs for {}: {}",
                prefix,
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut refs = HashSet::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            refs.insert(trimmed.to_string());
        }
    }

    Ok(refs)
}

fn scan_file_for_dependencies(
    path: &Path,
    display_path: &str,
    out_dir: &Path,
    local_basenames: &HashSet<String>,
    providers: &ProviderIndex,
    result: &mut RuntimeScanResult,
    allow_missing_files: bool,
) -> io::Result<()> {
    if path.is_dir() {
        return Ok(());
    }
    let category = determine_category(display_path);
    if let Some(elf) = read_elf_metadata(path, allow_missing_files)? {
        for needed in elf.needed {
            if needed.is_empty() || local_basenames.contains(&needed) {
                continue;
            }
            let reason = format!("{} needs {}", display_path, needed);
            let matches = resolve_requirement(providers, &needed);
            if matches.is_empty() {
                result.add_unresolved(&category, needed, reason);
            } else {
                for candidate in matches {
                    result.add_resolved(&category, &candidate.commit, reason.clone());
                }
            }
        }

        if let Some(interpreter) = elf.interpreter {
            if !interpreter.is_empty() {
                let reason = format!("{} uses interpreter {}", display_path, interpreter);
                let matches = resolve_requirement(providers, &interpreter);
                if matches.is_empty() {
                    result.add_unresolved(&category, interpreter, reason);
                } else {
                    for candidate in matches {
                        result.add_resolved(&category, &candidate.commit, reason.clone());
                    }
                }
            }
        }
    }

    if let Some(shebang) = parse_shebang_info(path, allow_missing_files)? {
        handle_shebang_requirement(
            &shebang.interpreter,
            display_path,
            out_dir,
            local_basenames,
            providers,
            result,
            &category,
            allow_missing_files,
        );

        if interpreter_is_env(&shebang.interpreter) {
            if let Some(target) = shebang.args.first() {
                handle_shebang_requirement(
                    target,
                    display_path,
                    out_dir,
                    local_basenames,
                    providers,
                    result,
                    &category,
                    allow_missing_files,
                );
            }
        }
    }

    Ok(())
}

fn resolve_requirement(providers: &ProviderIndex, reference: &str) -> Vec<ProviderMatch> {
    let trimmed = reference.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Some(matches) = providers.by_full_path.get(trimmed) {
        return matches.clone();
    }

    if let Some(basename) = Path::new(trimmed).file_name().and_then(|n| n.to_str()) {
        if let Some(matches) = providers.by_basename.get(basename) {
            return matches.clone();
        }
    }

    Vec::new()
}

fn handle_shebang_requirement(
    target: &str,
    display_path: &str,
    out_dir: &Path,
    local_basenames: &HashSet<String>,
    providers: &ProviderIndex,
    result: &mut RuntimeScanResult,
    category: &str,
    _allow_missing_files: bool,
) {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return;
    }

    if let Some(basename) = Path::new(trimmed).file_name().and_then(|n| n.to_str()) {
        if local_basenames.contains(basename) {
            return;
        }
    }

    if trimmed.starts_with('/') {
        let rel_path = trimmed.trim_start_matches('/');
        let candidate_path = out_dir.join(rel_path);
        if candidate_path.exists() {
            return;
        }
    }

    let reason = format!("{} shebang references {}", display_path, trimmed);
    let matches = resolve_requirement(providers, trimmed);
    if matches.is_empty() {
        result.add_unresolved(category, trimmed.to_string(), reason);
    } else {
        for candidate in matches {
            result.add_resolved(category, &candidate.commit, reason.clone());
        }
    }
}

fn interpreter_is_env(interpreter: &str) -> bool {
    Path::new(interpreter)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|name| name == "env")
        .unwrap_or(false)
}

#[derive(Default)]
struct RuntimeScanResult {
    resolved: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    unresolved: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
}

impl RuntimeScanResult {
    fn add_resolved(&mut self, category: &str, commit: &str, reason: String) {
        self.resolved
            .entry(category.to_string())
            .or_default()
            .entry(commit.to_string())
            .or_default()
            .insert(reason);
    }

    fn add_unresolved(&mut self, category: &str, requirement: String, reason: String) {
        self.unresolved
            .entry(category.to_string())
            .or_default()
            .entry(requirement)
            .or_default()
            .insert(reason);
    }

    fn category_resolved(&self, category: &str) -> Option<&BTreeMap<String, BTreeSet<String>>> {
        self.resolved.get(category)
    }

    fn category_unresolved(&self, category: &str) -> Option<&BTreeMap<String, BTreeSet<String>>> {
        self.unresolved.get(category)
    }
}

struct ElfMetadata {
    needed: Vec<String>,
    interpreter: Option<String>,
}

fn read_elf_metadata(path: &Path, allow_missing: bool) -> io::Result<Option<ElfMetadata>> {
    let data = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if handle_missing_path(path, allow_missing) {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Missing file during runtime dependency scan: {}",
                    path.display()
                ),
            ));
        }
        Err(e) => return Err(e),
    };

    if data.len() < 4 || &data[..4] != b"\x7FELF" {
        return Ok(None);
    }

    match Object::parse(&data) {
        Ok(Object::Elf(elf)) => {
            let needed = elf
                .libraries
                .iter()
                .map(|lib| lib.trim().to_string())
                .filter(|lib| !lib.is_empty())
                .collect();
            let interpreter = elf
                .interpreter
                .map(|interp| interp.trim().to_string())
                .filter(|interp| !interp.is_empty());

            Ok(Some(ElfMetadata {
                needed,
                interpreter,
            }))
        }
        _ => Ok(None),
    }
}

struct ShebangInfo {
    interpreter: String,
    args: Vec<String>,
}

fn parse_shebang_info(path: &Path, allow_missing: bool) -> io::Result<Option<ShebangInfo>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if matches!(e.kind(), io::ErrorKind::PermissionDenied) => return Ok(None),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if handle_missing_path(path, allow_missing) {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Missing file during runtime dependency scan: {}",
                    path.display()
                ),
            ));
        }
        Err(e) => return Err(e),
    };

    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let bytes_read = reader.read_until(b'\n', &mut buffer)?;

    if bytes_read < 2 || !buffer.starts_with(b"#!") {
        return Ok(None);
    }

    let line = String::from_utf8_lossy(&buffer[2..]);
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut parts = trimmed.split_whitespace();
    if let Some(interpreter) = parts.next() {
        let args = parts.map(|s| s.to_string()).collect();
        return Ok(Some(ShebangInfo {
            interpreter: interpreter.to_string(),
            args,
        }));
    }

    Ok(None)
}

fn handle_missing_path(path: &Path, allow_missing: bool) -> bool {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            println!(
                "Skipping dangling symlink during runtime scan: {}",
                path.display()
            );
            return true;
        }
    }
    allow_missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn base_manifest() -> &'static str {
        r#"
package:
  schema: 1
  name: sample
  slug: sample
  flavor: bootstrap/phase0
  version: "1.0"
dependencies: []
sources: []
build:
  script: "true"
outputs: {}
"#
    }

    #[test]
    fn parses_detailed_bundle_with_metadata() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    includes:\n      - bin\n      - lib\n    requires:\n      - x86_64/foo/1.0/outputs/lib\n    suggests:\n      - x86_64/bar/2.0/bundles/dev\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
        assert_eq!(
            bundle.requires,
            vec!["x86_64/foo/1.0/outputs/lib".to_string()]
        );
        assert_eq!(
            bundle.suggests,
            vec!["x86_64/bar/2.0/bundles/dev".to_string()]
        );
    }

    #[test]
    fn parses_detailed_output_with_metadata() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen(
                "outputs: {}",
                "outputs:\n  bin:\n    files:\n      - /usr/bin/foo\n    requires:\n      - x86_64/libfoo/1.0/outputs/lib\n    suggests:\n      - x86_64/foo-doc/1.0/outputs/doc",
                1,
            )
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files, vec!["/usr/bin/foo".to_string()]);
        assert_eq!(
            output.requires,
            vec!["x86_64/libfoo/1.0/outputs/lib".to_string()]
        );
        assert_eq!(
            output.suggests,
            vec!["x86_64/foo-doc/1.0/outputs/doc".to_string()]
        );
    }

    #[test]
    fn parses_legacy_output_format() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen("outputs: {}", "outputs:\n  bin:\n    - /usr/bin/foo", 1)
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files, vec!["/usr/bin/foo".to_string()]);
        assert!(output.requires.is_empty());
        assert!(output.suggests.is_empty());
    }

    #[test]
    fn parses_legacy_bundle_format() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    - bin\n    - lib\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
        assert!(bundle.requires.is_empty());
        assert!(bundle.suggests.is_empty());
    }

    #[test]
    fn parse_shebang_extracts_interpreter_and_args() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("script.sh");
        fs::write(
            &script_path,
            b"#!/usr/bin/env python3 -OO\nprint('hello world')\n",
        )
        .unwrap();

        let info = parse_shebang_info(&script_path, false)
            .expect("parse shebang")
            .expect("expected shebang info");
        assert_eq!(info.interpreter, "/usr/bin/env");
        assert_eq!(info.args, vec!["python3".to_string(), "-OO".to_string()]);
    }

    #[test]
    fn provider_index_resolves_full_and_basename_matches() {
        let mut index = ProviderIndex::default();
        index.add_entry(
            "x86_64/python/3.12/base/bundles/dev",
            "/usr/bin/python3".to_string(),
        );
        let basename_matches = resolve_requirement(&index, "python3");
        assert_eq!(basename_matches.len(), 1);
        assert_eq!(
            basename_matches[0].commit,
            "x86_64/python/3.12/base/bundles/dev"
        );
        assert_eq!(basename_matches[0].path, "/usr/bin/python3");

        let path_matches = resolve_requirement(&index, "/usr/bin/python3");
        assert_eq!(path_matches.len(), 1);
        assert_eq!(
            path_matches[0].commit,
            "x86_64/python/3.12/base/bundles/dev"
        );
    }

    #[test]
    fn manifest_prefix_extracts_base_path() {
        assert_eq!(
            manifest_prefix("x86_64/foo/1.0/base/bundles/dev"),
            Some("x86_64/foo/1.0/base".to_string())
        );
        assert_eq!(
            manifest_prefix("x86_64/foo/1.0/base/outputs/lib"),
            Some("x86_64/foo/1.0/base".to_string())
        );
        assert_eq!(manifest_prefix("x86_64/foo/1.0/base"), None);
    }
}
