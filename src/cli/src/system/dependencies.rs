//! System package dependency conversion helpers.

use crate::manifest::{Dependency, SystemPackage};

/// Convert system package entries into dependency refs for closure resolution.
pub fn dependencies_from_system_packages(packages: &[SystemPackage]) -> Vec<Dependency> {
    packages
        .iter()
        .map(|pkg| Dependency {
            commit: pkg.commit.clone(),
            name: pkg.name.clone(),
            manifest_ref: None,
        })
        .collect()
}
