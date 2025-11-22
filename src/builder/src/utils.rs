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
