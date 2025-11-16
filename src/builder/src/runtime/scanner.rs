use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::process::Command;

use goblin::Object;
use walkdir::WalkDir;

use crate::utils::{determine_category, manifest_prefix};

/// High-level API for running the runtime dependency scanner.
///
/// The struct owns the contextual information shared across scans
/// (repository path, dependency closure, and runtime handling flags)
/// so callers only provide per-package bits (name, version, base dir).
pub struct RuntimeScanner<'a> {
    repo_path: &'a str,
    dependency_commits: &'a [String],
    allow_missing_files: bool,
}

impl<'a> RuntimeScanner<'a> {
    pub fn new(repo_path: &'a str, dependency_commits: &'a [String]) -> Self {
        Self {
            repo_path,
            dependency_commits,
            allow_missing_files: false,
        }
    }

    pub fn with_allow_missing_files(mut self, allow_missing_files: bool) -> Self {
        self.allow_missing_files = allow_missing_files;
        self
    }

    pub fn scan<P: AsRef<Path>>(
        &self,
        package_name: &str,
        package_version: &str,
        base_dir: P,
        verbose_reasons: bool,
    ) -> io::Result<RuntimeScanResult> {
        scan_runtime_dependencies(
            package_name,
            package_version,
            base_dir.as_ref(),
            self.repo_path,
            self.dependency_commits,
            verbose_reasons,
            self.allow_missing_files,
        )
    }
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

fn scan_runtime_dependencies(
    package_name: &str,
    package_version: &str,
    base_dir: &Path,
    repo_path: &str,
    dependency_commits: &[String],
    verbose_reasons: bool,
    allow_missing_files: bool,
) -> io::Result<RuntimeScanResult> {
    let out_dir = base_dir.join("2nex/out");

    if !out_dir.exists() {
        println!(
            "No output directory found at {}. Skipping runtime dependency scan.",
            out_dir.display()
        );
        return Ok(RuntimeScanResult::default());
    }

    println!(
        "Scanning runtime dependencies for {} {}",
        package_name, package_version
    );

    // detect if we're building a phase3 package by checking dependencies
    let is_phase3 = dependency_commits.iter().any(|dep| dep.contains("/bootstrap/phase3/"));

    let local_basenames = collect_local_basenames(&out_dir)?;
    let provider_index = build_provider_index(repo_path, dependency_commits)?;
    let mut result = RuntimeScanResult::default();
    result.is_phase3 = is_phase3;

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
            package_name, package_version
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

fn build_provider_index(repo_path: &str, dependencies: &[String]) -> io::Result<ProviderIndex> {
    let mut index = ProviderIndex::default();
    let mut outputs_cache: HashMap<String, HashSet<String>> = HashMap::new();

    for dep in dependencies {
        let prefix = manifest_prefix(dep).unwrap_or_else(|| dep.clone());
        let outputs = match outputs_cache.entry(prefix.clone()) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
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
                );
            }
        }
    }

    Ok(())
}

// extract phase priority from commit path
// higher number = later phase = higher priority
fn phase_priority(commit: &str) -> u32 {
    if commit.contains("/bootstrap/phase3/") {
        3
    } else if commit.contains("/bootstrap/phase2/") {
        2
    } else if commit.contains("/bootstrap/phase1/") {
        1
    } else {
        // non-bootstrap packages (e.g., kernel flavor) have highest priority
        100
    }
}

fn resolve_requirement(providers: &ProviderIndex, reference: &str) -> Vec<ProviderMatch> {
    let trimmed = reference.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut matches = if let Some(m) = providers.by_full_path.get(trimmed) {
        m.clone()
    } else if let Some(basename) = Path::new(trimmed).file_name().and_then(|n| n.to_str()) {
        if let Some(m) = providers.by_basename.get(basename) {
            m.clone()
        } else {
            return Vec::new();
        }
    } else {
        return Vec::new();
    };

    if matches.len() <= 1 {
        return matches;
    }

    // find highest phase priority among matches
    let max_priority = matches.iter()
        .map(|m| phase_priority(&m.commit))
        .max()
        .unwrap_or(0);

    // filter to only keep matches with highest priority
    matches.retain(|m| phase_priority(&m.commit) == max_priority);

    matches
}

fn handle_shebang_requirement(
    target: &str,
    display_path: &str,
    out_dir: &Path,
    local_basenames: &HashSet<String>,
    providers: &ProviderIndex,
    result: &mut RuntimeScanResult,
    category: &str,
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
        // for phase3 packages, drop shebang dependencies to phase1/phase2
        // (shell scripts don't need strict runtime deps on interpreters during bootstrap)
        for candidate in matches {
            if result.is_phase3 && phase_priority(&candidate.commit) < 3 {
                continue;
            }
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

#[derive(Default, Clone)]
pub struct RuntimeScanResult {
    resolved: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    unresolved: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    is_phase3: bool,
}

impl RuntimeScanResult {
    pub fn add_resolved(&mut self, category: &str, commit: &str, reason: String) {
        self.resolved
            .entry(category.to_string())
            .or_default()
            .entry(commit.to_string())
            .or_default()
            .insert(reason);
    }

    pub fn add_unresolved(&mut self, category: &str, requirement: String, reason: String) {
        self.unresolved
            .entry(category.to_string())
            .or_default()
            .entry(requirement)
            .or_default()
            .insert(reason);
    }

    pub fn category_resolved(&self, category: &str) -> Option<&BTreeMap<String, BTreeSet<String>>> {
        self.resolved.get(category)
    }

    pub fn category_unresolved(
        &self,
        category: &str,
    ) -> Option<&BTreeMap<String, BTreeSet<String>>> {
        self.unresolved.get(category)
    }

    pub fn is_empty(&self) -> bool {
        self.resolved.is_empty() && self.unresolved.is_empty()
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
            return Err(e);
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
    use tempfile::tempdir;

    #[test]
    fn parse_shebang_extracts_interpreter_and_args() {
        let dir = tempdir().unwrap();
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
    fn runtime_scanner_builder_sets_flags() {
        let deps: Vec<String> = Vec::new();
        let scanner = RuntimeScanner::new("/tmp/repo", &deps).with_allow_missing_files(true);
        assert!(scanner.allow_missing_files);
        assert_eq!(scanner.repo_path, "/tmp/repo");
        assert!(scanner.dependency_commits.is_empty());
    }
}
