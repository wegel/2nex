//! Entry point for repository-owned tasks.

mod cargo_project;
mod cli;
mod commit_message;
mod config;
mod git;
mod hooks;
#[cfg(test)]
mod hooks_integration_tests;
mod style;

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command, HookCommand};

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("fatal: {error:#}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    let repo_root = git::repo_root()?;
    match Cli::parse().command {
        Command::InstallHooks => hooks::install(&repo_root),
        Command::CodeStyle(args) => hooks::run_code_style(&repo_root, args),
        Command::Hook { hook } => match hook {
            HookCommand::PreCommit => hooks::run_pre_commit(&repo_root),
            HookCommand::CommitMsg { message_file } => {
                hooks::run_commit_msg(&repo_root, &message_file)
            }
            HookCommand::PrePush => hooks::run_pre_push(&repo_root),
        },
    }
}
