use clap::Args;

#[derive(Args)]
pub struct BuildArgs {
    /// Manifest file to build
    pub manifest: String,

    /// Repository path (default: .nex/repo)
    #[clap(long)]
    pub repo: Option<String>,

    /// Build to system repo (requires root). Without this, builds go to user repo.
    #[clap(long)]
    pub system: bool,

    /// Base directory for searching manifests
    #[clap(long, default_value = ".")]
    pub manifest_dir: String,

    /// Run the build script on the host's filesystem (for bootstrapping)
    #[clap(long)]
    pub bootstrap: bool,

    /// Compute runtime dependencies (needs/resolution) and update manifest
    #[clap(long)]
    pub compute_deps: bool,

    /// Include per-reference explanations in runtime dependency output
    #[clap(long)]
    pub runtime_deps_verbose: bool,

    /// Validate build reproducibility by building twice and comparing
    #[clap(long, alias = "validate-reproducibility")]
    pub check: bool,

    /// Update the manifest checksum when build outputs differ from what is recorded
    #[clap(long)]
    pub update_checksum: bool,

    /// Rewrite output/bundle metadata without rebuilding (package manifests only)
    #[clap(long)]
    pub refresh_metadata: bool,

    /// Build only the specified manifest without dependencies
    #[clap(long)]
    pub single: bool,

    /// Show what would be built without actually building (dry run)
    #[clap(long)]
    pub dry_run: bool,

    /// Maximum number of parallel build jobs (default: number of CPUs)
    #[clap(short = 'j', long)]
    pub jobs: Option<usize>,

    /// Show dependency paths for all packages
    #[clap(long)]
    pub show_dep_paths: bool,

    /// Force rebuild even if package is already built
    #[clap(long)]
    pub force: bool,

    /// Add checksums to manifests missing them (from store or by building)
    #[clap(long)]
    pub add_checksums: bool,

    /// Trace which packages pull in a specific dependency (e.g. 'bootstrap/phase1')
    #[clap(long)]
    pub trace_dependency: Option<String>,

    /// Specify build directory (default: ./build_rootfs_{slug}_{namespace})
    #[clap(long)]
    pub build_dir: Option<String>,

    /// Expand dependencies to include all transitive deps, ordered by depth
    #[clap(long)]
    pub hydrate_dependencies: bool,

    /// Generate outputs section from build results and write to manifest
    #[clap(long)]
    pub generate_outputs: bool,

    /// Fallback repository for dependency lookups (can be specified multiple times)
    #[clap(long = "fallback-repo", action = clap::ArgAction::Append)]
    pub fallback_repos: Vec<String>,
}

/// Build options passed to build functions
pub struct BuildOpts {
    pub repo_path: String,
    pub manifest_file: String,
    pub check: bool,
    pub update_checksum: bool,
    pub bootstrap: bool,
    pub compute_deps: bool,
    pub runtime_deps_verbose: bool,
    pub refresh_metadata: bool,
    pub force: bool,
    pub build_dir: Option<String>,
    pub generate_outputs: bool,
    pub fallback_repos: Vec<String>,
}

impl BuildOpts {
    pub fn from_args(args: &BuildArgs, repo_path: String) -> Self {
        Self {
            repo_path,
            manifest_file: args.manifest.clone(),
            check: args.check,
            update_checksum: args.update_checksum,
            bootstrap: args.bootstrap,
            compute_deps: args.compute_deps,
            runtime_deps_verbose: args.runtime_deps_verbose,
            refresh_metadata: args.refresh_metadata,
            force: args.force,
            build_dir: args.build_dir.clone(),
            generate_outputs: args.generate_outputs,
            fallback_repos: args.fallback_repos.clone(),
        }
    }
}
