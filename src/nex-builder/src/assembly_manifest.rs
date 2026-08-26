//! System assembly data read from YAML.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::manifest::{validate_dependencies, validate_sources, Build, Dependency, Source};
use crate::reference::InputRef;
use crate::schema::{
    invalid, manifest_error, validate_hash, validate_name, validate_version, SchemaVersion,
};
use crate::Result as BuildResult;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssemblyManifest {
    pub system: System,
    pub base: Option<Base>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub packages: Vec<AssemblyPackage>,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub providers: BTreeMap<String, InputRef>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub files: Vec<AssemblyFile>,
    pub build: Build,
}

impl AssemblyManifest {
    pub(crate) fn reference(&self) -> String {
        format!("systems/{}/{}", self.system.slug, self.system.version)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct System {
    #[serde(rename = "schema", skip_serializing)]
    _schema: SchemaVersion,
    #[serde(rename = "name", skip_serializing)]
    _name: String,
    pub slug: String,
    pub version: String,
    #[serde(rename = "description", skip_serializing)]
    _description: String,
    #[serde(default)]
    pub nex_structure: bool,
    #[serde(skip_serializing)]
    pub checksum: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Base {
    #[serde(skip_serializing)]
    pub commit: String,
    pub manifest: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AssemblyFile {
    pub path: PathBuf,
    #[serde(flatten)]
    pub kind: AssemblyFileKind,
    pub mode: Option<u32>,
    #[serde(default)]
    pub replace: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum AssemblyFileKind {
    Content { content: String },
    Symlink { symlink: PathBuf },
    Directory { directory: True },
    Source { source: PathBuf },
    Empty {},
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AssemblyPackage {
    #[serde(flatten)]
    pub dependency: Dependency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<PackagePlacement>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAssemblyPackage {
    name: Option<String>,
    commit: InputRef,
    #[serde(default)]
    paths: Vec<PathBuf>,
    placement: Option<PackagePlacement>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PackagePlacement {
    Capsule,
    Root,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct True;

impl<'de> Deserialize<'de> for True {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        bool::deserialize(deserializer)?
            .then_some(Self)
            .ok_or_else(|| D::Error::custom("directory must be true"))
    }
}

impl Serialize for True {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for AssemblyPackage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAssemblyPackage::deserialize(deserializer)?;
        Ok(Self {
            dependency: Dependency {
                name: raw.name,
                commit: raw.commit,
                paths: raw.paths,
            },
            placement: raw.placement,
        })
    }
}

pub(crate) fn load_assembly(path: &Path) -> BuildResult<AssemblyManifest> {
    let bytes = fs::read(path)?;
    let manifest = serde_yaml::from_slice(&bytes)
        .map_err(|source| manifest_error(path, &bytes, "system", source))?;
    validate_assembly(&manifest)?;
    Ok(manifest)
}

fn validate_assembly(manifest: &AssemblyManifest) -> io::Result<()> {
    validate_name(&manifest.system.slug, "system slug")?;
    validate_version(&manifest.system.version)?;
    if let Some(checksum) = &manifest.system.checksum {
        validate_hash(checksum, "system checksum")?;
    }
    validate_dependencies(&manifest.dependencies)?;
    let packages = manifest
        .packages
        .iter()
        .map(|package| package.dependency.clone())
        .collect::<Vec<_>>();
    validate_dependencies(&packages)?;
    crate::runtime::validate_providers(&packages, &manifest.providers)?;
    if !manifest.system.nex_structure
        && manifest
            .packages
            .iter()
            .any(|package| package.placement.is_some())
    {
        return Err(invalid("flat assembly package cannot declare placement"));
    }
    validate_sources(&manifest.sources)?;
    for file in &manifest.files {
        validate_file(file)?;
    }
    Ok(())
}

fn validate_file(file: &AssemblyFile) -> io::Result<()> {
    if !safe_absolute(&file.path) {
        return Err(invalid(format!(
            "assembly file path must be absolute and stay in the system: {}",
            file.path.display()
        )));
    }
    if matches!(file.kind, AssemblyFileKind::Directory { .. }) && file.replace {
        return Err(invalid(format!(
            "assembly directory {} cannot replace an entry",
            file.path.display()
        )));
    }
    if file.mode.is_some_and(|mode| mode > 0o7777) {
        return Err(invalid(format!(
            "assembly file {} has invalid mode",
            file.path.display()
        )));
    }
    Ok(())
}

fn safe_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.file_name().is_some()
        && path
            .components()
            .all(|part| !matches!(part, Component::ParentDir | Component::Prefix(_)))
}

#[cfg(test)]
#[path = "assembly_manifest_tests.rs"]
mod tests;
