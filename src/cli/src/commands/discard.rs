use clap::Args;
use std::io;

use super::stage::{cleanup_staging, is_staged};

#[derive(Args)]
pub struct DiscardArgs {
    /// Force discard even if there are uncommitted changes
    #[clap(long)]
    pub force: bool,
}

pub fn run(args: &DiscardArgs) -> io::Result<()> {
    if !is_staged() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Not in staging mode. Nothing to discard.",
        ));
    }

    if !args.force {
        // check if there are changes in the upper dirs
        let upper_nex = "/run/nex/staging/upper/nex";
        let upper_usr_bin = "/run/nex/staging/upper/usr_bin";

        let has_nex_changes = std::fs::read_dir(upper_nex)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        let has_bin_changes = std::fs::read_dir(upper_usr_bin)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

        if has_nex_changes || has_bin_changes {
            return Err(io::Error::other(
                "Uncommitted changes exist. Use --force to discard them, or 'nex commit' to save.",
            ));
        }
    }

    println!("Discarding staging changes...");
    cleanup_staging()?;
    println!("Staging mode exited. All changes discarded.");

    Ok(())
}
