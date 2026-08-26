//! Command-line entry point for the small Nex graph builder.

use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::Parser;
use nex_builder::{build_graph, GraphOptions, OutputMode, OutputOptions, Result};

#[derive(Parser)]
#[command(version, about = "Build a Nex package or complete assembly graph")]
struct Args {
    /// Package or assembly manifest at the graph root.
    manifest: PathBuf,

    /// Zub repository that supplies dependencies and receives results.
    #[arg(long, default_value = ".nex/repo")]
    repo: PathBuf,

    /// Directory used for node roots and logs.
    #[arg(long, default_value = ".nex/tmp/graph-builder")]
    build_dir: PathBuf,

    /// Directory used for verified source files.
    #[arg(long, default_value = ".nex/cache/sources")]
    source_cache: PathBuf,

    /// Build twice from clean roots and compare output checksums.
    #[arg(long)]
    check: bool,

    /// Most build scripts to run at once.
    #[arg(long, default_value = "1")]
    jobs: NonZeroUsize,

    /// Total logical CPUs shared by running build scripts.
    #[arg(long)]
    cpus: Option<NonZeroUsize>,

    /// Show build-script output alongside progress.
    #[arg(short, long, conflicts_with = "quiet")]
    verbose: bool,

    /// Suppress progress, status, and the final summary.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,

    /// Print stable status lines instead of redrawing progress bars.
    #[arg(long)]
    no_progress: bool,

    /// Replace timing profiles in manifests after successful builds.
    #[arg(long)]
    record_profile: bool,

    /// Trust complete refs without recipe metadata and attach the current recipe.
    #[arg(long)]
    adopt_existing: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mode = if args.quiet {
        OutputMode::Quiet
    } else if args.no_progress && args.verbose {
        OutputMode::PlainVerbose
    } else if args.no_progress {
        OutputMode::Plain
    } else if args.verbose {
        OutputMode::ProgressVerbose
    } else {
        OutputMode::Progress
    };
    let result = build_graph(&GraphOptions {
        manifest: args.manifest,
        repo: args.repo,
        build_dir: args.build_dir,
        source_cache: args.source_cache,
        check: args.check,
        adopt_existing: args.adopt_existing,
        jobs: args.jobs,
        cpus: args
            .cpus
            .unwrap_or_else(|| std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)),
        output: OutputOptions {
            mode,
            record_profile: args.record_profile,
        },
    })?;

    if args.quiet {
        return Ok(());
    }
    println!("built {}", result.built);
    println!("reused {}", result.reused);
    for reference in result.root_references {
        println!("root {reference}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
