use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

pub mod build;
pub mod cargo_vendor;
pub mod commands;
pub mod deps;
pub mod go_vendor;
pub mod manifest;
pub mod materializer;
pub mod outputs;
pub mod progress;
pub mod refs;
pub mod repo;
pub mod store;
pub mod system;
pub mod zig_vendor;

mod utils;

use build::orchestration::{
    build_with_dependencies, hydrate_dependencies, link_manifest_dependencies,
};
use manifest::*;
#[derive(Parser)]
#[clap(
    name = "nex",
    version = "1.0",
    about = "nex package builder and manager"
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build packages from manifests
    Build(commands::build::BuildArgs),

    /// Check manifest files for issues
    Check(commands::check::CheckArgs),

    /// List packages in the repository
    List(commands::list::ListArgs),

    /// Show information about a package
    Info(commands::info::InfoArgs),

    /// Search for packages
    Search(commands::search::SearchArgs),

    /// Pin dependencies and assembly packages to their current Git commits
    Link(commands::link::LinkArgs),

    /// Enter staging mode (create overlay for transactional changes)
    Stage(commands::stage::StageArgs),

    /// Discard staged changes and exit staging mode
    Discard(commands::discard::DiscardArgs),

    /// Install a package (requires staging mode or --commit)
    Install(commands::install::InstallArgs),

    /// Remove a package (requires staging mode)
    Remove(commands::remove::RemoveArgs),

    /// Switch the active version of a package
    Switch(commands::switch::SwitchArgs),

    /// Commit staged changes
    Commit(commands::commit::CommitArgs),

    /// Deploy a system ref to /nex/deployments
    Deploy(commands::deploy::DeployArgs),

    /// Upgrade this machine by deploying a built system ref
    Upgrade(commands::deploy::DeployArgs),

    /// List system deployments
    Deployments(commands::deployments::DeploymentsArgs),

    /// Show current and previous deployments
    Status(commands::status::StatusArgs),

    /// Rollback to a previous version
    Rollback(commands::rollback::RollbackArgs),

    /// Garbage-collect old deployments and (optionally) zub objects
    Gc(commands::gc::GcArgs),

    /// Resolve and show runtime dependencies for a package
    Resolve(commands::resolve::ResolveArgs),

    /// Compute and store runtime dependencies in manifest
    ComputeDeps(commands::compute_deps::ComputeDepsArgs),

    /// Show full transitive dependency graph of a manifest
    #[clap(name = "dep-graph")]
    DepGraph(commands::dep_graph::DepGraphArgs),

    /// Format manifest files
    Format(commands::format::FormatArgs),

    /// Generate ref completions for shell auto-completion
    Complete(commands::complete::CompleteArgs),

    /// Get git blob SHA1 for a file
    #[clap(name = "git-sha1")]
    GitSha1(commands::git_sha1::GitSha1Args),

    /// Remote helper for SSH transport (internal use)
    #[clap(name = "zub-remote")]
    ZubRemote {
        /// Repository path
        path: PathBuf,
    },
}

// re-export BuildOpts for internal use
pub use commands::build::BuildOpts;

fn main() -> io::Result<()> {
    let cli: Cli = Cli::parse();

    match cli.command {
        Command::Build(args) => run_build(&args),
        Command::Check(args) => commands::check::run(&args),
        Command::List(args) => commands::list::run(&args),
        Command::Info(args) => commands::info::run(&args),
        Command::Search(args) => commands::search::run(&args),
        Command::Link(args) => link_manifest_dependencies(&args.manifest),
        Command::Stage(args) => commands::stage::run(&args),
        Command::Discard(args) => commands::discard::run(&args),
        Command::Install(args) => commands::install::run(&args),
        Command::Remove(args) => commands::remove::run(&args),
        Command::Switch(args) => commands::switch::run(&args),
        Command::Commit(args) => commands::commit::run(&args),
        Command::Deploy(args) => commands::deploy::run(&args),
        Command::Upgrade(args) => commands::deploy::run(&args),
        Command::Deployments(args) => commands::deployments::run(&args),
        Command::Status(args) => commands::status::run(&args),
        Command::Rollback(args) => commands::rollback::run(&args),
        Command::Gc(args) => commands::gc::run(&args),
        Command::Resolve(args) => commands::resolve::run(&args),
        Command::ComputeDeps(args) => commands::compute_deps::run(&args),
        Command::DepGraph(args) => commands::dep_graph::run(&args),
        Command::Format(args) => commands::format::run(&args),
        Command::Complete(args) => commands::complete::run(&args),
        Command::GitSha1(args) => commands::git_sha1::run(&args),
        Command::ZubRemote { path } => run_zub_remote(&path),
    }
}

fn run_build(args: &commands::build::BuildArgs) -> io::Result<()> {
    let manifest_path = Path::new(&args.manifest).canonicalize()?;
    let repositories = manifest::ManifestRepositories::discover(&manifest_path)?;
    let manifest_dirs = build_manifest_dirs(args, &repositories)?;
    let repo_path = resolve_build_repo_path(args, &repositories)?;
    configure_build_jobs(args.jobs)?;
    let opts = BuildOpts::from_args(
        args,
        repo_path.clone(),
        &manifest_path,
        manifest_dirs.clone(),
        repositories.product_root().to_path_buf(),
    );

    if args.hydrate_dependencies {
        hydrate_dependencies(&repo_path, &opts.manifest_file, &manifest_dirs)
    } else if args.single || args.refresh_metadata {
        build::build_single(&opts)
    } else {
        build_with_dependencies(&manifest_path, &manifest_dirs, &opts)
    }
}

fn build_manifest_dirs(
    args: &commands::build::BuildArgs,
    repositories: &manifest::ManifestRepositories,
) -> io::Result<Vec<PathBuf>> {
    match &args.manifest_dir {
        Some(directory) => Ok(vec![directory.canonicalize()?]),
        None => Ok(repositories.package_dirs()),
    }
}

fn resolve_build_repo_path(
    args: &commands::build::BuildArgs,
    repositories: &manifest::ManifestRepositories,
) -> io::Result<String> {
    if let Some(repo_path) = &args.repo {
        return repo::resolve_repo_path(Some(repo_path));
    }
    let product_repo = repositories.product_root().join(".nex/repo");
    if !args.system && product_repo.exists() {
        return canonical_path_string(&product_repo);
    }
    if args.system {
        return repo::resolve_repo_path(None);
    }

    let context = repo::detect_context(false)?;
    if !context.repo_path.exists() {
        repo::ensure_user_dirs(&context)?;
        eprintln!(
            "Created user environment at {}",
            context
                .repo_path
                .parent()
                .unwrap_or(&context.repo_path)
                .display()
        );
    }
    Ok(context.repo_path.to_string_lossy().into_owned())
}

fn canonical_path_string(path: &Path) -> io::Result<String> {
    Ok(path.canonicalize()?.to_string_lossy().into_owned())
}

fn configure_build_jobs(jobs: Option<usize>) -> io::Result<()> {
    let Some(num_jobs) = jobs else {
        return Ok(());
    };
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_jobs)
        .build_global()
        .map_err(|error| io::Error::other(format!("Failed to configure thread pool: {error}")))
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.namespace, self.slug)
    }
}

/// run the zub remote helper protocol (server side of SSH transport)
fn run_zub_remote(repo_path: &Path) -> io::Result<()> {
    let repo = zub::Repo::open(repo_path).map_err(|e| io::Error::other(e.to_string()))?;
    zub::transport::serve_remote(&repo).map_err(|e| io::Error::other(e.to_string()))
}

/// Verifies and commits outputs to store branches based on the manifest.
///
/// This function takes the manifest, base directory, and repository path as input.
/// It verifies and commits the outputs specified in the manifest to the corresponding
/// store branches. The function categorizes the output files, checks their existence,
/// moves them to the appropriate output directories, and commits them to the
/// repository.

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn base_manifest() -> &'static str {
        r#"
package:
  schema: 1
  name: sample
  slug: sample
  namespace: bootstrap/phase0
  version: "1.0"
dependencies: []
sources: []
build:
  environment: env/test.yaml
  script: "true"
outputs: {}
"#
    }

    #[test]
    fn parses_bundle_with_includes() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    includes:\n      - bin\n      - lib\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
    }

    #[test]
    fn parses_output_with_files_and_needs() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen(
                "outputs: {}",
                "outputs:\n  bin:\n    files:\n      - path: /usr/bin/foo\n        needs:\n          - /usr/lib/libc.so.6",
                1,
            )
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].path, "/usr/bin/foo");
        assert_eq!(
            output.files[0].needs,
            vec!["/usr/lib/libc.so.6".to_string()]
        );
    }

    #[test]
    fn parses_simple_bundle_format() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    - bin\n    - lib\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
    }

    // note: old closure_resolution tests removed - now uses manifest-based resolution
    // which requires actual manifest files, not mock fetch functions

    #[test]
    fn detects_system_manifest_kind() {
        let yaml = r#"
schema: 1
kind: system
system:
  name: Demo
  slug: demo
  version: "1.0"
packages:
  - commit: x86_64/foo/1.0/base/bundles/dev
build:
  environment: env/test.yaml
  script: ":"
sources: []
dependencies: []
"#;
        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(detect_manifest_kind(&value), ManifestKind::System);
        let sys: SystemManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(sys.system.slug, "demo");
        assert_eq!(sys.system.version, "1.0");
    }

    #[test]
    fn load_manifest_parses_system_kind() {
        let yaml = r#"
schema: 1
kind: system
system:
  name: Demo
  slug: demo
  version: "1.0"
packages:
  - commit: x86_64/foo/1.0/base/bundles/dev
build:
  environment: env/test.yaml
  script: ":"
sources: []
dependencies: []
"#;
        let mut file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file, yaml.as_bytes()).unwrap();
        match load_manifest(file.path().to_str().unwrap()).unwrap() {
            ManifestData::System(sys) => {
                assert_eq!(sys.system.slug, "demo");
                assert_eq!(sys.packages.len(), 1);
            }
            _ => panic!("Expected system manifest"),
        }
    }

    #[test]
    fn parses_upgrade_as_deploy_args() {
        let cli = Cli::try_parse_from([
            "nex",
            "upgrade",
            "systems/desktop-vwl/0.0.1",
            "--sysroot",
            "/",
        ])
        .expect("upgrade command should parse");

        match cli.command {
            Command::Upgrade(args) => {
                assert_eq!(args.system_ref, "systems/desktop-vwl/0.0.1");
                assert_eq!(args.sysroot, PathBuf::from("/"));
            }
            _ => panic!("expected upgrade command"),
        }
    }
}
