//! package reference parsing and manipulation.
//!
//! refs follow the format: `{arch}/pkg/{namespace}/{slug}/{version}/{type}[/{name_or_path}]`
//! where type is one of: `outputs`, `bundles`, `files` (the anchor keyword).

use std::fmt;

/// parsed package reference.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageRef {
    pub arch: String,
    pub namespace: String,
    pub slug: String,
    pub version: String,
    pub ref_type: RefType,
}

/// the type component of a package reference.
#[derive(Debug, Clone, PartialEq)]
pub enum RefType {
    /// a specific output (e.g., `outputs/bin` or `outputs/bin/usr/bin/foo`)
    Output { name: String, path: Option<String> },
    /// a bundle of outputs (e.g., `bundles/full` or `bundles/full/usr/bin/foo`)
    Bundle { name: String, path: Option<String> },
    /// the files tree, optionally with a path (e.g., `files` or `files/usr/bin/foo`)
    Files { path: Option<String> },
}

impl PackageRef {
    /// parse a ref string into a PackageRef.
    ///
    /// uses `outputs`, `bundles`, `files` as anchor keywords to determine
    /// where namespace ends and version/slug begin.
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('/').collect();

        // find the anchor keyword (outputs, bundles, files)
        let anchor_idx = parts
            .iter()
            .position(|&p| p == "outputs" || p == "bundles" || p == "files")
            .ok_or_else(|| {
                format!(
                    "no anchor keyword (outputs/bundles/files) found in ref: {}",
                    s
                )
            })?;

        // need at least: arch/pkg/namespace.../slug/version/anchor
        // minimum: x86_64/pkg/ns/slug/ver/anchor = 6 parts, anchor at index 5
        if anchor_idx < 5 {
            return Err(format!(
                "ref too short, expected at least arch/pkg/namespace/slug/version/anchor: {}",
                s
            ));
        }

        // validate structure
        if parts.get(1) != Some(&"pkg") {
            return Err(format!(
                "expected 'pkg' at position 1, got: {:?}",
                parts.get(1)
            ));
        }

        let arch = parts[0].to_string();
        let version = parts[anchor_idx - 1].to_string();
        let slug = parts[anchor_idx - 2].to_string();
        let namespace = parts[2..anchor_idx - 2].join("/");

        if namespace.is_empty() {
            return Err(format!("namespace cannot be empty: {}", s));
        }

        let ref_type = match parts[anchor_idx] {
            "outputs" => {
                let name = parts
                    .get(anchor_idx + 1)
                    .ok_or_else(|| format!("outputs requires a name: {}", s))?
                    .to_string();
                let path = if anchor_idx + 2 < parts.len() {
                    Some(parts[anchor_idx + 2..].join("/"))
                } else {
                    None
                };
                RefType::Output { name, path }
            }
            "bundles" => {
                let name = parts
                    .get(anchor_idx + 1)
                    .ok_or_else(|| format!("bundles requires a name: {}", s))?
                    .to_string();
                let path = if anchor_idx + 2 < parts.len() {
                    Some(parts[anchor_idx + 2..].join("/"))
                } else {
                    None
                };
                RefType::Bundle { name, path }
            }
            "files" => {
                let path = if anchor_idx + 1 < parts.len() {
                    Some(parts[anchor_idx + 1..].join("/"))
                } else {
                    None
                };
                RefType::Files { path }
            }
            _ => unreachable!(),
        };

        Ok(PackageRef {
            arch,
            namespace,
            slug,
            version,
            ref_type,
        })
    }

    /// convert back to a ref string.
    pub fn to_ref_string(&self) -> String {
        let base = format!(
            "{}/pkg/{}/{}/{}",
            self.arch, self.namespace, self.slug, self.version
        );

        match &self.ref_type {
            RefType::Output { name, path: None } => format!("{}/outputs/{}", base, name),
            RefType::Output { name, path: Some(p) } => format!("{}/outputs/{}/{}", base, name, p),
            RefType::Bundle { name, path: None } => format!("{}/bundles/{}", base, name),
            RefType::Bundle { name, path: Some(p) } => format!("{}/bundles/{}/{}", base, name, p),
            RefType::Files { path: None } => format!("{}/files", base),
            RefType::Files { path: Some(p) } => format!("{}/files/{}", base, p),
        }
    }

    /// get the commit ref (without internal path).
    pub fn commit_ref(&self) -> String {
        let base = format!(
            "{}/pkg/{}/{}/{}",
            self.arch, self.namespace, self.slug, self.version
        );

        match &self.ref_type {
            RefType::Output { name, .. } => format!("{}/outputs/{}", base, name),
            RefType::Bundle { name, .. } => format!("{}/bundles/{}", base, name),
            RefType::Files { .. } => format!("{}/files", base),
        }
    }

    /// get the internal path (if any).
    pub fn internal_path(&self) -> Option<&str> {
        match &self.ref_type {
            RefType::Output { path, .. } => path.as_deref(),
            RefType::Bundle { path, .. } => path.as_deref(),
            RefType::Files { path } => path.as_deref(),
        }
    }

    /// get the relative manifest path (e.g., "libs/compression/bzip2.yaml").
    pub fn manifest_rel_path(&self) -> String {
        format!("{}/{}.yaml", self.namespace, self.slug)
    }

    /// get the output or bundle name, if applicable.
    pub fn output_name(&self) -> Option<&str> {
        match &self.ref_type {
            RefType::Output { name, .. } => Some(name),
            RefType::Bundle { name, .. } => Some(name),
            RefType::Files { .. } => None,
        }
    }

    /// check if this is a files ref.
    pub fn is_files(&self) -> bool {
        matches!(self.ref_type, RefType::Files { .. })
    }

    /// check if this ref has an internal path (needs partial extraction).
    pub fn has_internal_path(&self) -> bool {
        self.internal_path().is_some()
    }
}

impl fmt::Display for PackageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_ref_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_output_ref() {
        let r = PackageRef::parse("x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/bin").unwrap();
        assert_eq!(r.arch, "x86_64");
        assert_eq!(r.namespace, "libs/compression");
        assert_eq!(r.slug, "bzip2");
        assert_eq!(r.version, "1.0.8");
        assert_eq!(
            r.ref_type,
            RefType::Output {
                name: "bin".to_string(),
                path: None
            }
        );
        assert_eq!(r.commit_ref(), "x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/bin");
        assert_eq!(r.internal_path(), None);
    }

    #[test]
    fn test_parse_output_with_path() {
        let r = PackageRef::parse("x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/bin/usr/bin/bzip2").unwrap();
        assert_eq!(
            r.ref_type,
            RefType::Output {
                name: "bin".to_string(),
                path: Some("usr/bin/bzip2".to_string())
            }
        );
        assert_eq!(r.commit_ref(), "x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/bin");
        assert_eq!(r.internal_path(), Some("usr/bin/bzip2"));
    }

    #[test]
    fn test_parse_bundle_ref() {
        let r = PackageRef::parse("x86_64/pkg/cli/editors/neovim/0.11.0/bundles/full").unwrap();
        assert_eq!(r.namespace, "cli/editors");
        assert_eq!(r.slug, "neovim");
        assert_eq!(r.version, "0.11.0");
        assert_eq!(
            r.ref_type,
            RefType::Bundle {
                name: "full".to_string(),
                path: None
            }
        );
    }

    #[test]
    fn test_parse_bundle_with_path() {
        let r = PackageRef::parse("x86_64/pkg/cli/editors/neovim/0.11.0/bundles/full/usr/bin/nvim").unwrap();
        assert_eq!(
            r.ref_type,
            RefType::Bundle {
                name: "full".to_string(),
                path: Some("usr/bin/nvim".to_string())
            }
        );
        assert_eq!(r.commit_ref(), "x86_64/pkg/cli/editors/neovim/0.11.0/bundles/full");
        assert_eq!(r.internal_path(), Some("usr/bin/nvim"));
    }

    #[test]
    fn test_parse_files_ref() {
        let r = PackageRef::parse("x86_64/pkg/libs/compression/bzip2/1.0.8/files").unwrap();
        assert_eq!(r.ref_type, RefType::Files { path: None });
        assert_eq!(r.commit_ref(), "x86_64/pkg/libs/compression/bzip2/1.0.8/files");
    }

    #[test]
    fn test_parse_files_with_path() {
        let r = PackageRef::parse("x86_64/pkg/libs/compression/bzip2/1.0.8/files/usr/bin/bzip2")
            .unwrap();
        assert_eq!(
            r.ref_type,
            RefType::Files {
                path: Some("usr/bin/bzip2".to_string())
            }
        );
        assert_eq!(r.commit_ref(), "x86_64/pkg/libs/compression/bzip2/1.0.8/files");
        assert_eq!(r.internal_path(), Some("usr/bin/bzip2"));
    }

    #[test]
    fn test_parse_deep_namespace() {
        let r =
            PackageRef::parse("x86_64/pkg/core/kernel/drivers/nvidia/535.154/outputs/bin").unwrap();
        assert_eq!(r.namespace, "core/kernel/drivers");
        assert_eq!(r.slug, "nvidia");
        assert_eq!(r.version, "535.154");
    }

    #[test]
    fn test_roundtrip() {
        let refs = [
            "x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/bin",
            "x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/bin/usr/bin/bzip2",
            "x86_64/pkg/cli/editors/neovim/0.11.0/bundles/full",
            "x86_64/pkg/cli/editors/neovim/0.11.0/bundles/full/usr/bin/nvim",
            "x86_64/pkg/libs/system/glibc/2.39/files",
            "x86_64/pkg/libs/system/glibc/2.39/files/usr/lib/libc.so.6",
        ];
        for s in refs {
            let r = PackageRef::parse(s).unwrap();
            assert_eq!(r.to_ref_string(), s);
        }
    }

    #[test]
    fn test_manifest_rel_path() {
        let r = PackageRef::parse("x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/bin").unwrap();
        assert_eq!(r.manifest_rel_path(), "libs/compression/bzip2.yaml");
    }

    #[test]
    fn test_error_no_anchor() {
        let err = PackageRef::parse("x86_64/pkg/libs/compression/bzip2/1.0.8").unwrap_err();
        assert!(err.contains("no anchor keyword"));
    }

    #[test]
    fn test_error_too_short() {
        let err = PackageRef::parse("x86_64/pkg/foo/1.0/outputs/bin").unwrap_err();
        assert!(err.contains("too short"));
    }

    #[test]
    fn test_error_missing_output_name() {
        let err = PackageRef::parse("x86_64/pkg/libs/compression/bzip2/1.0.8/outputs").unwrap_err();
        assert!(err.contains("requires a name"));
    }
}
