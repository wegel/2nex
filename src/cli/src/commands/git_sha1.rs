use std::io;
use std::path::Path;
use std::process::Command;

use clap::Args;

#[derive(Args)]
pub struct GitSha1Args {
    /// Path to the file
    pub file: String,
}

pub fn run(args: &GitSha1Args) -> io::Result<()> {
    let path = Path::new(&args.file);

    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", args.file),
        ));
    }

    // use git hash-object to get the blob SHA1
    let output = Command::new("git")
        .args(["hash-object", &args.file])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "git hash-object failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    let sha1 = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("{}", sha1);

    Ok(())
}
