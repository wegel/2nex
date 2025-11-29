use std::io;
use std::path::Path;
use std::process::Command;

/// Check if an OSTree branch exists in the repository
pub fn ensure_branch_exists(repo_path: &str, branch: &str) -> io::Result<()> {
    let output = Command::new("ostree")
        .arg("rev-parse")
        .arg("--repo")
        .arg(repo_path)
        .arg(branch)
        .output()
        .map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to run ostree: {}", e))
        })?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Branch {} missing in {}: {}",
                branch,
                repo_path,
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(())
}

/// Get the commit ID (hash) for a branch
pub fn get_commit_id(repo_path: &str, branch: &str) -> io::Result<String> {
    let output = Command::new("ostree")
        .arg("rev-parse")
        .arg("--repo")
        .arg(repo_path)
        .arg(branch)
        .output()
        .map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to run ostree: {}", e))
        })?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Branch {} not found", branch),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get a metadata value from an OSTree commit
pub fn get_commit_metadata(repo_path: &str, commit: &str, key: &str) -> io::Result<String> {
    let output = Command::new("ostree")
        .arg("show")
        .arg("--repo")
        .arg(repo_path)
        .arg(format!("--print-metadata-key={}", key))
        .arg(commit)
        .output()
        .map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to run ostree: {}", e))
        })?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Metadata key {} not found for {}", key, commit),
        ));
    }

    // ostree returns quoted strings, remove the quotes
    let value = String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_matches('\'')
        .to_string();

    Ok(value)
}

/// Parse an OSTree commit reference into (slug, version, namespace)
pub fn parse_commit_ref(commit: &str) -> Option<(String, String, String)> {
    // format: x86_64/{namespace}/{slug}/{version}/...
    let parts: Vec<&str> = commit.split('/').collect();
    if parts.len() < 4 {
        return None;
    }

    let boundary = parts
        .iter()
        .position(|part| *part == "outputs" || *part == "bundles")
        .unwrap_or(parts.len());

    if boundary < 3 {
        return None;
    }

    let slug_idx = boundary.saturating_sub(2);
    let version_idx = boundary.saturating_sub(1);
    if slug_idx >= parts.len() || version_idx >= parts.len() || slug_idx < 1 {
        return None;
    }

    let slug = parts[slug_idx].to_string();
    let version = parts[version_idx].to_string();
    let namespace = parts[1..slug_idx].join("/");
    Some((slug, version, namespace))
}

/// Checkout an OSTree commit into a directory
pub fn checkout_ostree_into(
    repo_path: &str,
    commit: &str,
    dest: &Path,
    union: bool,
    allow_noent: bool,
) -> io::Result<()> {
    println!(
        "Checking out OSTree commit {} into {} (union: {})",
        commit,
        dest.display(),
        union
    );

    let mut command = Command::new("unshare");
    command.args(&["--map-root-user", "--user", "--"]);
    command.arg("ostree");
    command.arg("checkout");
    command.arg("--repo").arg(repo_path);
    // prevent libostree from trying to interact with host's sysroot on ostree-deployed systems
    command.env("OSTREE_SYSROOT", "/nonexistent");
    if union {
        command.arg("--union");
    }
    if allow_noent {
        command.arg("--allow-noent");
    }
    command.arg(commit);
    command.arg(dest);

    let output = command.output().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to run ostree checkout: {}", e),
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to checkout {}: {}", commit, stderr),
        ));
    }

    Ok(())
}

/// Commit a directory to OSTree
pub fn commit_to_ostree(
    repo_path: &str,
    branch: &str,
    tree_path: &Path,
    metadata: &[(String, String)],
) -> io::Result<()> {
    println!(
        "Committing {} to OSTree branch {}",
        tree_path.display(),
        branch
    );

    let mut command = Command::new("unshare");
    command.args(&["--map-root-user", "--user", "--"]);
    command.arg("ostree");
    command.arg("commit");
    command.arg("--repo").arg(repo_path);
    command.arg("--branch").arg(branch);
    command.arg("--no-xattrs");
    command.arg("--no-bindings");
    // prevent libostree from trying to interact with host's sysroot on ostree-deployed systems
    command.env("OSTREE_SYSROOT", "/nonexistent");

    for (key, value) in metadata {
        command.arg("--add-metadata-string");
        command.arg(format!("{}={}", key, value));
    }

    command.arg(tree_path.to_str().unwrap());

    let output = command.output().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to run ostree commit: {}", e),
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to commit to {}: {}", branch, stderr),
        ));
    }

    Ok(())
}

/// Rewrite OSTree branch metadata without changing the tree
pub fn rewrite_branch_metadata(
    repo_path: &str,
    branch: &str,
    metadata: &[(String, String)],
) -> io::Result<()> {
    println!("Rewriting metadata for {}", branch);
    let mut command = Command::new("unshare");
    command.args(&["--map-root-user", "--user", "--"]);
    command.arg("ostree");
    command.arg("commit");
    command.arg("--repo").arg(repo_path);
    command.arg("--branch").arg(branch);
    command.arg(format!("--tree=ref={}", branch));

    for (key, value) in metadata {
        command.arg(format!("--add-metadata-string={}={}", key, value));
    }

    let output = command.output().map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Failed to run ostree: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to rewrite metadata for {}: {}", branch, stderr),
        ));
    }

    Ok(())
}

/// Get metadata value from an OSTree branch
pub fn get_branch_metadata(repo_path: &str, branch: &str, key: &str) -> io::Result<String> {
    let output = Command::new("ostree")
        .arg("show")
        .arg("--repo")
        .arg(repo_path)
        .arg("--print-metadata-key")
        .arg(key)
        .arg(branch)
        .output()
        .map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to run ostree: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to get metadata {}: {}", key, stderr),
        ));
    }

    let value = String::from_utf8_lossy(&output.stdout);
    // ostree outputs the value with quotes and newline, strip them
    Ok(value.trim().trim_matches('\'').to_string())
}

/// Encode a list of strings as JSON for OSTree metadata
pub fn encode_metadata_list(values: &[String]) -> io::Result<Option<String>> {
    if values.is_empty() {
        return Ok(None);
    }
    let json = serde_json::to_string(values).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to encode metadata: {}", e),
        )
    })?;
    Ok(Some(json))
}

/// Read build checksum from an OSTree commit
pub fn read_checksum_from_commit(repo_path: &str, commit: &str) -> io::Result<String> {
    let output = Command::new("ostree")
        .arg("show")
        .arg("--repo")
        .arg(repo_path)
        .arg("--print-metadata-key")
        .arg("nex.build.checksum")
        .arg(commit)
        .output()
        .map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to run ostree: {}", e))
        })?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Checksum metadata not found",
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let trimmed = text.trim();

    // ostree wraps strings in single quotes, strip them
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        Ok(trimmed[1..trimmed.len() - 1].to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

/// Find a commit in OSTree history by its manifest hash.
/// Traverses the commit history of a branch looking for a commit with matching
/// `nex.manifest.hash` metadata.
pub fn find_commit_by_manifest_hash(
    repo_path: &str,
    branch: &str,
    target_hash: &str,
) -> io::Result<Option<String>> {
    // get the commit history for the branch
    let output = Command::new("ostree")
        .arg("log")
        .arg("--repo")
        .arg(repo_path)
        .arg(branch)
        .output()
        .map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("failed to run ostree log: {}", e))
        })?;

    if !output.status.success() {
        // branch doesn't exist
        return Ok(None);
    }

    let log_output = String::from_utf8_lossy(&output.stdout);

    // parse commit IDs from log output
    // ostree log format: "commit <commit_id>"
    let commits: Vec<&str> = log_output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("commit ") {
                Some(trimmed.strip_prefix("commit ")?.trim())
            } else {
                None
            }
        })
        .collect();

    // check each commit's manifest hash
    for commit_id in commits {
        match get_branch_metadata(repo_path, commit_id, "nex.manifest.hash") {
            Ok(hash) if hash == target_hash => {
                return Ok(Some(commit_id.to_string()));
            }
            _ => continue,
        }
    }

    Ok(None)
}
