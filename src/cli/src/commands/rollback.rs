use clap::Args;
use std::io;

#[derive(Args)]
pub struct RollbackArgs {
    /// Package version to rollback to (format: namespace/slug version/checksum)
    pub package: Option<String>,

    /// Don't prompt for confirmation
    #[clap(long, short = 'y')]
    pub yes: bool,
}

pub fn run(_args: &RollbackArgs) -> io::Result<()> {
    // with zub, there's no deployment concept - packages are just checked out
    // rollback would be switching to a different version of a package
    println!("Rollback is not needed with the zub store backend.");
    println!();
    println!("To switch package versions, use:");
    println!("  nex switch <package> <version>");
    println!();
    println!("To list installed versions:");
    println!("  nex status -v");

    Ok(())
}
