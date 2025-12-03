use clap::Args;

#[derive(Args)]
pub struct LinkArgs {
    /// The manifest file to process
    pub manifest: String,
}

// the actual implementation is in main.rs (link_manifest_dependencies)
// this module just defines the CLI args
