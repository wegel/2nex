use std::collections::HashMap;
use std::fs::{self, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::process;

use walkdir::WalkDir;

use crate::build::*;
use crate::deps::*;
use crate::manifest::types::BuildPaths;
use crate::manifest::*;
use crate::materializer::resolver::resolve_runtime_deps_precomputed;
use crate::materializer::types::MaterializeRequest;
use crate::materializer::{checkout_files, flatten_capsule_precomputed};
use crate::outputs::calculate_output_checksum;
use crate::store::{
    checkout_into, commit_tree, encode_metadata_list, export_path, get_commit_id,
    get_commit_metadata,
};
use crate::BuildOpts;

pub fn build_system_manifest(opts: &BuildOpts, manifest: &SystemManifest) -> io::Result<()> {
    build_system_manifest_with_dir(opts, manifest, ".nex/tmp/build_rootfs")
}

pub fn build_system_manifest_with_dir(
    opts: &BuildOpts,
    manifest: &SystemManifest,
    base_dir: &str,
) -> io::Result<()> {
    if opts.runtime_deps_verbose || opts.compute_deps {
        println!("Note: --compute-deps is not available for system manifests (deps come from package manifests).");
    }

    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    // load manifest index for dependency resolution
    let manifest_index = ManifestIndex::load("pkg")?;

    let dependency_commits = resolve_dependency_closure(&manifest.dependencies, &manifest_index)?;
    let package_dependency_specs = dependencies_from_system_packages(&manifest.packages);
    let package_commits = resolve_dependency_closure(&package_dependency_specs, &manifest_index)?;

    // load build environment from git blob (needed for paths)
    let build_env = load_environment(&opts.repo_path, &manifest.build.environment)?;

    setup_composite_rootfs(
        base_dir,
        &opts.repo_path,
        &opts.fallback_repos,
        &dependency_commits,
        &build_env.paths,
    )?;
    layer_commits_into_rootfs(
        base_dir,
        &opts.repo_path,
        &opts.fallback_repos,
        &package_commits,
    )?;

    // get original package commits (not expanded) for materialize
    let original_package_commits: Vec<String> =
        manifest.packages.iter().map(|p| p.commit.clone()).collect();

    if manifest.system.nex_structure {
        materialize_nex_structure(base_dir, &opts.repo_path, &original_package_commits)?;
    } else {
        materialize_system_packages(
            base_dir,
            &opts.repo_path,
            &original_package_commits,
            &manifest_index,
        )?;
    }

    apply_overlays(&manifest.overlays, base_dir)?;

    let use_absolute_paths = !build_env.execution.chroot;

    let env_vars = build_system_env_vars(
        manifest,
        download_dir,
        base_dir,
        use_absolute_paths,
        &build_env.paths,
    )?;

    println!(
        "Building system {} {}",
        manifest.system.slug, manifest.system.version
    );

    run_build_script_with_env(
        &manifest.build.script,
        base_dir,
        &env_vars,
        &build_env,
        None,
    )?;

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

    if opts.check {
        println!("Validating build reproducibility by building the system a second time.");
        fs::remove_dir_all(base_dir)?;

        setup_composite_rootfs(
            base_dir,
            &opts.repo_path,
            &opts.fallback_repos,
            &dependency_commits,
            &build_env.paths,
        )?;
        layer_commits_into_rootfs(
            base_dir,
            &opts.repo_path,
            &opts.fallback_repos,
            &package_commits,
        )?;

        if manifest.system.nex_structure {
            materialize_nex_structure(base_dir, &opts.repo_path, &package_commits)?;
        } else {
            materialize_system_packages(
                base_dir,
                &opts.repo_path,
                &original_package_commits,
                &manifest_index,
            )?;
        }

        apply_overlays(&manifest.overlays, base_dir)?;

        let env_vars = build_system_env_vars(
            manifest,
            download_dir,
            base_dir,
            use_absolute_paths,
            &build_env.paths,
        )?;
        run_build_script_with_env(
            &manifest.build.script,
            base_dir,
            &env_vars,
            &build_env,
            None,
        )?;

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
    use_absolute_paths: bool,
    paths: &BuildPaths,
) -> io::Result<HashMap<String, String>> {
    // system builds are always chroot, so no canonical prefix needed
    let mut env_vars = handle_inputs(
        &manifest.sources,
        download_dir,
        base_dir,
        use_absolute_paths,
        paths,
        None,
    )?;
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
    manifest_index: &ManifestIndex,
) -> io::Result<()> {
    let target_dir = Path::new(base_dir).join("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    // use the materializer's resolver to get file-level deps
    let requests: Vec<MaterializeRequest> = package_commits
        .iter()
        .map(|c| MaterializeRequest::Output { commit: c.clone() })
        .collect();

    let closure = resolve_runtime_deps_precomputed(repo_path, &requests, manifest_index, &[])?;

    // error if there are unresolved dependencies (missing /files commits)
    if closure.has_unresolved() {
        let mut msg = String::from("Unresolved runtime dependencies:\n");
        for (dep, reasons) in &closure.unresolved {
            msg.push_str(&format!(
                "  {} - run 'nex compute-deps' on the package\n",
                dep
            ));
            for reason in reasons {
                msg.push_str(&format!("    needed by: {}\n", reason));
            }
        }
        return Err(io::Error::new(io::ErrorKind::NotFound, msg));
    }

    // checkout each commit, using file-level for transitive deps
    for commit in closure.all_commits() {
        if let Some(files) = closure.get_files(commit) {
            // file-level checkout: only extract specific files from {checksum}/files commit
            let files_vec: Vec<String> = files.iter().cloned().collect();
            println!("  Checking out {} file(s) from {}", files_vec.len(), commit);
            checkout_files(repo_path, commit, &files_vec, &target_dir, &[])?;
        } else {
            // full checkout for root commits
            checkout_into(repo_path, commit, &target_dir, true, false)?;
        }
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

    commit_tree(repo_path, &branch_name, &target_dir, &metadata)
}

/// Materialize packages using the /nex/pkg/ structure with deploy bundles.
///
/// For each package:
/// 1. Find or create its deploy bundle
/// 2. Checkout to /nex/pkg/<ns>/<slug>/<version>/<checksum>/
/// 3. Create symlinks from /usr/bin/<binary> to the deploy bundle
/// 4. Install nex-ld-shim at /lib64/ld-linux-x86-64.so.2
pub fn materialize_nex_structure(
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
) -> io::Result<()> {
    let target_dir = Path::new(base_dir).join("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    // create nex directory structure
    let nex_pkg_dir = target_dir.join("nex/pkg");
    fs::create_dir_all(&nex_pkg_dir)?;

    // deploy manifests to /nex/db for runtime package resolution
    deploy_manifests_to_nex_db(&target_dir)?;

    // create /lib64 for nex-ld-shim
    let lib64_dir = target_dir.join("lib64");
    fs::create_dir_all(&lib64_dir)?;

    // track which packages we've installed (by slug) to avoid duplicates
    let mut installed_packages: HashMap<String, String> = HashMap::new();

    for commit in package_commits {
        use crate::refs::PackageRef;

        // parse the commit ref to get package info
        let pkg_ref = match PackageRef::parse(commit) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Warning: could not parse package commit {}: {}", commit, e);
                continue;
            }
        };

        let namespace = &pkg_ref.namespace;
        let slug = &pkg_ref.slug;
        let version = &pkg_ref.version;

        // use commit_ref() for metadata lookup (strips internal path if present)
        let base_commit = pkg_ref.commit_ref();

        // check if this is a kernel output (boot or modules) - these need special handling
        let is_boot_output = commit.contains("/outputs/boot");
        let is_kernel_module_output =
            commit.contains("/kernel/linux/") && commit.contains("/outputs/") && !is_boot_output;

        // kernel module outputs are layered directly into /target/usr/lib/modules/
        // they don't go through the /nex/pkg/ structure
        if is_kernel_module_output {
            println!(
                "Installing kernel modules: {}/{}/{} (direct layer)",
                namespace, slug, version
            );
            checkout_into(repo_path, commit, &target_dir, true, false)?;
            continue;
        }

        // use manifest hash to group outputs from the same build together
        // use base_commit (without internal path) for metadata lookup
        let manifest_hash = get_commit_metadata(repo_path, &base_commit, "nex.manifest.hash")
            .unwrap_or_else(|_| {
                // fallback to commit hash if no manifest hash
                get_commit_id(repo_path, &base_commit).unwrap_or_default()
            });
        let short_hash = &manifest_hash[..8.min(manifest_hash.len())];

        // check if we've already processed this package
        let pkg_key = format!("{}/{}", namespace, slug);
        let already_installed = installed_packages.contains_key(&pkg_key);

        if already_installed {
            println!(
                "  Merging output into {}/{}/{} ({})",
                namespace, slug, version, short_hash
            );
        } else {
            println!("Installing package: {}/{}/{}", namespace, slug, version);
            println!("  Using manifest hash: {} ({})", base_commit, short_hash);
        }

        let (install_ref, checksum) = (commit.clone(), short_hash.to_string());

        // create package directory: /nex/pkg/<ns>/<slug>/<version>/<checksum>/
        let pkg_install_dir = nex_pkg_dir
            .join(&namespace)
            .join(&slug)
            .join(&version)
            .join(&checksum);

        // create parent directories (checkout will create the final directory)
        if let Some(parent) = pkg_install_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        // checkout package using --union to merge multiple outputs of the same package
        // this allows bin and lib outputs to coexist in the same directory
        checkout_into(repo_path, &install_ref, &pkg_install_dir, true, false)?;

        // write .nex-app-root sentinel for nex-ld-shim to find the package root
        let sentinel_path = pkg_install_dir.join(".nex-app-root");
        if !sentinel_path.exists() {
            fs::write(&sentinel_path, format!("{}\n", install_ref))?;
        }

        // create file-level symlinks for all directories in the package
        // rule: directories are real, files are symlinks (stow-style, first package wins)
        for entry in fs::read_dir(&pkg_install_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().to_string();
            let pkg_dir = pkg_install_dir.join(&dir_name);
            let target_subdir = target_dir.join(&dir_name);

            fs::create_dir_all(&target_subdir)?;
            create_file_symlinks_recursive(
                &pkg_dir,
                &target_subdir,
                &namespace,
                &slug,
                &version,
                &checksum,
                &dir_name,
            )?;
        }

        installed_packages.insert(pkg_key, commit.clone());
    }

    // flatten runtime dependencies into each package capsule
    let nex_db_pkg = target_dir.join("nex/db/pkg");
    if nex_db_pkg.exists() {
        flatten_package_dependencies(repo_path, &nex_pkg_dir, &nex_db_pkg)?;
    } else {
        println!("  Skipping dependency flattening: /nex/db/pkg not found");
    }

    // symlink flattened libs to /usr/lib/ (first-come-first-own)
    symlink_flattened_libs_to_usr(&nex_pkg_dir, &target_dir)?;

    // install nex-ld-shim at /lib64/ld-linux-x86-64.so.2
    install_nex_ld_shim(repo_path, &lib64_dir)?;

    // create FHS compatibility symlinks
    create_target_fhs_symlinks(&target_dir)?;

    println!(
        "Materialized {} packages to /nex/pkg/ structure",
        installed_packages.len()
    );

    Ok(())
}

/// Symlink flattened libraries from package capsules to /usr/lib/ (first-come-first-own).
fn symlink_flattened_libs_to_usr(nex_pkg_dir: &Path, target_dir: &Path) -> io::Result<()> {
    let usr_lib = target_dir.join("usr/lib");
    fs::create_dir_all(&usr_lib)?;

    for entry in WalkDir::new(nex_pkg_dir).min_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // look for usr/lib/ directories inside package capsules
        if !path.is_dir() {
            continue;
        }

        let path_str = path.to_string_lossy();
        if !path_str.ends_with("/usr/lib") {
            continue;
        }

        // found a usr/lib/ in a capsule - symlink its contents to /usr/lib/
        for lib_entry in WalkDir::new(path).min_depth(1).max_depth(1) {
            let lib_entry = match lib_entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let lib_path = lib_entry.path();
            let lib_name = match lib_path.file_name() {
                Some(n) => n,
                None => continue,
            };

            let target_path = usr_lib.join(lib_name);

            // first-come-first-own: skip if already exists
            if target_path.exists() || target_path.symlink_metadata().is_ok() {
                continue;
            }

            // compute relative path: /usr/lib -> /nex/pkg/.../usr/lib/file
            // relative path is ../../nex/pkg/{rel_from_nex_pkg}/usr/lib/{file}
            if let Ok(rel_from_nex_pkg) = lib_path.strip_prefix(nex_pkg_dir) {
                let symlink_target = Path::new("../../nex/pkg").join(rel_from_nex_pkg);
                let _ = symlink(&symlink_target, &target_path);
            }
        }
    }

    Ok(())
}

/// Create file-level symlinks recursively (stow-style linking).
/// Rule: directories are real, files are symlinks.
/// This allows multiple packages to contribute files to the same directory.
fn create_file_symlinks_recursive(
    src_dir: &Path,
    dst_dir: &Path,
    namespace: &str,
    slug: &str,
    version: &str,
    checksum: &str,
    relative_base: &str,
) -> io::Result<()> {
    for entry in WalkDir::new(src_dir).min_depth(1) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(src_dir).unwrap();
        let dst_path = dst_dir.join(rel_path);

        if entry.file_type().is_dir() {
            // directories are real - create them
            fs::create_dir_all(&dst_path)?;
        } else {
            // files and symlinks become absolute symlinks to /nex/pkg/
            let target_path = format!(
                "/nex/pkg/{}/{}/{}/{}/{}/{}",
                namespace,
                slug,
                version,
                checksum,
                relative_base,
                rel_path.display()
            );

            // skip if symlink already exists (first package wins)
            if dst_path.symlink_metadata().is_ok() {
                continue;
            }

            symlink(&target_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Install nex-ld-shim at /lib64/ld-linux-x86-64.so.2 from the packaged bundle.
/// nex-ld-shim is a static binary with no runtime deps, so we can use outputs/bin or bundles/full.
fn install_nex_ld_shim(repo_path: &str, lib64_dir: &Path) -> io::Result<()> {
    let shim_path = lib64_dir.join("ld-linux-x86-64.so.2");

    // use Store to list refs
    let store = crate::store::Store::open(repo_path)?;
    let refs = store.refs(None)?;

    // try deploy first, then bundles/full, then outputs/bin
    let shim_ref = refs
        .iter()
        .find(|r| r.contains("nex-ld-shim") && r.contains("/deploy/"))
        .or_else(|| refs.iter().find(|r| r.contains("nex-ld-shim") && r.contains("/bundles/")))
        .or_else(|| refs.iter().find(|r| r.contains("nex-ld-shim") && r.contains("/outputs/")))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "nex-ld-shim not found. Build with: nex build <repo> pkg/core/nex-ld-shim/nex-ld-shim.yaml",
            )
        })?;

    println!("Using packaged nex-ld-shim: {}", shim_ref);

    // export the shim binary directly from the store (no temp checkout)
    export_path(
        repo_path,
        shim_ref,
        "/usr/lib/nex-ld-shim",
        &shim_path,
        false, // copy to allow cross-device targets without hardlink issues
    )?;
    println!("  Installed nex-ld-shim at /lib64/ld-linux-x86-64.so.2");

    Ok(())
}

/// Create FHS compatibility symlinks in the target directory.
fn create_target_fhs_symlinks(target_dir: &Path) -> io::Result<()> {
    let symlinks = [("bin", "usr/bin"), ("sbin", "usr/bin"), ("lib", "usr/lib")];

    for (link_name, target) in &symlinks {
        let link_path = target_dir.join(link_name);
        if !link_path.exists() {
            symlink(target, &link_path)?;
        }
    }

    // ensure /usr/lib exists
    let usr_lib = target_dir.join("usr/lib");
    if !usr_lib.exists() {
        fs::create_dir_all(&usr_lib)?;
    }

    // /usr/sbin -> bin
    let usr_sbin = target_dir.join("usr/sbin");
    if !usr_sbin.exists() {
        symlink("bin", &usr_sbin)?;
    }

    Ok(())
}

/// Apply overlay YAML files to /target.
fn apply_overlays(overlays: &[std::path::PathBuf], base_dir: &str) -> io::Result<()> {
    use crate::manifest::types::Overlay;

    if overlays.is_empty() {
        return Ok(());
    }

    let target_dir = Path::new(base_dir).join("target");

    for overlay_path in overlays {
        if !overlay_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("overlay not found: {}", overlay_path.display()),
            ));
        }

        println!("  Applying overlay: {}", overlay_path.display());

        let overlay_content = fs::read_to_string(overlay_path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to read {}: {}", overlay_path.display(), e),
            )
        })?;

        let overlay: Overlay = serde_yaml::from_str(&overlay_content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse {}: {}", overlay_path.display(), e),
            )
        })?;

        let overlay_dir = overlay_path.parent().unwrap_or(Path::new("."));

        for entry in &overlay.files {
            let rel_path = entry.path.strip_prefix("/").unwrap_or(&entry.path);
            let dst = target_dir.join(rel_path);

            // handle existing paths
            if dst.symlink_metadata().is_ok() {
                if dst.is_dir() && !dst.is_symlink() {
                    // existing directory
                    if entry.replace {
                        // replace: true - remove the directory
                        fs::remove_dir_all(&dst).map_err(|e| {
                            io::Error::new(
                                e.kind(),
                                format!("failed to remove dir {}: {}", dst.display(), e),
                            )
                        })?;
                    } else if !entry.directory {
                        // can't replace dir with non-dir without replace: true
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            format!("overlay wants to replace directory {} with non-directory (use replace: true)", dst.display()),
                        ));
                    }
                    // else: directory exists, entry is directory - just proceed
                } else {
                    // existing file or symlink - remove it
                    fs::remove_file(&dst).map_err(|e| {
                        io::Error::new(
                            e.kind(),
                            format!("failed to remove existing {}: {}", dst.display(), e),
                        )
                    })?;
                }
            }

            // create parent directories if needed
            if let Some(parent) = dst.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| {
                        io::Error::new(
                            e.kind(),
                            format!("failed to create parent dir for {}: {}", dst.display(), e),
                        )
                    })?;
                }
            }

            if entry.directory {
                fs::create_dir_all(&dst)?;
                if let Some(mode) = entry.mode {
                    fs::set_permissions(&dst, Permissions::from_mode(mode))?;
                }
            } else if let Some(ref target) = entry.symlink {
                symlink(target, &dst).map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!(
                            "failed to create symlink {} -> {}: {}",
                            dst.display(),
                            target.display(),
                            e
                        ),
                    )
                })?;
            } else if let Some(ref content) = entry.content {
                let mut file = fs::File::create(&dst).map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!("failed to create file {}: {}", dst.display(), e),
                    )
                })?;
                file.write_all(content.as_bytes())?;
                if let Some(mode) = entry.mode {
                    fs::set_permissions(&dst, Permissions::from_mode(mode))?;
                }
            } else if let Some(ref source) = entry.source {
                let src = overlay_dir.join(source);
                fs::copy(&src, &dst).map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!(
                            "failed to copy {} -> {}: {}",
                            src.display(),
                            dst.display(),
                            e
                        ),
                    )
                })?;
                if let Some(mode) = entry.mode {
                    fs::set_permissions(&dst, Permissions::from_mode(mode))?;
                }
            } else {
                // empty file
                fs::File::create(&dst)?;
                if let Some(mode) = entry.mode {
                    fs::set_permissions(&dst, Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}

/// Deploy manifest files to /nex/db/pkg for runtime package resolution.
///
/// This copies the entire pkg/ directory tree to the target system's /nex/db/pkg/,
/// enabling the ManifestIndex to resolve package providers at runtime without
/// needing to scan the store.
///
/// TODO: uncertain if this belongs in the nex CLI or should be done externally
/// (e.g., by the system assembly tooling). For now it's here for convenience,
/// but may be moved out if manifest deployment needs more control/customization.
fn deploy_manifests_to_nex_db(target_dir: &Path) -> io::Result<()> {
    let src_pkg_dir = Path::new("pkg");
    let dst_db_dir = target_dir.join("nex/db/pkg");

    if !src_pkg_dir.exists() {
        println!("  Warning: pkg/ directory not found, skipping manifest deployment");
        return Ok(());
    }

    println!("Deploying manifests to /nex/db/pkg...");

    // ensure /nex/db exists
    fs::create_dir_all(&dst_db_dir)?;

    // recursively copy all .yaml files from pkg/ to /nex/db/pkg/
    let mut count = 0;
    for entry in WalkDir::new(src_pkg_dir).into_iter().filter_map(|e| e.ok()) {
        let src_path = entry.path();
        let rel_path = src_path.strip_prefix(src_pkg_dir).unwrap_or(src_path);
        let dst_path = dst_db_dir.join(rel_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dst_path)?;
        } else if src_path
            .extension()
            .map_or(false, |ext| ext == "yaml" || ext == "yml")
        {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src_path, &dst_path)?;
            count += 1;
        }
    }

    println!("  Deployed {} manifest files to /nex/db/pkg", count);
    Ok(())
}

/// Flatten runtime dependencies into each package's lib/ directory.
/// This creates self-contained "capsules" that nex-ld-shim can use.
/// Uses precomputed deps from manifests - no ELF scanning at install time.
fn flatten_package_dependencies(
    repo_path: &str,
    nex_pkg_dir: &Path,
    manifest_dir: &Path,
) -> io::Result<()> {
    println!("Flattening runtime dependencies into package capsules...");

    // load manifest index for precomputed deps
    let manifest_index = ManifestIndex::load(manifest_dir)?;
    println!("  Loaded {} manifests", manifest_index.manifest_count());

    // iterate all package directories under nex/pkg/
    // look for directories with .nex-app-root (the package capsules)
    for entry in WalkDir::new(nex_pkg_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }

        let pkg_dir = entry.path();

        // only process directories that have .nex-app-root (valid package capsule)
        let app_root = pkg_dir.join(".nex-app-root");
        if !app_root.exists() {
            continue;
        }

        // read the commit ref from .nex-app-root
        let commit = fs::read_to_string(&app_root)?
            .lines()
            .next()
            .map(|s| s.to_string())
            .unwrap_or_default();

        if commit.is_empty() {
            continue;
        }

        // use precomputed deps from manifest
        let flattened_count =
            flatten_capsule_precomputed(repo_path, pkg_dir, &commit, &manifest_index, &[])?;

        if flattened_count > 0 {
            let rel_path = pkg_dir.strip_prefix(nex_pkg_dir).unwrap_or(pkg_dir);
            println!(
                "  Flattened {} libs into {}",
                flattened_count,
                rel_path.display()
            );
        }
    }

    Ok(())
}
