use std::io;
use std::path::Path;
use std::process::Command;

/// Classifies an output path into the manifest output category the builder
/// expects. The logic matches the manifest schema so both the runtime scanner
/// and the manifest pretty-printer can share the same categorization rules.
pub fn determine_category(file_path: &str) -> String {
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

/// Extracts the manifest prefix (namespace/slug/version) from a bundle or output
/// branch reference. This is used by the runtime scanner when it has to resolve
/// canonical dependency paths from arbitrary OSTree refs.
pub fn manifest_prefix(commit: &str) -> Option<String> {
    if let Some(idx) = commit.find("/bundles/") {
        Some(commit[..idx].to_string())
    } else if let Some(idx) = commit.find("/outputs/") {
        Some(commit[..idx].to_string())
    } else {
        None
    }
}

/// Infer the manifest file path from an OSTree commit reference.
/// Example: "x86_64/pkg/libs/openssl/3.0/bundles/dev" -> "pkg/libs/openssl.yaml"
pub fn infer_manifest_path(commit_ref: &str) -> Option<String> {
    let parts: Vec<&str> = commit_ref.split('/').collect();
    if parts.len() < 4 {
        return None;
    }

    // find the boundary (outputs or bundles)
    let boundary = parts
        .iter()
        .position(|part| *part == "outputs" || *part == "bundles")
        .unwrap_or(parts.len());

    if boundary < 3 {
        return None;
    }

    // extract namespace and slug
    // format: arch/{namespace...}/{slug}/{version}/...
    let slug_idx = boundary.saturating_sub(2);
    if slug_idx < 1 {
        return None;
    }

    let namespace = parts[1..slug_idx].join("/");
    let slug = parts[slug_idx];

    // convert slug with hyphens to underscores for filename matching
    let slug_underscore = slug.replace("-", "_");

    Some(format!("{}/{}.yaml", namespace, slug_underscore))
}

/// Fetch content from a git blob by its SHA.
pub fn fetch_git_blob(repo_root: &Path, sha: &str) -> io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("cat-file")
        .arg("-p")
        .arg(sha)
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("failed to fetch git blob {}: {}", sha, stderr),
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid UTF-8: {}", e)))
}

/// Calculate the git blob SHA of a file's content.
pub fn hash_file_content(path: &Path) -> io::Result<String> {
    let output = Command::new("git")
        .arg("hash-object")
        .arg(path)
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("failed to hash file {}: {}", path.display(), stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determine_category_handles_common_layouts() {
        assert_eq!(determine_category("/usr/lib/libfoo.so"), "lib");
        assert_eq!(determine_category("/usr/include/foo.h"), "dev");
        assert_eq!(determine_category("/etc/foo.conf"), "conf");
        assert_eq!(determine_category("/usr/share/doc/foo/readme"), "doc");
        assert_eq!(determine_category("/usr/bin/foo"), "bin");
    }

    #[test]
    fn manifest_prefix_extracts_base_path() {
        assert_eq!(
            manifest_prefix("x86_64/pkg/base/foo/1.0/bundles/dev"),
            Some("x86_64/pkg/base/foo/1.0".to_string())
        );
        assert_eq!(
            manifest_prefix("x86_64/pkg/base/foo/1.0/outputs/lib"),
            Some("x86_64/pkg/base/foo/1.0".to_string())
        );
        assert_eq!(manifest_prefix("x86_64/pkg/base/foo/1.0"), None);
    }
}
