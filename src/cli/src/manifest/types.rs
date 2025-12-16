use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// deserialize version field that accepts both strings and numbers
fn deserialize_version<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct VersionVisitor;

    impl<'de> Visitor<'de> for VersionVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_f64<E>(self, value: f64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(VersionVisitor)
}

/// Source for loading a manifest - either from disk, git blob, or a skip marker
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ManifestSource {
    /// Load from file path on disk (floating mode)
    Path(PathBuf),
    /// Load from git blob SHA (pinned mode), with path for identification
    Blob { sha: String, path: PathBuf },
    /// Skip marker for already-built packages in dependency graph
    Skip,
}

impl ManifestSource {
    /// Get the path (for identification/display). Returns empty path for Skip.
    pub fn path(&self) -> &PathBuf {
        // static empty path for Skip variant
        static EMPTY_PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        match self {
            ManifestSource::Path(p) => p,
            ManifestSource::Blob { path, .. } => path,
            ManifestSource::Skip => EMPTY_PATH.get_or_init(PathBuf::new),
        }
    }

    /// Check if this is a skip marker (for already-built packages)
    pub fn is_skip(&self) -> bool {
        matches!(self, ManifestSource::Skip)
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
    /// resolution map: file_path (e.g., "/usr/lib/libc.so.6") -> dependency_name (e.g., "glibc")
    /// internal libraries use "self" as the value
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub resolution: HashMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Package {
    pub name: String,
    pub slug: String,
    #[serde(deserialize_with = "deserialize_version")]
    pub version: String,
    #[serde(alias = "flavor")]
    pub namespace: String,
    pub checksum: Option<String>,
    pub stable_checksum: Option<bool>,
    /// when true, bootstrap dependencies are allowed (seed packages bootstrap the system)
    #[serde(default)]
    pub seed: bool,
}

impl Package {
    /// Return the namespace path used for store branches, ensuring it is rooted under `pkg/`.
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
    #[serde(deserialize_with = "deserialize_version")]
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
    /// when true, packages are installed to /nex/pkg/ with symlink forest in /usr/bin/
    #[serde(default)]
    pub nex_structure: bool,
    /// path to base assembly manifest to extend (relative to repo root)
    #[serde(default)]
    pub extends: Option<PathBuf>,
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
    #[serde(default)]
    pub overlays: Vec<PathBuf>,
    pub build: Build,
    /// items to exclude from parent assembly when extending
    #[serde(default)]
    pub exclude: Option<ExcludeConfig>,
}

/// configuration for excluding items from parent assembly
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ExcludeConfig {
    #[serde(default)]
    pub packages: Vec<ExcludeSpec>,
    #[serde(default)]
    pub dependencies: Vec<ExcludeSpec>,
}

/// specifies an item to exclude by name or commit
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ExcludeSpec {
    ByName { name: String },
    ByCommit { commit: String },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Overlay {
    #[serde(default)]
    pub files: Vec<OverlayEntry>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OverlayEntry {
    pub path: PathBuf,
    #[serde(default)]
    pub mode: Option<u32>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub source: Option<PathBuf>,
    #[serde(default)]
    pub symlink: Option<PathBuf>,
    #[serde(default)]
    pub directory: bool,
    #[serde(default)]
    pub replace: bool,
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
    /// URL or path to Cargo.lock file for automatic vendoring
    #[serde(default)]
    pub cargo_lock: Option<String>,
    /// optional: explicit Cargo.toml URL (defaults to same directory as cargo_lock)
    #[serde(default)]
    pub cargo_toml: Option<String>,
    /// URL or path to go.sum file for automatic Go module vendoring
    #[serde(default)]
    pub go_sum: Option<String>,
    /// URL or path to build.zig.zon file for automatic Zig dependency vendoring
    #[serde(default)]
    pub zig_zon: Option<String>,
    pub sha256: String,
}

/// Execution configuration for build environment
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ExecutionConfig {
    /// whether to chroot into the build directory (false = run on host filesystem)
    #[serde(default = "default_true")]
    pub chroot: bool,
}

fn default_true() -> bool {
    true
}

/// Paths configuration for build directories (relative to build_dir)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BuildPaths {
    /// relative path for work directory (e.g., "nex/work")
    pub work: String,
    /// relative path for output directory (e.g., "nex/out")
    pub out: String,
    /// relative path for inputs directory (e.g., "inputs")
    pub inputs: String,
}

/// Build environment definition loaded from external YAML file
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BuildEnvironment {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// execution mode configuration
    #[serde(default)]
    pub execution: ExecutionConfig,
    /// environment variables to set during build
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// script to run before the build script (device mounts, FHS setup, etc.)
    #[serde(default)]
    pub preamble: String,
    /// directory structure paths (relative to build_dir)
    pub paths: BuildPaths,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Build {
    /// git blob SHA1 of the environment definition file
    pub environment: String,
    pub script: String,
    /// progress profile for build time estimation
    /// each element is "bytes:time_ms"
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profile: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bundle {
    #[serde(default)]
    pub includes: Vec<String>,
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
            BundleDef::Simple(includes) => Bundle { includes },
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

/// File entry with optional dependencies
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// file path in this output
    pub path: String,
    /// runtime dependency file paths (empty for scripts, static binaries)
    /// these are resolved via the manifest's `resolution` map
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputSpec {
    /// file entries with per-file dependencies
    #[serde(default)]
    pub files: Vec<FileEntry>,
}

// note: no backwards compatibility - OutputSpec is the only format now

fn deserialize_outputs<'de, D>(deserializer: D) -> Result<HashMap<String, OutputSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    HashMap::deserialize(deserializer)
}
