pub mod format;
pub mod index;
pub mod parser;
pub mod repositories;
pub mod source;
pub mod system_base;
pub mod types;
pub mod update;

pub use format::format_manifest;
pub use index::ManifestIndex;
pub use parser::*;
pub use repositories::{
    repository_root_for_path, resolve_repository_reference, ManifestRepositories,
};
pub use source::ManifestSource;
pub use system_base::system_base_source;
pub use types::*;
pub use update::*;

#[cfg(test)]
mod format_tests;
