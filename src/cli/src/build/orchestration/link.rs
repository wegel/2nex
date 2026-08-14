//! Manifest dependency linking to Git repository revisions.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::{
    load_manifest, repository_root_for_path, ManifestData, ManifestRepositories, Source,
};

use super::manifest_lookup::find_manifest_for_commit;

/// Pin manifest dependencies to repository revisions and the build environment to a Git blob.
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

    linked_count += link_system_packages(&manifest_data, &manifest_dirs, &mut updated_content)?;
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
                let revision = repository_revision_for_manifest(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), revision);
                linked_count += upsert_manifest_refs_in_section(
                    updated_content,
                    "dependencies",
                    &dep.commit,
                    &revision,
                );
            }
            Err(e) => eprintln!("  Warning: {}: {}", dep.commit, e),
        }
    }

    Ok(linked_count)
}

fn upsert_manifest_refs_in_section(
    content: &mut String,
    section_name: &str,
    commit: &str,
    revision: &str,
) -> usize {
    let lines = line_spans(content);
    let Some((section_start, section_end)) =
        top_level_section_bounds(content, &lines, section_name)
    else {
        return 0;
    };

    let mut edits = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        if line.start < section_start || line.start >= section_end {
            continue;
        }
        let line_text = &content[line.start..line.content_end];
        if !line_has_value(line_text, "commit", commit) {
            continue;
        }

        let field_indent = leading_whitespace(line_text);
        let entry_start = lines[..=line_index]
            .iter()
            .rev()
            .find(|candidate| {
                candidate.start >= section_start
                    && is_next_list_entry(
                        &content[candidate.start..candidate.content_end],
                        field_indent,
                    )
            })
            .map(|candidate| candidate.start)
            .unwrap_or(line.start);
        let entry_end = lines
            .iter()
            .skip(line_index + 1)
            .find(|candidate| {
                candidate.start >= section_end
                    || is_next_list_entry(
                        &content[candidate.start..candidate.content_end],
                        field_indent,
                    )
            })
            .map(|candidate| candidate.start.min(section_end))
            .unwrap_or(section_end);

        let manifest_ref = lines
            .iter()
            .skip_while(|candidate| candidate.start < entry_start)
            .take_while(|candidate| candidate.start < entry_end)
            .find(|candidate| {
                let candidate_text = &content[candidate.start..candidate.content_end];
                leading_whitespace(candidate_text) == field_indent
                    && candidate_text.trim_start().starts_with("manifest_ref:")
            });

        if let Some(manifest_ref) = manifest_ref {
            let line_text = &content[manifest_ref.start..manifest_ref.content_end];
            let value_start = manifest_ref.start
                + line_text
                    .find(':')
                    .expect("manifest_ref line contains a colon")
                + 1;
            let replacement = format!(" {}", revision);
            if content[value_start..manifest_ref.content_end] != replacement {
                edits.push((value_start, manifest_ref.content_end, replacement));
            }
        } else {
            let indent = &line_text[..field_indent];
            edits.push((
                line.content_end,
                line.content_end,
                format!("\n{}manifest_ref: {}", indent, revision),
            ));
        }
    }

    let edit_count = edits.len();
    for (start, end, replacement) in edits.into_iter().rev() {
        content.replace_range(start..end, &replacement);
    }
    edit_count
}

#[derive(Clone, Copy)]
struct LineSpan {
    start: usize,
    content_end: usize,
    end: usize,
}

fn line_spans(content: &str) -> Vec<LineSpan> {
    let mut lines = Vec::new();
    let mut start = 0;
    for segment in content.split_inclusive('\n') {
        let end = start + segment.len();
        let content_end = end - usize::from(segment.ends_with('\n'));
        lines.push(LineSpan {
            start,
            content_end,
            end,
        });
        start = end;
    }
    if content.is_empty() || start < content.len() {
        lines.push(LineSpan {
            start,
            content_end: content.len(),
            end: content.len(),
        });
    }
    lines
}

fn top_level_section_bounds(
    content: &str,
    lines: &[LineSpan],
    section_name: &str,
) -> Option<(usize, usize)> {
    let header = format!("{}:", section_name);
    let header_index = lines
        .iter()
        .position(|line| content[line.start..line.content_end].trim_end() == header)?;
    let section_start = lines[header_index].end;
    let section_end = lines
        .iter()
        .skip(header_index + 1)
        .find(|line| is_top_level_mapping_key(&content[line.start..line.content_end]))
        .map(|line| line.start)
        .unwrap_or(content.len());
    Some((section_start, section_end))
}

fn is_top_level_mapping_key(line: &str) -> bool {
    !line.is_empty()
        && !line.starts_with(char::is_whitespace)
        && !line.starts_with('-')
        && !line.starts_with('#')
        && line.contains(':')
}

fn line_has_value(line: &str, key: &str, expected: &str) -> bool {
    let Some(value) = line
        .trim_start()
        .strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(':'))
    else {
        return false;
    };
    value.trim().trim_matches(['\'', '"']) == expected
}

fn leading_whitespace(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_next_list_entry(line: &str, field_indent: usize) -> bool {
    let trimmed = line.trim_start();
    leading_whitespace(line) < field_indent && (trimmed == "-" || trimmed.starts_with("- "))
}

fn link_system_packages(
    manifest_data: &ManifestData,
    manifest_dirs: &[PathBuf],
    updated_content: &mut String,
) -> io::Result<usize> {
    let ManifestData::System(manifest) = manifest_data else {
        return Ok(0);
    };
    let mut linked_count = 0;

    for pkg in &manifest.packages {
        match find_manifest_for_commit(&pkg.commit, manifest_dirs) {
            Ok(manifest_path) => {
                let revision = repository_revision_for_manifest(&manifest_path)?;
                println!("  {} -> {}", manifest_path.display(), revision);
                linked_count += upsert_manifest_refs_in_section(
                    updated_content,
                    "packages",
                    &pkg.commit,
                    &revision,
                );
            }
            Err(e) => eprintln!("  Warning: {}: {}", pkg.commit, e),
        }
    }
    Ok(linked_count)
}

fn repository_revision_for_manifest(manifest_path: &Path) -> io::Result<String> {
    let repository_root = repository_root_for_path(manifest_path)?;
    ensure_file_matches_head(&repository_root, manifest_path, "Manifest")?;

    let manifest = load_manifest(&manifest_path.to_string_lossy())?;
    let sources = match manifest {
        ManifestData::Package(manifest) => manifest.sources,
        ManifestData::System(manifest) => manifest.sources,
    };
    for source in &sources {
        ensure_source_matches_head(&repository_root, source)?;
    }

    git_output(
        &repository_root,
        &["rev-parse", "--verify", "HEAD^{commit}"],
    )
}

fn ensure_source_matches_head(repository_root: &Path, source: &Source) -> io::Result<()> {
    if source.dev.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Cannot pin source '{}' because dev: reads mutable working-tree content",
                source.name
            ),
        ));
    }

    for reference in [
        source.file.as_deref(),
        source.cargo_lock.as_deref(),
        source.cargo_toml.as_deref(),
        source.go_sum.as_deref(),
        source.zig_zon.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|reference| !is_remote_reference(reference))
    {
        ensure_file_matches_head(repository_root, Path::new(reference), "Source")?;
    }
    Ok(())
}

fn is_remote_reference(reference: &str) -> bool {
    reference.starts_with("http://") || reference.starts_with("https://")
}

fn ensure_file_matches_head(repository_root: &Path, path: &Path, kind: &str) -> io::Result<()> {
    let canonical = path.canonicalize().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to resolve {}: {}", path.display(), error),
        )
    })?;
    if !canonical.is_file() || canonical != path {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} '{}' must be a regular file without symlinks",
                kind,
                path.display()
            ),
        ));
    }
    let relative_path = canonical.strip_prefix(repository_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} '{}' is outside {}",
                kind,
                path.display(),
                repository_root.display()
            ),
        )
    })?;

    let tracked = std::process::Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(relative_path)
        .output()?;
    if !tracked.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} '{}' is not tracked by Git", kind, path.display()),
        ));
    }

    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["status", "--porcelain", "--untracked-files=all", "--"])
        .arg(relative_path)
        .output()?;
    if !status.status.success() {
        return Err(io::Error::other(format!(
            "git status failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&status.stderr)
        )));
    }
    if !status.stdout.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} '{}' differs from HEAD; commit it before linking",
                kind,
                path.display()
            ),
        ));
    }
    Ok(())
}

fn git_output(repository_root: &Path, args: &[&str]) -> io::Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            repository_root.display(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

    ensure_file_matches_head(&repository_root, &env_path, "Environment file")?;
    let relative_path = env_path.strip_prefix(&repository_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is outside {}",
                env_path.display(),
                repository_root.display()
            ),
        )
    })?;
    let object = format!("HEAD:{}", relative_path.to_string_lossy());
    let env_sha = git_output(&repository_root, &["rev-parse", &object])?;
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
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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

#[cfg(test)]
#[path = "link_tests.rs"]
mod link_tests;
