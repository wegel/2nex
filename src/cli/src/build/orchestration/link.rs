//! Manifest dependency linking to git blob references.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::{
    load_manifest, repository_root_for_path, ManifestData, ManifestRepositories,
};
use crate::utils::hash_file_content;

use super::manifest_lookup::find_manifest_for_commit;

/// Pin manifest dependencies and build environment paths to current git blob SHAs.
pub fn link_manifest_dependencies(manifest_file: &str) -> io::Result<()> {
    let manifest_path = Path::new(manifest_file).canonicalize()?;
    let manifest_path_str = manifest_path.to_string_lossy();
    let repositories = ManifestRepositories::discover(&manifest_path)?;
    let manifest_dirs = repositories.package_dirs();
    let manifest_data = load_manifest(&manifest_path_str)?;
    print_manifest_kind(&manifest_path_str, &manifest_data);

    let original_content = fs::read_to_string(&manifest_path)?;
    let mut updated_content = original_content.clone();
    let mut linked_count = link_dependencies(&manifest_data, &manifest_dirs, &mut updated_content)?;

    print_system_package_refs(&manifest_data, &manifest_dirs)?;
    linked_count += link_environment_ref(&manifest_data, &manifest_path, &mut updated_content)?;
    write_linked_manifest(&manifest_path_str, updated_content, linked_count)
}

fn print_manifest_kind(manifest_file: &str, manifest_data: &ManifestData) {
    match manifest_data {
        ManifestData::Package(_) => println!("Linking package manifest: {}", manifest_file),
        ManifestData::System(_) => println!("Linking system manifest: {}", manifest_file),
    }
}

fn link_dependencies(
    manifest_data: &ManifestData,
    manifest_dirs: &[PathBuf],
    updated_content: &mut String,
) -> io::Result<usize> {
    let dependencies = match manifest_data {
        ManifestData::Package(manifest) => &manifest.dependencies,
        ManifestData::System(manifest) => &manifest.dependencies,
    };
    let mut linked_count = 0;

    for dep in dependencies {
        match find_manifest_for_commit(&dep.commit, manifest_dirs) {
            Ok(manifest_path) => {
                let blob_sha = hash_file_content(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), blob_sha);
                linked_count += insert_manifest_ref(updated_content, &dep.commit, &blob_sha);
            }
            Err(e) => eprintln!("  Warning: {}: {}", dep.commit, e),
        }
    }

    Ok(linked_count)
}

fn insert_manifest_ref(updated_content: &mut String, commit: &str, blob_sha: &str) -> usize {
    let commit_pattern = format!("commit: {}", commit);
    let Some(pos) = updated_content.find(&commit_pattern) else {
        return 0;
    };

    let after_commit = pos + commit_pattern.len();
    let next_section = next_manifest_entry_offset(updated_content, after_commit);
    let entry_section = &updated_content[pos..after_commit + next_section];
    if entry_section.contains("manifest_ref:") {
        return 0;
    }

    let insertion = format!("\n    manifest_ref: {}", blob_sha);
    updated_content.insert_str(after_commit, &insertion);
    1
}

fn next_manifest_entry_offset(content: &str, after_commit: usize) -> usize {
    content[after_commit..]
        .find("\n  - ")
        .or_else(|| content[after_commit..].find("\nsources:"))
        .or_else(|| content[after_commit..].find("\npackages:"))
        .unwrap_or(content.len() - after_commit)
}

fn print_system_package_refs(
    manifest_data: &ManifestData,
    manifest_dirs: &[PathBuf],
) -> io::Result<()> {
    let ManifestData::System(manifest) = manifest_data else {
        return Ok(());
    };

    for pkg in &manifest.packages {
        match find_manifest_for_commit(&pkg.commit, manifest_dirs) {
            Ok(manifest_path) => {
                let blob_sha = hash_file_content(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), blob_sha);
            }
            Err(e) => eprintln!("  Warning: {}: {}", pkg.commit, e),
        }
    }
    Ok(())
}

fn link_environment_ref(
    manifest_data: &ManifestData,
    manifest_path: &Path,
    updated_content: &mut String,
) -> io::Result<usize> {
    let Some(env) = manifest_environment(manifest_data) else {
        return Ok(0);
    };
    if is_git_sha(&env) {
        return Ok(0);
    }

    let repository_root = repository_root_for_path(manifest_path)?;
    let declared_path = Path::new(&env);
    let env_path = if declared_path.is_absolute() {
        declared_path.to_path_buf()
    } else {
        repository_root.join(declared_path)
    };
    if !env_path.exists() {
        eprintln!("  Warning: environment file not found: {}", env);
        return Ok(0);
    }

    ensure_file_is_tracked(&repository_root, &env_path)?;
    let env_sha = hash_file_content(&env_path)?;
    println!("  environment: {} -> {}", env, env_sha);
    Ok(replace_environment_ref(updated_content, &env, &env_sha))
}

fn manifest_environment(manifest_data: &ManifestData) -> Option<String> {
    match manifest_data {
        ManifestData::Package(manifest) => Some(manifest.build.environment.clone()),
        ManifestData::System(manifest) => Some(manifest.build.environment.clone()),
    }
}

fn is_git_sha(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn ensure_file_is_tracked(repository_root: &Path, path: &Path) -> io::Result<()> {
    let relative_path = path.strip_prefix(repository_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is outside {}",
                path.display(),
                repository_root.display()
            ),
        )
    })?;
    let git_check = std::process::Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .arg("ls-files")
        .arg(relative_path)
        .output()?;
    if !String::from_utf8_lossy(&git_check.stdout).trim().is_empty() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "Environment file '{}' is not tracked by git. Run 'git add {}' first.",
            path.display(),
            path.display()
        ),
    ))
}

fn replace_environment_ref(updated_content: &mut String, env: &str, env_sha: &str) -> usize {
    let patterns = [
        format!("environment: {}", env),
        format!("environment: '{}'", env),
        format!("environment: \"{}\"", env),
    ];

    for pattern in patterns {
        if updated_content.contains(&pattern) {
            *updated_content =
                updated_content.replace(&pattern, &format!("environment: {}", env_sha));
            return 1;
        }
    }
    0
}

fn write_linked_manifest(
    manifest_file: &str,
    updated_content: String,
    linked_count: usize,
) -> io::Result<()> {
    if linked_count > 0 {
        fs::write(manifest_file, updated_content)?;
        println!("\nLinked {} references in {}", linked_count, manifest_file);
    } else {
        println!("\nNo references to link or all already linked");
    }
    Ok(())
}
