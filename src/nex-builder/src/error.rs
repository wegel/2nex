//! Public failures returned by complete graph builds.

use std::error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// A failure to load, plan, execute, or publish a build graph.
pub enum Error {
    /// A manifest uses a schema version this builder does not understand.
    UnsupportedSchema {
        /// Manifest that declared the version.
        path: PathBuf,
        /// Unsupported numeric schema version.
        version: u64,
    },
    /// A manifest could not be decoded as the closed YAML schema.
    Manifest {
        /// Manifest that could not be decoded.
        path: PathBuf,
        /// YAML decoder failure.
        source: serde_yaml::Error,
    },
    /// A dependency is neither a stored content hash nor a package ref.
    InvalidReference {
        /// Rejected dependency value.
        value: String,
    },
    /// Package or assembly dependencies contain a cycle.
    GraphCycle {
        /// Node labels from the first repeated node through the repetition.
        chain: String,
    },
    /// A content hash named as an input is absent from the Zub store.
    MissingInput {
        /// Missing content hash.
        reference: String,
        /// Zub lookup failure.
        source: zub::Error,
    },
    /// A built package or system differs from its declared checksum.
    ChecksumMismatch {
        /// Check that found the mismatch.
        kind: ChecksumKind,
        /// Checksum declared by the manifest or produced by the first build.
        expected: String,
        /// Checksum produced by the build under test.
        actual: String,
    },
    /// One graph node failed after its build log was created.
    NodeFailed {
        /// Package or system label shown by progress output.
        label: String,
        /// File that captured the build script's output.
        log: PathBuf,
        /// Failure reported while building the node.
        source: Box<Error>,
    },
    /// Zub could not publish a package or system ref.
    Publication {
        /// Ref that the builder attempted to publish.
        reference: String,
        /// Filesystem or Zub failure from the publication path.
        source: io::Error,
    },
    /// A filesystem, process, or other operating-system operation failed.
    Io(io::Error),
    /// A Zub store operation failed outside publication.
    Store(zub::Error),
}

/// The checksum comparison that rejected built output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumKind {
    /// A package output differs from its manifest checksum.
    PackageOutput,
    /// A system tree differs from its manifest checksum.
    System,
    /// Two clean builds produced different trees.
    Reproducibility,
}

/// A builder result with a matchable [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { path, version } => {
                write!(
                    formatter,
                    "{} uses unsupported schema version {version}",
                    path.display()
                )
            }
            Self::Manifest { path, source } => {
                write!(
                    formatter,
                    "cannot parse manifest {}: {source}",
                    path.display()
                )
            }
            Self::InvalidReference { value } => {
                write!(formatter, "unsupported dependency reference {value:?}")
            }
            Self::GraphCycle { chain } => write!(formatter, "build graph cycle: {chain}"),
            Self::MissingInput { reference, source } => {
                write!(formatter, "missing stored commit {reference}: {source}")
            }
            Self::ChecksumMismatch {
                kind,
                expected,
                actual,
            } => write!(
                formatter,
                "{kind} checksum mismatch: expected {expected}, got {actual}"
            ),
            Self::NodeFailed { label, log, source } => write!(
                formatter,
                "{label} failed: {source}; build log: {}",
                log.display()
            ),
            Self::Publication { reference, source } => {
                write!(formatter, "cannot publish {reference}: {source}")
            }
            Self::Io(source) => source.fmt(formatter),
            Self::Store(source) => source.fmt(formatter),
        }
    }
}

impl fmt::Display for ChecksumKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PackageOutput => "output",
            Self::System => "system",
            Self::Reproducibility => "reproducibility",
        })
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Manifest { source, .. } => Some(source),
            Self::MissingInput { source, .. } | Self::Store(source) => Some(source),
            Self::NodeFailed { source, .. } => Some(source),
            Self::Publication { source, .. } | Self::Io(source) => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<zub::Error> for Error {
    fn from(source: zub::Error) -> Self {
        Self::Store(source)
    }
}
