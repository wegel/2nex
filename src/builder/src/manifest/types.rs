use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Source for loading a manifest - either from disk or from a git blob
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ManifestSource {
    /// Load from file path on disk (floating mode)
    Path(PathBuf),
    /// Load from git blob SHA (pinned mode), with path for identification
    Blob { sha: String, path: PathBuf },
}

impl ManifestSource {
    /// Get the path (for identification/display)
    pub fn path(&self) -> &PathBuf {
        match self {
            ManifestSource::Path(p) => p,
            ManifestSource::Blob { path, .. } => path,
        }
    }

    /// Check if this is an empty marker (for skipped nodes)
    pub fn is_empty(&self) -> bool {
        match self {
            ManifestSource::Path(p) => p == &PathBuf::new(),
            ManifestSource::Blob { path, .. } => path == &PathBuf::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub package: Package,
    pub dependencies: Vec<Dependency>,
    pub sources: Vec<Source>,
    pub build: Build,
    #[serde(deserialize_with = "deserialize_outputs")]
    pub outputs: HashMap<String, OutputSpec>,
    #[serde(deserialize_with = "deserialize_bundles")]
    pub bundles: HashMap<String, Bundle>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Package {
    pub name: String,
    pub slug: String,
    pub version: String,
    #[serde(alias = "flavor")]
    pub namespace: String,
    pub checksum: Option<String>,
    pub stable_checksum: Option<bool>,
    #[serde(default)]
    pub bootstrap: bool,
}

impl Package {
    /// Return the namespace path used for OSTree branches, ensuring it is rooted under `pkg/`.
    pub fn namespace_path(&self) -> String {
        if self.namespace.starts_with("pkg/") {
            self.namespace.clone()
        } else {
            format!("pkg/{}", self.namespace)
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ManifestKind {
    Package,
    System,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SystemMeta {
    pub name: String,
    pub slug: String,
    pub version: String,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub boot_method: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub checksum: Option<String>,
    #[serde(default)]
    pub stable_checksum: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SystemManifest {
    #[serde(default)]
    pub schema: Option<u32>,
    pub system: SystemMeta,
    #[serde(default)]
    pub packages: Vec<SystemPackage>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub sources: Vec<Source>,
    pub build: Build,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SystemPackage {
    pub commit: String,
    #[serde(default)]
    pub name: Option<String>,
}

pub enum ManifestData {
    Package(Manifest),
    System(SystemManifest),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Dependency {
    pub commit: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub manifest_ref: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Source {
    pub name: String,
    pub url: Option<String>,
    pub file: Option<String>,
    pub sha256: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Build {
    pub script: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bundle {
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub suggests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BundleDef {
    Simple(Vec<String>),
    Detailed(Bundle),
}

impl From<BundleDef> for Bundle {
    fn from(def: BundleDef) -> Self {
        match def {
            BundleDef::Simple(includes) => Bundle {
                includes,
                requires: Vec::new(),
                suggests: Vec::new(),
            },
            BundleDef::Detailed(bundle) => bundle,
        }
    }
}

fn deserialize_bundles<'de, D>(deserializer: D) -> Result<HashMap<String, Bundle>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, BundleDef> = HashMap::deserialize(deserializer)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.into())).collect())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputSpec {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub suggests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OutputDef {
    Simple(Vec<String>),
    Detailed(OutputSpec),
}

impl From<OutputDef> for OutputSpec {
    fn from(def: OutputDef) -> Self {
        match def {
            OutputDef::Simple(files) => OutputSpec {
                files,
                requires: Vec::new(),
                suggests: Vec::new(),
            },
            OutputDef::Detailed(spec) => spec,
        }
    }
}

fn deserialize_outputs<'de, D>(deserializer: D) -> Result<HashMap<String, OutputSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, OutputDef> = HashMap::deserialize(deserializer)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.into())).collect())
}
