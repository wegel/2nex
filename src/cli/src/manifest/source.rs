//! Locations from which Nex loads package and assembly manifests.

use std::path::{Path, PathBuf};

/// Source for loading a manifest from disk, a Git blob, or a graph skip marker.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ManifestSource {
    /// Load from a file path on disk in floating mode.
    Path(PathBuf),
    /// Load a pinned blob from the Git repository that owns its manifest.
    Blob {
        sha: String,
        path: PathBuf,
        git_root: PathBuf,
    },
    /// Mark a package whose existing build can satisfy the graph.
    Skip,
}

impl ManifestSource {
    /// Return the source path used for display and graph identity.
    pub fn path(&self) -> &Path {
        match self {
            Self::Path(path) | Self::Blob { path, .. } => path,
            Self::Skip => Path::new(""),
        }
    }

    /// Return the Git root for a pinned blob source.
    pub fn git_root(&self) -> Option<&Path> {
        match self {
            Self::Blob { git_root, .. } => Some(git_root),
            Self::Path(_) | Self::Skip => None,
        }
    }

    /// Report whether the graph should skip this source.
    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}
