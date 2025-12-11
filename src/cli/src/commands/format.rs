use clap::Args;
use std::io;

use crate::manifest::format_manifest;

#[derive(Args)]
pub struct FormatArgs {
    /// Manifest file(s) to format
    #[clap(required = true)]
    pub files: Vec<String>,

    /// Check formatting without modifying files (exit 1 if changes needed)
    #[clap(long)]
    pub check: bool,
}

pub fn run(args: &FormatArgs) -> io::Result<()> {
    let mut needs_formatting = false;

    for file in &args.files {
        if args.check {
            // check mode: compare formatted output to original
            let original = std::fs::read_to_string(file)?;
            let formatted = crate::manifest::format::format_manifest_string(&original)?;

            if original != formatted {
                println!("{}: needs formatting", file);
                needs_formatting = true;
            }
        } else {
            // format mode: modify file in place
            format_manifest(file)?;
            println!("formatted {}", file);
        }
    }

    if args.check && needs_formatting {
        return Err(io::Error::other("some files need formatting"));
    }

    Ok(())
}
