//! CLI arguments and option conversion for `nex build`.

use std::io;
use std::path::{Path, PathBuf};
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

    /// Override the directories Nex searches for package manifests
    #[clap(long)]
    pub manifest_dir: Option<PathBuf>,

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
    /// Package manifest directories used for dependency and runtime lookup.
    pub manifest_dirs: Vec<PathBuf>,
    /// Git repository whose manifests this command may rewrite.
    pub writable_manifest_root: Option<PathBuf>,
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
    pub fn from_args(
        args: &BuildArgs,
        repo_path: String,
        manifest_path: &Path,
        manifest_dirs: Vec<PathBuf>,
        writable_manifest_root: PathBuf,
    ) -> Self {
        Self {
            repo_path,
            manifest_file: manifest_path.to_string_lossy().into_owned(),
            manifest_dirs,
            writable_manifest_root: Some(writable_manifest_root),
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

    /// Refuse a write flag when the current manifest belongs to an imported repository.
    pub fn ensure_manifest_write_allowed(&self) -> io::Result<()> {
        if !self.requests_manifest_write() {
            return Ok(());
        }
        let Some(writable_root) = &self.writable_manifest_root else {
            return Ok(());
        };
        let manifest_path = Path::new(&self.manifest_file);
        let owner = crate::manifest::repository_root_for_path(manifest_path)?;
        if &owner == writable_root {
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing to modify imported manifest {}; update it in its own repository",
                manifest_path.display()
            ),
        ))
    }

    /// Drop flags that can rewrite a manifest before the scheduler builds an imported package.
    pub fn restrict_imported_manifest_writes(&mut self) -> io::Result<()> {
        let Some(writable_root) = &self.writable_manifest_root else {
            return Ok(());
        };
        let owner = crate::manifest::repository_root_for_path(Path::new(&self.manifest_file))?;
        if &owner == writable_root {
            return Ok(());
        }
        self.update_checksum = false;
        self.compute_deps = false;
        self.refresh_metadata = false;
        self.generate_outputs = false;
        self.record_profile = false;
        self.add_checksums = false;
        Ok(())
    }

    fn requests_manifest_write(&self) -> bool {
        self.update_checksum
            || self.compute_deps
            || self.refresh_metadata
            || self.generate_outputs
            || self.record_profile
            || self.add_checksums
    }
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod build_tests;
