//! Package state tracking for /nex/var/
//!
//! Tracks installed package versions and which version is "current" (has symlinks).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use crate::repo::NexContext;

const NEX_VAR_DIR: &str = "/nex/var";
const STATE_FILE: &str = "/nex/var/installed.json";

/// Information about a single installed version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// when this version was installed (ISO 8601)
    pub installed_at: String,
    /// binaries provided by this package
    pub provides: Vec<String>,
    /// the ref in the package store
    pub store_ref: String,
}

/// State for a single package (can have multiple versions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageState {
    /// map of "version/checksum" -> VersionInfo
    pub versions: HashMap<String, VersionInfo>,
    /// currently active version (has symlinks), format: "version/checksum"
    pub current: Option<String>,
}

/// Global installed packages state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstalledState {
    /// map of "namespace/slug" -> PackageState
    pub packages: HashMap<String, PackageState>,
}

impl InstalledState {
    /// Load state from disk, or return empty state if not found
    pub fn load() -> io::Result<Self> {
        let path = Path::new(STATE_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse state file: {}", e),
            )
        })
    }

    /// Save state to disk
    pub fn save(&self) -> io::Result<()> {
        // ensure directory exists
        fs::create_dir_all(NEX_VAR_DIR)?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::other(format!("Failed to serialize state: {}", e)))?;

        fs::write(STATE_FILE, content)
    }

    /// Load state from a specific var directory
    pub fn load_from(var_path: &Path) -> io::Result<Self> {
        let path = var_path.join("installed.json");
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse state file: {}", e),
            )
        })
    }

    /// Save state to a specific var directory
    pub fn save_to(&self, var_path: &Path) -> io::Result<()> {
        fs::create_dir_all(var_path)?;

        let path = var_path.join("installed.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::other(format!("Failed to serialize state: {}", e)))?;

        fs::write(&path, content)
    }

    /// Load state using NexContext
    pub fn load_for_context(ctx: &NexContext) -> io::Result<Self> {
        Self::load_from(&ctx.var_path)
    }

    /// Save state using NexContext
    pub fn save_for_context(&self, ctx: &NexContext) -> io::Result<()> {
        self.save_to(&ctx.var_path)
    }

    /// Get package state by namespace/slug
    pub fn get_package(&self, namespace: &str, slug: &str) -> Option<&PackageState> {
        let key = format!("{}/{}", namespace, slug);
        self.packages.get(&key)
    }

    /// Get mutable package state, creating if needed
    pub fn get_or_create_package(&mut self, namespace: &str, slug: &str) -> &mut PackageState {
        let key = format!("{}/{}", namespace, slug);
        self.packages.entry(key).or_insert_with(|| PackageState {
            versions: HashMap::new(),
            current: None,
        })
    }

    /// Check if a specific version is installed
    pub fn is_version_installed(
        &self,
        namespace: &str,
        slug: &str,
        version: &str,
        checksum: &str,
    ) -> bool {
        let key = format!("{}/{}", namespace, slug);
        let version_key = format!("{}/{}", version, checksum);

        self.packages
            .get(&key)
            .map(|p| p.versions.contains_key(&version_key))
            .unwrap_or(false)
    }

    /// Get the current version for a package
    pub fn get_current_version(&self, namespace: &str, slug: &str) -> Option<&str> {
        let key = format!("{}/{}", namespace, slug);
        self.packages.get(&key).and_then(|p| p.current.as_deref())
    }

    /// Record a newly installed version
    pub fn record_install(
        &mut self,
        namespace: &str,
        slug: &str,
        version: &str,
        checksum: &str,
        provides: Vec<String>,
        store_ref: &str,
        set_current: bool,
    ) {
        let pkg = self.get_or_create_package(namespace, slug);
        let version_key = format!("{}/{}", version, checksum);

        pkg.versions.insert(
            version_key.clone(),
            VersionInfo {
                installed_at: chrono_now(),
                provides,
                store_ref: store_ref.to_string(),
            },
        );

        if set_current || pkg.current.is_none() {
            pkg.current = Some(version_key);
        }
    }

    /// Remove a version from tracking
    pub fn record_remove(
        &mut self,
        namespace: &str,
        slug: &str,
        version: &str,
        checksum: &str,
    ) -> Option<VersionInfo> {
        let key = format!("{}/{}", namespace, slug);
        let version_key = format!("{}/{}", version, checksum);

        let pkg = self.packages.get_mut(&key)?;
        let removed = pkg.versions.remove(&version_key);

        // if we removed the current version, pick another or clear
        if pkg.current.as_deref() == Some(&version_key) {
            pkg.current = pkg.versions.keys().next().cloned();
        }

        // if no versions left, remove the package entirely
        if pkg.versions.is_empty() {
            self.packages.remove(&key);
        }

        removed
    }

    /// Switch the current version for a package
    pub fn switch_current(
        &mut self,
        namespace: &str,
        slug: &str,
        version: &str,
        checksum: &str,
    ) -> io::Result<()> {
        let key = format!("{}/{}", namespace, slug);
        let version_key = format!("{}/{}", version, checksum);

        let pkg = self.packages.get_mut(&key).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Package {} not installed", key),
            )
        })?;

        if !pkg.versions.contains_key(&version_key) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Version {} not installed for {}", version_key, key),
            ));
        }

        pkg.current = Some(version_key);
        Ok(())
    }

    /// List all installed packages
    pub fn list_packages(&self) -> Vec<(&str, &PackageState)> {
        self.packages.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    /// Get count of installed versions for a package
    pub fn version_count(&self, namespace: &str, slug: &str) -> usize {
        let key = format!("{}/{}", namespace, slug);
        self.packages
            .get(&key)
            .map(|p| p.versions.len())
            .unwrap_or(0)
    }
}

/// Simple timestamp function (avoid chrono dependency)
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_install() {
        let mut state = InstalledState::default();

        state.record_install(
            "cli/shells",
            "bash",
            "5.2.21",
            "a19536f4",
            vec!["bash".to_string(), "sh".to_string()],
            "x86_64/pkg/cli/shells/bash/5.2.21/deploy/a19536f4",
            true,
        );

        assert!(state.is_version_installed("cli/shells", "bash", "5.2.21", "a19536f4"));
        assert_eq!(
            state.get_current_version("cli/shells", "bash"),
            Some("5.2.21/a19536f4")
        );
        assert_eq!(state.version_count("cli/shells", "bash"), 1);
    }

    #[test]
    fn test_multi_version() {
        let mut state = InstalledState::default();

        // install first version
        state.record_install(
            "cli/shells",
            "bash",
            "5.2.21",
            "a19536f4",
            vec!["bash".to_string()],
            "ref1",
            true,
        );

        // install second version (don't set as current)
        state.record_install(
            "cli/shells",
            "bash",
            "5.3.0",
            "b2c3d4e5",
            vec!["bash".to_string()],
            "ref2",
            false,
        );

        assert_eq!(state.version_count("cli/shells", "bash"), 2);
        // current should still be the first version
        assert_eq!(
            state.get_current_version("cli/shells", "bash"),
            Some("5.2.21/a19536f4")
        );
    }
}
