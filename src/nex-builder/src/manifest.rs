//! Package and build-environment data read from YAML and Git.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::reference::InputRef;
use crate::schema::{
    invalid, invalid_data, manifest_error, validate_hash, validate_name, validate_version,
    SchemaVersion,
};
use crate::Result as BuildResult;

// --- Public package data ---

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageManifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub sources: Vec<Source>,
    pub build: Build,
    #[serde(default)]
    pub outputs: BTreeMap<String, Output>,
    #[serde(default)]
    pub bundles: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub resolution: BTreeMap<String, ResolutionTarget>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Package {
    #[serde(rename = "schema", skip_serializing)]
    _schema: SchemaVersion,
    #[serde(rename = "name", skip_serializing)]
    _name: String,
    pub slug: String,
    pub namespace: String,
    pub version: String,
    #[serde(rename = "description", skip_serializing)]
    _description: String,
    #[serde(default, rename = "homepage", skip_serializing)]
    _homepage: Option<String>,
    #[serde(skip_serializing)]
    pub checksum: Option<String>,
    #[serde(default, rename = "seed", skip_serializing)]
    _seed: bool,
    #[serde(default, rename = "stable_checksum", skip_serializing)]
    _stable_checksum: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Dependency {
    #[serde(skip_serializing)]
    pub name: Option<String>,
    #[serde(skip_serializing)]
    pub commit: InputRef,
    #[serde(default)]
    pub paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Source {
    pub name: String,
    pub sha256: String,
    #[serde(flatten)]
    pub kind: SourceSpec,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum SourceSpec {
    Url { url: String },
    File { file: PathBuf },
    CargoLock { cargo_lock: String },
    GoSum { go_sum: String },
    ZigZon { zig_zon: String },
    GitBundle { git_bundle: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Build {
    pub environment: String,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub profile: Vec<String>,
    pub script: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Output {
    #[serde(default)]
    pub files: Vec<FileEntry>,
    #[serde(default, rename = "provides", skip_serializing)]
    _provides: Vec<String>,
    #[serde(default, rename = "capability_files", skip_serializing)]
    _capability_files: BTreeMap<String, Vec<PathBuf>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileEntry {
    pub path: String,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub needs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum ResolutionTarget {
    Dependency(String),
    Capability {
        capability: String,
        fallback: Option<String>,
    },
}

// --- Build environment ---

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct BuildEnvironment {
    pub name: String,
    pub execution: Execution,
    pub paths: BuildPaths,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub preamble: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Execution {
    #[serde(default)]
    pub chroot: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct BuildPaths {
    pub work: PathBuf,
    pub out: PathBuf,
    pub inputs: PathBuf,
}

// --- Loading and checks ---

pub(crate) fn load_manifest(path: &Path) -> BuildResult<PackageManifest> {
    let bytes = fs::read(path)?;
    let manifest: PackageManifest = serde_yaml::from_slice(&bytes)
        .map_err(|source| manifest_error(path, &bytes, "package", source))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub(crate) fn load_environment(path: &Path, reference: &str) -> io::Result<BuildEnvironment> {
    let bytes = if let Some(file) = environment_file(path, reference) {
        fs::read(file)?
    } else {
        read_git_blob(path, reference)?
    };
    let environment = serde_yaml::from_slice(&bytes).map_err(invalid_data)?;
    validate_environment(&environment)?;
    Ok(environment)
}

pub(crate) fn validate_manifest(manifest: &PackageManifest) -> io::Result<()> {
    if manifest.outputs.is_empty() {
        return Err(invalid("package declares no outputs"));
    }
    validate_package(&manifest.package)?;
    validate_dependencies(&manifest.dependencies)?;
    validate_sources(&manifest.sources)?;
    validate_outputs(&manifest.outputs)?;
    validate_bundles(&manifest.outputs, &manifest.bundles)?;
    crate::runtime::validate(manifest)
}

pub(crate) fn validate_dependencies(dependencies: &[Dependency]) -> io::Result<()> {
    let mut names = BTreeMap::new();
    for dependency in dependencies {
        if let Some(name) = &dependency.name {
            validate_name(name, "dependency")?;
            if let Some(previous) = names.insert(name, dependency) {
                if previous.commit != dependency.commit || previous.paths != dependency.paths {
                    return Err(invalid(format!(
                        "dependency name {name} refers to more than one input"
                    )));
                }
            }
        }
        for path in &dependency.paths {
            let value = path.to_str().ok_or_else(|| {
                invalid(format!("dependency path is not UTF-8: {}", path.display()))
            })?;
            validate_output_path(value)?;
        }
        let mut paths = dependency.paths.iter().collect::<Vec<_>>();
        paths.sort_unstable();
        for pair in paths.windows(2) {
            if pair[0] == pair[1] {
                return Err(invalid(format!(
                    "dependency path appears more than once: {}",
                    pair[0].display()
                )));
            }
            if pair[1].starts_with(pair[0]) {
                return Err(invalid(format!(
                    "dependency paths overlap: {} and {}",
                    pair[0].display(),
                    pair[1].display()
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_sources(sources: &[Source]) -> io::Result<()> {
    let mut source_names = std::collections::BTreeSet::new();
    for source in sources {
        validate_hash(&source.sha256, "source checksum")?;
        validate_name(&source.name, "source")?;
        if !source_names.insert(&source.name) {
            return Err(invalid(format!(
                "source name appears more than once: {}",
                source.name
            )));
        }
    }
    Ok(())
}

fn validate_outputs(outputs: &BTreeMap<String, Output>) -> io::Result<()> {
    for (name, output) in outputs {
        validate_name(name, "output")?;
        if name != "discard" && output.files.is_empty() {
            return Err(invalid(format!("output {name} declares no files")));
        }
        for entry in &output.files {
            validate_output_path(&entry.path)?;
        }
    }
    Ok(())
}

fn validate_bundles(
    outputs: &BTreeMap<String, Output>,
    bundles: &BTreeMap<String, BTreeSet<String>>,
) -> io::Result<()> {
    for (name, bundle) in bundles {
        validate_name(name, "bundle")?;
        if bundle.is_empty() {
            return Err(invalid(format!("bundle {name} declares no outputs")));
        }
        for output in bundle {
            if output == "discard" {
                return Err(invalid(format!("bundle {name} includes discarded files")));
            }
            if !outputs.contains_key(output) {
                return Err(invalid(format!(
                    "bundle {name} names unknown output {output}"
                )));
            }
        }
    }
    Ok(())
}

fn validate_package(package: &Package) -> io::Result<()> {
    validate_name(&package.slug, "package slug")?;
    validate_version(&package.version)?;
    if let Some(checksum) = &package.checksum {
        validate_hash(checksum, "package checksum")?;
    }
    let namespace = package.namespace.trim_start_matches("pkg/");
    let valid_namespace = !namespace.is_empty()
        && namespace
            .split('/')
            .all(|part| validate_name(part, "namespace").is_ok());
    if valid_namespace {
        Ok(())
    } else {
        Err(invalid(format!(
            "invalid package namespace {:?}",
            package.namespace
        )))
    }
}

pub(crate) fn validate_environment(environment: &BuildEnvironment) -> io::Result<()> {
    validate_name(&environment.name, "build environment")?;
    let paths = [
        &environment.paths.work,
        &environment.paths.out,
        &environment.paths.inputs,
    ];
    for path in paths {
        if !safe_relative_path(path) {
            return Err(invalid(format!(
                "build path must stay below its root: {}",
                path.display()
            )));
        }
    }
    for (index, left) in paths.iter().enumerate() {
        for right in &paths[index + 1..] {
            if left.starts_with(right) || right.starts_with(left) {
                return Err(invalid(
                    "build work, output, and input paths must not overlap",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path.file_name().is_some()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn validate_output_path(value: &str) -> io::Result<()> {
    let path = Path::new(value);
    let escapes = path
        .components()
        .any(|part| matches!(part, Component::ParentDir | Component::Prefix(_)));
    if path.is_absolute() && !escapes && path.file_name().is_some() {
        Ok(())
    } else {
        Err(invalid(format!("invalid output path {value:?}")))
    }
}

fn environment_file(manifest: &Path, reference: &str) -> Option<PathBuf> {
    let reference = Path::new(reference);
    let beside_manifest = manifest.parent()?.join(reference);
    if reference.is_absolute() && reference.is_file() {
        Some(reference.to_path_buf())
    } else if beside_manifest.is_file() {
        Some(beside_manifest)
    } else if reference.is_file() {
        Some(reference.to_path_buf())
    } else {
        None
    }
}

fn read_git_blob(manifest: &Path, reference: &str) -> io::Result<Vec<u8>> {
    let directory = manifest.parent().unwrap_or_else(|| Path::new("."));
    let output = Command::new("git")
        .args(["-C", path_text(directory)?, "cat-file", "blob", reference])
        .output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(invalid(format!(
            "cannot read build environment {reference}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn path_text(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| invalid(format!("path is not UTF-8: {}", path.display())))
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
