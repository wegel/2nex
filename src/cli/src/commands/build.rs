//! CLI arguments and option conversion for `nex build`.

use std::sync::Arc;

use clap::Args;
use indicatif::MultiProgress;

/// Command-line arguments for `nex build`.
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

    /// Specify build directory (default: .nex/tmp/build_rootfs_{slug}_{namespace})
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

    /// Show build output alongside progress bar
    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Force re-recording of build profile (discards existing profile)
    #[clap(long)]
    pub record_profile: bool,

    /// Disable progress tracking (legacy behavior, shows raw output)
    #[clap(long)]
    pub no_progress: bool,

    /// Reuse existing build rootfs directory (skip deletion for faster iteration)
    #[clap(long)]
    pub reuse_rootfs: bool,
}

/// Build options passed from the CLI into package, system, and graph builders.
pub struct BuildOpts {
    /// Primary zub repository path used for build inputs and committed outputs.
    pub repo_path: String,
    /// Manifest path passed on the command line.
    pub manifest_file: String,
    /// Run a second build and compare output checksums.
    pub check: bool,
    /// Rewrite the manifest checksum when a build produces a new checksum.
    pub update_checksum: bool,
    /// Compute runtime dependency metadata after package outputs are committed.
    pub compute_deps: bool,
    /// Print per-reference runtime dependency explanations.
    pub runtime_deps_verbose: bool,
    /// Refresh existing output and bundle metadata without rebuilding.
    pub refresh_metadata: bool,
    /// Rebuild even when the store already has matching output refs.
    pub force: bool,
    /// Explicit build directory, or `None` for the default package/system path.
    pub build_dir: Option<String>,
    /// Regenerate output file lists from the built root.
    pub generate_outputs: bool,
    /// Fallback zub repositories used when refs are missing from `repo_path`.
    pub fallback_repos: Vec<String>,
    /// Print verbose checkout and build output.
    pub verbose: bool,
    /// Record a new build progress profile into the manifest.
    pub record_profile: bool,
    /// Disable progress UI and show raw build output.
    pub no_progress: bool,
    /// Print the build graph without building packages.
    pub dry_run: bool,
    /// Build missing package checksums for manifests that do not record them.
    pub add_checksums: bool,
    /// Print dependency paths in the build graph.
    pub show_dep_paths: bool,
    /// Print why a specific dependency appears in the graph.
    pub trace_dependency: Option<String>,
    /// Shared progress renderer for parallel builds.
    pub multi_progress: Option<Arc<MultiProgress>>,
    /// Reuse an existing build rootfs directory instead of deleting it first.
    pub reuse_rootfs: bool,
}

impl BuildOpts {
    /// Convert parsed CLI arguments into internal builder options.
    pub fn from_args(args: &BuildArgs, repo_path: String) -> Self {
        Self {
            repo_path,
            manifest_file: args.manifest.clone(),
            check: args.check,
            update_checksum: args.update_checksum,
            compute_deps: args.compute_deps,
            runtime_deps_verbose: args.runtime_deps_verbose,
            refresh_metadata: args.refresh_metadata,
            force: args.force,
            build_dir: args.build_dir.clone(),
            generate_outputs: args.generate_outputs,
            fallback_repos: args.fallback_repos.clone(),
            verbose: args.verbose,
            record_profile: args.record_profile,
            no_progress: args.no_progress,
            dry_run: args.dry_run,
            add_checksums: args.add_checksums,
            show_dep_paths: args.show_dep_paths,
            trace_dependency: args.trace_dependency.clone(),
            multi_progress: None,
            reuse_rootfs: args.reuse_rootfs,
        }
    }
}
