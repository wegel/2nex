//! Command-line arguments for repository-owned tasks.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Run repository-owned tasks.
#[derive(Debug, Parser)]
#[command(name = "xtask")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Point this clone at the tracked Git hooks.
    InstallHooks,
    /// Run one tracked Git hook.
    Hook {
        #[command(subcommand)]
        hook: HookCommand,
    },
    /// Check Rust source rules directly.
    CodeStyle(CodeStyleArgs),
}

#[derive(Clone, Debug, Args)]
pub(crate) struct CodeStyleArgs {
    /// Check the blobs currently staged in Git.
    #[arg(long, conflicts_with = "all")]
    pub(crate) staged: bool,

    /// Check every eligible tracked or untracked Rust file.
    #[arg(long)]
    pub(crate) all: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum HookCommand {
    /// Check staged Rust code before Git creates a commit.
    PreCommit,
    /// Check the message that Git will attach to a commit.
    CommitMsg {
        /// Message file supplied by Git.
        message_file: PathBuf,
    },
    /// Run checks before Git sends commits to a remote.
    PrePush,
}
