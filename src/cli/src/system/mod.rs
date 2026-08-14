//! System assembly build flow and materialization helpers.

mod build;
mod commit;
mod config;
mod dependencies;
mod env;
mod flat;
mod nex;
mod nex_db;
mod nex_links;
mod nex_shim;
mod overlays;

pub use build::{build_system_manifest, build_system_manifest_with_dir};
pub use commit::commit_system_rootfs;
pub use dependencies::dependencies_from_system_packages;
pub use env::build_system_env_vars;
pub use flat::materialize_system_packages;
pub use nex::materialize_nex_structure;
