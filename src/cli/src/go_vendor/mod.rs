//! go_sum source type: vendors Go dependencies by parsing go.sum and go.mod.
//!
//! this module parses go.sum and go.mod files, downloads each module from proxy.golang.org,
//! and creates a deterministic vendor tarball. Similar to Nix's buildGoModule.

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::{copy_dir_recursive, create_deterministic_tarball, fetch_url_or_file};

/// vendor Go dependencies by parsing go.sum and go.mod.
/// returns path to the vendored tarball.
pub fn vendor_from_sum(
    go_sum_ref: &str,
    expected_sha256: &str,
    cache_dir: &str,
) -> io::Result<PathBuf> {
    // check cache first
    let vendor_cache = Path::new(cache_dir).join("go_vendor");
    let cache_path = vendor_cache.join(format!("sha256-{}.tar.gz", expected_sha256));

    if cache_path.exists() {
        // verify cached tarball
        let mut file = File::open(&cache_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        let calculated_hash = hex::encode(Sha256::digest(&contents));

        if calculated_hash == expected_sha256 {
            println!("Found cached Go vendor tarball: {}", cache_path.display());
            return Ok(cache_path);
        } else {
            println!(
                "Cached Go vendor tarball hash mismatch, re-vendoring (expected {}, got {})",
                expected_sha256, calculated_hash
            );
            fs::remove_file(&cache_path)?;
        }
    }

    // fetch go.sum content
    println!("Fetching go.sum from {}", go_sum_ref);
    let go_sum_content = fetch_url_or_file(go_sum_ref)?;

    // derive go.mod URL from go.sum URL and fetch it
    let go_mod_ref = go_sum_ref.replace("go.sum", "go.mod");
    println!("Fetching go.mod from {}", go_mod_ref);
    let go_mod_content = fetch_url_or_file(&go_mod_ref)?;

    // parse go.mod to get required modules (direct and indirect)
    let (direct_deps, indirect_deps) = parse_go_mod(&go_mod_content)?;
    let all_required: HashSet<String> = direct_deps.iter().chain(indirect_deps.iter()).cloned().collect();

    // parse go.sum and filter to only modules in go.mod
    let all_modules = parse_go_sum(&go_sum_content)?;
    let modules: Vec<GoModule> = all_modules
        .into_iter()
        .filter(|m| all_required.contains(&m.path))
        .collect();

    println!(
        "Found {} modules in go.mod ({} direct, {} indirect)",
        modules.len(),
        direct_deps.len(),
        indirect_deps.len()
    );

    // create work directory for vendor
    let work_dir = tempfile::tempdir()?;
    let vendor_dir = work_dir.path().join("vendor");
    fs::create_dir_all(&vendor_dir)?;

    // download each module
    let module_cache = vendor_cache.join("modules");
    fs::create_dir_all(&module_cache)?;

    for module in &modules {
        download_and_extract_module(module, &vendor_dir, &module_cache)?;
    }

    // create modules.txt file (required by go for vendor mode)
    create_modules_txt(&modules, &vendor_dir, &direct_deps)?;

    // create deterministic tarball
    fs::create_dir_all(&vendor_cache)?;
    let tmp_tarball = cache_path.with_extension("tmp");
    create_deterministic_tarball(&vendor_dir, &tmp_tarball)?;

    // verify hash
    let mut file = File::open(&tmp_tarball)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    let calculated_hash = hex::encode(Sha256::digest(&contents));

    if calculated_hash != expected_sha256 {
        // keep the tarball for inspection but rename it
        let debug_path = vendor_cache.join(format!("sha256-{}.tar.gz", calculated_hash));
        fs::rename(&tmp_tarball, &debug_path)?;

        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Go vendor tarball hash mismatch: expected {}, got {}.\nTarball saved to {} for inspection.",
                expected_sha256, calculated_hash, debug_path.display()
            ),
        ));
    }

    // move to final location
    fs::rename(&tmp_tarball, &cache_path)?;
    println!("Created Go vendor tarball: {}", cache_path.display());

    Ok(cache_path)
}

/// parse go.mod and return (direct_deps, indirect_deps)
fn parse_go_mod(content: &str) -> io::Result<(HashSet<String>, HashSet<String>)> {
    let mut direct_deps = HashSet::new();
    let mut indirect_deps = HashSet::new();
    let mut in_require_block = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("require (") || line == "require(" {
            in_require_block = true;
            continue;
        }

        if in_require_block && line == ")" {
            in_require_block = false;
            continue;
        }

        // handle single-line require: require github.com/foo/bar v1.0.0
        if line.starts_with("require ") && !line.contains("(") {
            let parts: Vec<&str> = line.strip_prefix("require ").unwrap().split_whitespace().collect();
            if !parts.is_empty() {
                let is_indirect = line.contains("// indirect");
                if is_indirect {
                    indirect_deps.insert(parts[0].to_string());
                } else {
                    direct_deps.insert(parts[0].to_string());
                }
            }
            continue;
        }

        // handle require block entries
        if in_require_block && !line.is_empty() && !line.starts_with("//") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let is_indirect = line.contains("// indirect");
                if is_indirect {
                    indirect_deps.insert(parts[0].to_string());
                } else {
                    direct_deps.insert(parts[0].to_string());
                }
            }
        }
    }

    Ok((direct_deps, indirect_deps))
}

/// module info from go.sum
#[derive(Debug, Clone)]
struct GoModule {
    path: String,
    version: String,
    hash: String,
}

/// parse go.sum and extract modules (deduplicated)
fn parse_go_sum(content: &str) -> io::Result<Vec<GoModule>> {
    let mut modules = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // format: module/path version hash
        // or: module/path version/go.mod hash
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            continue;
        }

        let path = parts[0];
        let version_part = parts[1];
        let hash = parts[2];

        // skip go.mod entries, we only need the module source
        if version_part.ends_with("/go.mod") {
            continue;
        }

        // deduplicate
        let key = format!("{}@{}", path, version_part);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);

        modules.push(GoModule {
            path: path.to_string(),
            version: version_part.to_string(),
            hash: hash.to_string(),
        });
    }

    Ok(modules)
}

/// download and extract a Go module from proxy.golang.org
fn download_and_extract_module(
    module: &GoModule,
    vendor_dir: &Path,
    cache_dir: &Path,
) -> io::Result<()> {
    // vendor path: vendor/module/path (without version)
    let dest_dir = vendor_dir.join(&module.path);

    if dest_dir.exists() {
        return Ok(());
    }

    // cache by module@version
    let cache_key = format!("{}@{}", module.path.replace('/', "_"), module.version);
    let zip_file = cache_dir.join(format!("{}.zip", cache_key));

    if !zip_file.exists() {
        // download from proxy.golang.org
        // URL: https://proxy.golang.org/{module}/@v/{version}.zip
        // module path needs to be escaped (uppercase -> !lowercase)
        let escaped_path = escape_module_path(&module.path);
        let url = format!(
            "https://proxy.golang.org/{}/@v/{}.zip",
            escaped_path, module.version
        );

        println!("  Downloading {}@{}", module.path, module.version);

        let status = Command::new("curl")
            .args([
                "-L",
                "-f",
                "-s",
                "--output",
                zip_file.to_str().unwrap(),
                &url,
            ])
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to download {}", url),
            ));
        }
    }

    // extract to temp directory
    // Go module zips contain: modulepath@version/files...
    // we need to copy contents to vendor/modulepath/
    let tmp_extract = cache_dir.join(format!("tmp-{}", cache_key));
    if tmp_extract.exists() {
        fs::remove_dir_all(&tmp_extract)?;
    }
    fs::create_dir_all(&tmp_extract)?;

    let status = Command::new("unzip")
        .args([
            "-q",
            zip_file.to_str().unwrap(),
            "-d",
            tmp_extract.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        fs::remove_dir_all(&tmp_extract).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to extract {}@{}", module.path, module.version),
        ));
    }

    // Go module zip structure: modulepath@version/files...
    // e.g., github.com/foo/bar@v1.0.0/file.go
    let versioned_path = format!("{}@{}", module.path, module.version);
    let extracted_dir = tmp_extract.join(&versioned_path);

    if !extracted_dir.exists() {
        fs::remove_dir_all(&tmp_extract).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Expected directory {} not found in zip for {}@{}",
                versioned_path, module.path, module.version
            ),
        ));
    }

    // create destination and copy contents (not the directory itself)
    fs::create_dir_all(&dest_dir)?;
    copy_dir_recursive(&extracted_dir, &dest_dir)?;

    fs::remove_dir_all(&tmp_extract).ok();

    Ok(())
}

/// escape module path for proxy.golang.org URL
/// uppercase letters become !lowercase (e.g., GitHub -> !git!hub)
fn escape_module_path(path: &str) -> String {
    let mut result = String::new();
    for c in path.chars() {
        if c.is_ascii_uppercase() {
            result.push('!');
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// create vendor/modules.txt file
/// lists all packages (directories with .go files) for each module
fn create_modules_txt(modules: &[GoModule], vendor_dir: &Path, direct_deps: &HashSet<String>) -> io::Result<()> {
    let mut lines = Vec::new();

    // sort modules for reproducibility
    let mut sorted_modules = modules.to_vec();
    sorted_modules.sort_by(|a, b| a.path.cmp(&b.path));

    for module in &sorted_modules {
        // format: # module/path version
        lines.push(format!("# {} {}", module.path, module.version));

        // read go version from module's go.mod if present
        let module_dir = vendor_dir.join(&module.path);
        let go_version = read_go_version(&module_dir);

        // all modules in go.mod (direct or indirect) are "explicitly required"
        // only test deps (in go.sum but not go.mod) should be excluded entirely
        if let Some(ver) = go_version {
            lines.push(format!("## explicit; go {}", ver));
        } else {
            lines.push("## explicit".to_string());
        }

        // find all packages (directories with .go files) in this module
        let mut packages = find_go_packages(&module_dir, &module.path)?;
        packages.sort();

        for pkg in packages {
            lines.push(pkg);
        }
    }

    let content = lines.join("\n") + "\n";
    fs::write(vendor_dir.join("modules.txt"), content)?;

    Ok(())
}

/// read go version from a module's go.mod file
fn read_go_version(module_dir: &Path) -> Option<String> {
    let go_mod_path = module_dir.join("go.mod");
    if !go_mod_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&go_mod_path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("go ") {
            // extract version after "go "
            let version = line.strip_prefix("go ")?.trim();
            // handle cases like "go 1.21" or "go 1.21.0"
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}

/// find all Go packages (directories containing .go files) under a directory
fn find_go_packages(dir: &Path, base_path: &str) -> io::Result<Vec<String>> {
    let mut packages = Vec::new();

    if !dir.exists() {
        return Ok(packages);
    }

    // check if this directory has .go files
    let has_go_files = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .any(|e| {
            e.path().extension().map_or(false, |ext| ext == "go")
                && !e.file_name().to_string_lossy().ends_with("_test.go")
        });

    if has_go_files {
        packages.push(base_path.to_string());
    }

    // recurse into subdirectories
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            // skip hidden directories and testdata
            if !name.starts_with('.') && name != "testdata" {
                let sub_path = format!("{}/{}", base_path, name);
                let sub_packages = find_go_packages(&path, &sub_path)?;
                packages.extend(sub_packages);
            }
        }
    }

    Ok(packages)
}
