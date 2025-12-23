use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use super::parser::detect_manifest_kind;
use super::types::*;

/// resolve inheritance chain, loading and merging parent manifests
pub fn resolve_inheritance(manifest_path: &Path) -> io::Result<SystemManifest> {
    let canonical = manifest_path.canonicalize().map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("failed to resolve path {}: {}", manifest_path.display(), e),
        )
    })?;
    let mut visited = HashSet::new();
    resolve_inheritance_chain(&canonical, &mut visited)
}

fn resolve_inheritance_chain(
    manifest_path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> io::Result<SystemManifest> {
    // detect circular inheritance
    if visited.contains(manifest_path) {
        let chain: Vec<_> = visited.iter().map(|p| p.display().to_string()).collect();
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "circular inheritance detected: {} -> {}",
                chain.join(" -> "),
                manifest_path.display()
            ),
        ));
    }
    visited.insert(manifest_path.to_path_buf());

    let manifest = load_raw_system_manifest(manifest_path)?;

    match &manifest.system.extends {
        Some(extends_path) => {
            // resolve relative to repo root (current working directory)
            let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let base_path = repo_root.join(extends_path);
            let base_canonical = base_path.canonicalize().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "base manifest not found: {} (from {}): {}",
                        extends_path.display(),
                        manifest_path.display(),
                        e
                    ),
                )
            })?;
            let base = resolve_inheritance_chain(&base_canonical, visited)?;
            Ok(merge_manifests(&base, &manifest))
        }
        None => Ok(manifest),
    }
}

/// load a system manifest without resolving inheritance or validating
/// (validation happens after merge in resolve_inheritance)
fn load_raw_system_manifest(path: &Path) -> io::Result<SystemManifest> {
    let content = std::fs::read_to_string(path)?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if detect_manifest_kind(&doc) != ManifestKind::System {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is a package manifest, not a system manifest", path.display()),
        ));
    }

    let sys: SystemManifest = serde_yaml::from_value(doc)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(sys)
}

/// merge parent and child manifests
fn merge_manifests(base: &SystemManifest, child: &SystemManifest) -> SystemManifest {
    let excludes = child.exclude.clone().unwrap_or_default();

    SystemManifest {
        schema: child.schema.or(base.schema),
        system: merge_meta(&base.system, &child.system),
        packages: merge_packages(&base.packages, &child.packages, &excludes.packages),
        dependencies: merge_deps(&base.dependencies, &child.dependencies, &excludes.dependencies),
        sources: merge_sources(&base.sources, &child.sources),
        overlays: [base.overlays.clone(), child.overlays.clone()].concat(),
        build: merge_build(&base.build, &child.build),
        exclude: None, // doesn't propagate
    }
}

/// merge system metadata - child values override parent
fn merge_meta(base: &SystemMeta, child: &SystemMeta) -> SystemMeta {
    SystemMeta {
        name: child.name.clone(),
        slug: child.slug.clone(),
        version: child.version.clone(),
        architecture: child.architecture.clone().or_else(|| base.architecture.clone()),
        boot_method: child.boot_method.clone().or_else(|| base.boot_method.clone()),
        description: child.description.clone().or_else(|| base.description.clone()),
        checksum: None, // will be recomputed
        stable_checksum: child.stable_checksum.or(base.stable_checksum),
        nex_structure: child.nex_structure || base.nex_structure,
        extends: None, // doesn't propagate after merge
    }
}

/// merge packages - child appends to parent, same-name overrides
fn merge_packages(
    base: &[SystemPackage],
    child: &[SystemPackage],
    excludes: &[ExcludeSpec],
) -> Vec<SystemPackage> {
    let mut result: Vec<SystemPackage> = base
        .iter()
        .filter(|pkg| !should_exclude_package(pkg, excludes))
        .cloned()
        .collect();

    for child_pkg in child {
        // check if this overrides an existing package by name
        if let Some(name) = &child_pkg.name {
            if let Some(pos) = result.iter().position(|p| p.name.as_ref() == Some(name)) {
                result[pos] = child_pkg.clone();
                continue;
            }
        }
        result.push(child_pkg.clone());
    }

    result
}

/// check if a package should be excluded
fn should_exclude_package(pkg: &SystemPackage, excludes: &[ExcludeSpec]) -> bool {
    for spec in excludes {
        match spec {
            ExcludeSpec::ByName { name } => {
                if pkg.name.as_ref() == Some(name) {
                    return true;
                }
            }
            ExcludeSpec::ByCommit { commit } => {
                if &pkg.commit == commit {
                    return true;
                }
            }
        }
    }
    false
}

/// merge dependencies - child appends to parent, same-name overrides
fn merge_deps(
    base: &[Dependency],
    child: &[Dependency],
    excludes: &[ExcludeSpec],
) -> Vec<Dependency> {
    let mut result: Vec<Dependency> = base
        .iter()
        .filter(|dep| !should_exclude_dep(dep, excludes))
        .cloned()
        .collect();

    for child_dep in child {
        // check if this overrides an existing dependency by name
        if let Some(name) = &child_dep.name {
            if let Some(pos) = result.iter().position(|d| d.name.as_ref() == Some(name)) {
                result[pos] = child_dep.clone();
                continue;
            }
        }
        result.push(child_dep.clone());
    }

    result
}

/// check if a dependency should be excluded
fn should_exclude_dep(dep: &Dependency, excludes: &[ExcludeSpec]) -> bool {
    for spec in excludes {
        match spec {
            ExcludeSpec::ByName { name } => {
                if dep.name.as_ref() == Some(name) {
                    return true;
                }
            }
            ExcludeSpec::ByCommit { commit } => {
                if &dep.commit == commit {
                    return true;
                }
            }
        }
    }
    false
}

/// merge sources - child appends to parent, same-name overrides
fn merge_sources(base: &[Source], child: &[Source]) -> Vec<Source> {
    let mut result = base.to_vec();
    for child_src in child {
        if let Some(pos) = result.iter().position(|s| s.name == child_src.name) {
            result[pos] = child_src.clone();
        } else {
            result.push(child_src.clone());
        }
    }
    result
}

/// merge build config - scripts concatenate
fn merge_build(base: &Build, child: &Build) -> Build {
    let merged_script = if child.script.trim().is_empty() {
        base.script.clone()
    } else if base.script.trim().is_empty() {
        child.script.clone()
    } else {
        format!("{}\n\n# === extended assembly ===\n{}", base.script, child.script)
    };

    Build {
        environment: child.environment.clone(),
        script: merged_script,
        profile: if child.profile.is_empty() {
            base.profile.clone()
        } else {
            child.profile.clone()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_exclude_package_by_name() {
        let pkg = SystemPackage {
            commit: "x86_64/pkg/foo/1.0/outputs/bin".to_string(),
            name: Some("foo".to_string()),
        };
        let excludes = vec![ExcludeSpec::ByName {
            name: "foo".to_string(),
        }];
        assert!(should_exclude_package(&pkg, &excludes));
    }

    #[test]
    fn test_should_exclude_package_by_commit() {
        let pkg = SystemPackage {
            commit: "x86_64/pkg/foo/1.0/outputs/bin".to_string(),
            name: Some("foo".to_string()),
        };
        let excludes = vec![ExcludeSpec::ByCommit {
            commit: "x86_64/pkg/foo/1.0/outputs/bin".to_string(),
        }];
        assert!(should_exclude_package(&pkg, &excludes));
    }

    #[test]
    fn test_should_not_exclude_package() {
        let pkg = SystemPackage {
            commit: "x86_64/pkg/foo/1.0/outputs/bin".to_string(),
            name: Some("foo".to_string()),
        };
        let excludes = vec![ExcludeSpec::ByName {
            name: "bar".to_string(),
        }];
        assert!(!should_exclude_package(&pkg, &excludes));
    }

    #[test]
    fn test_merge_packages_override() {
        let base = vec![SystemPackage {
            commit: "old-commit".to_string(),
            name: Some("foo".to_string()),
        }];
        let child = vec![SystemPackage {
            commit: "new-commit".to_string(),
            name: Some("foo".to_string()),
        }];
        let result = merge_packages(&base, &child, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].commit, "new-commit");
    }

    #[test]
    fn test_merge_packages_append() {
        let base = vec![SystemPackage {
            commit: "foo-commit".to_string(),
            name: Some("foo".to_string()),
        }];
        let child = vec![SystemPackage {
            commit: "bar-commit".to_string(),
            name: Some("bar".to_string()),
        }];
        let result = merge_packages(&base, &child, &[]);
        assert_eq!(result.len(), 2);
    }
}
