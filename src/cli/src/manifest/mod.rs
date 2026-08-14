pub mod format;
pub mod index;
pub mod inheritance;
pub mod parser;
pub mod repositories;
pub mod source;
pub mod types;
pub mod update;

pub use format::format_manifest;
pub use index::ManifestIndex;
pub use inheritance::resolve_inheritance;
pub use parser::load_system_manifest_resolved;
pub use parser::*;
pub use repositories::{repository_root_for_path, ManifestRepositories};
pub use source::ManifestSource;
pub use types::*;
pub use update::*;

#[cfg(test)]
mod format_tests;
