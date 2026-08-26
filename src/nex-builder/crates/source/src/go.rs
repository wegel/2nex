//! go.sum parsing and Go vendor archives.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

use crate::{archive, cache, fetch, invalid};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Module {
    path: String,
    version: String,
    checksum: String,
}

pub(crate) fn vendor(
    sum_reference: &str,
    manifest: &Path,
    cache_root: &Path,
    output: &Path,
) -> io::Result<()> {
    let sum = fetch::text(sum_reference, manifest)?;
    let mod_reference = go_mod_reference(sum_reference)?;
    let module_file = fetch::text(&mod_reference, manifest)?;
    let required = parse_go_mod(&module_file);
    let modules = parse_go_sum(&sum)
        .into_iter()
        .filter(|module| required.get(&module.path) == Some(&module.version))
        .collect::<Vec<_>>();
    let work = tempfile::tempdir()?;
    let vendor = work.path().join("vendor");
    fs::create_dir(&vendor)?;
    let module_cache = cache_root.join("go_vendor/modules");
    for module in &modules {
        stage_module(module, &module_cache, &vendor)?;
    }
    write_modules(&modules, &vendor)?;
    archive::create(&vendor, output)
}

fn go_mod_reference(sum: &str) -> io::Result<String> {
    sum.strip_suffix("go.sum")
        .map(|prefix| format!("{prefix}go.mod"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("go_sum reference does not end in go.sum: {sum}"),
            )
        })
}

fn parse_go_mod(content: &str) -> BTreeMap<String, String> {
    let mut required = BTreeMap::new();
    let mut block = false;
    for raw in content.lines() {
        let line = raw.trim();
        if line == "require (" || line == "require(" {
            block = true;
        } else if block && line == ")" {
            block = false;
        } else if block {
            insert_module(line, &mut required);
        } else if let Some(requirement) = line.strip_prefix("require ") {
            insert_module(requirement, &mut required);
        }
    }
    required
}

fn insert_module(line: &str, required: &mut BTreeMap<String, String>) {
    if line.is_empty() || line.starts_with("//") {
        return;
    }
    let mut fields = line.split_whitespace();
    if let (Some(path), Some(version)) = (fields.next(), fields.next()) {
        required.insert(path.to_string(), version.to_string());
    }
}

fn parse_go_sum(content: &str) -> Vec<Module> {
    let mut modules = Vec::new();
    let mut seen = BTreeSet::new();
    for line in content.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || fields[1].ends_with("/go.mod") {
            continue;
        }
        let key = (fields[0].to_string(), fields[1].to_string());
        if seen.insert(key.clone()) {
            modules.push(Module {
                path: key.0,
                version: key.1,
                checksum: fields[2].to_string(),
            });
        }
    }
    modules
}

fn stage_module(module: &Module, module_cache: &Path, vendor: &Path) -> io::Result<()> {
    safe_module(module)?;
    fs::create_dir_all(module_cache)?;
    let zip = module_cache
        .join(escape(&module.path))
        .join(format!("{}.zip", escape(&module.version)));
    cache::locked(&zip, || {
        if zip.is_file() && verify_module_zip(&zip, &module.checksum).is_ok() {
            return Ok(());
        }
        let work = tempfile::Builder::new()
            .prefix(".module-")
            .tempdir_in(module_cache)?;
        let temporary = work.path().join("download");
        let url = format!(
            "https://proxy.golang.org/{}/@v/{}.zip",
            escape(&module.path),
            escape(&module.version)
        );
        fetch::download(&url, &temporary)?;
        verify_module_zip(&temporary, &module.checksum)?;
        if zip.exists() {
            fs::remove_file(&zip)?;
        }
        fs::rename(temporary, &zip)
    })?;

    let work = tempfile::Builder::new()
        .prefix(".unzip-")
        .tempdir_in(module_cache)?;
    archive::extract_zip(&zip, work.path())?;
    let original = work
        .path()
        .join(format!("{}@{}", module.path, module.version));
    let escaped = work.path().join(format!(
        "{}@{}",
        escape(&module.path),
        escape(&module.version)
    ));
    let extracted = if original.is_dir() { original } else { escaped };
    if !extracted.is_dir() {
        return Err(invalid(format!(
            "module zip did not contain {}@{}",
            module.path, module.version
        )));
    }
    let destination = vendor.join(&module.path);
    if !destination.exists() {
        archive::copy_tree(&extracted, &destination)?;
    }
    Ok(())
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_uppercase() {
            escaped.push('!');
            escaped.push((byte + b'a' - b'A') as char);
        } else {
            escaped.push(byte as char);
        }
    }
    escaped
}

fn verify_module_zip(path: &Path, expected: &str) -> io::Result<()> {
    let actual = module_zip_hash(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(invalid(format!(
            "Go module checksum mismatch for {}: expected {expected}, got {actual}",
            path.display()
        )))
    }
}

fn module_zip_hash(path: &Path) -> io::Result<String> {
    let mut archive = zip::ZipArchive::new(File::open(path)?).map_err(io::Error::other)?;
    let mut files = Vec::with_capacity(archive.len());
    let mut buffer = [0; 64 * 1024];
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(io::Error::other)?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        if name.contains('\n') {
            return Err(invalid("Go module ZIP contains a newline in a file name"));
        }
        let mut hash = Sha256::new();
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        files.push((name, hash.finalize()));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = Sha256::new();
    for (name, contents) in files {
        hash.update(format!("{contents:x}  {name}\n"));
    }
    Ok(format!("h1:{}", STANDARD.encode(hash.finalize())))
}

fn safe_module(module: &Module) -> io::Result<()> {
    let unsafe_path = Path::new(&module.path)
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)));
    let unsafe_version = !crate::safe_token(&module.version, b".-+_");
    if unsafe_path || unsafe_version {
        Err(invalid(format!(
            "unsafe Go module {}@{}",
            module.path, module.version
        )))
    } else {
        Ok(())
    }
}

fn write_modules(modules: &[Module], vendor: &Path) -> io::Result<()> {
    let mut modules = modules.to_vec();
    modules.sort_by(|left, right| left.path.cmp(&right.path));
    let mut lines = Vec::new();
    for module in modules {
        lines.push(format!("# {} {}", module.path, module.version));
        match read_go_version(&vendor.join(&module.path)) {
            Some(version) => lines.push(format!("## explicit; go {version}")),
            None => lines.push("## explicit".to_string()),
        }
        lines.extend(find_packages(&vendor.join(&module.path), &module.path)?);
    }
    fs::write(vendor.join("modules.txt"), lines.join("\n") + "\n")
}

fn read_go_version(module: &Path) -> Option<String> {
    let content = fs::read_to_string(module.join("go.mod")).ok()?;
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix("go ")
            .and_then(|value| value.split("//").next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn find_packages(directory: &Path, module_path: &str) -> io::Result<Vec<String>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut packages = BTreeSet::new();
    find_packages_below(directory, directory, module_path, &mut packages)?;
    Ok(packages.into_iter().collect())
}

fn find_packages_below(
    root: &Path,
    directory: &Path,
    module_path: &str,
    packages: &mut BTreeSet<String>,
) -> io::Result<()> {
    let entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    let has_source = entries.iter().any(|entry| {
        entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "go")
            && !entry.file_name().to_string_lossy().ends_with("_test.go")
    });
    if has_source {
        let relative = directory.strip_prefix(root).map_err(io::Error::other)?;
        let suffix = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        packages.insert(if suffix.is_empty() {
            module_path.to_string()
        } else {
            format!("{module_path}/{suffix}")
        });
    }
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().is_dir() && !name.starts_with('.') && name != "testdata" {
            find_packages_below(root, &entry.path(), module_path, packages)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "go_tests.rs"]
mod tests;
