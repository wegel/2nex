//! Package build orchestration modules and public build entry points.

pub mod commits;
pub mod dependencies;
pub mod env;
pub mod inputs;
pub mod orchestration;
pub mod package;
pub mod package_outputs;
pub mod reproducibility;
pub mod rootfs;
pub mod script;
pub mod status;

pub use commits::{
    append_checksum_file, create_and_commit_bundles, refresh_package_metadata,
    verify_and_commit_outputs,
};
pub use env::{expand_env_templates, load_environment};
pub use inputs::handle_inputs;
pub use package::{build_package_manifest_with_dir, build_single};
pub use rootfs::{
    layer_commits_into_rootfs, setup_composite_rootfs, stage_existing_outputs, RootfsSetup,
};
pub use script::{run_build_script_with_env, BuildScriptResult};
pub use status::{check_if_built, compute_manifest_hash};

/// Output category name for files that should be discarded instead of committed.
pub const OUTPUT_DISCARD: &str = "discard";
