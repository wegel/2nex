use clap::Args;
use std::io;
use std::process::Command;

#[derive(Args)]
pub struct RollbackArgs {
    /// Deployment index to rollback to (default: previous deployment)
    #[clap(long, short = 'n')]
    pub index: Option<usize>,

    /// Don't prompt for confirmation
    #[clap(long, short = 'y')]
    pub yes: bool,
}

pub fn run(args: &RollbackArgs) -> io::Result<()> {
    // check if we're on an OSTree system
    let output = Command::new("ostree").args(["admin", "status"]).output();

    let status_output = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Not an OSTree-managed system. Rollback is only available on OSTree systems.",
            ));
        }
    };

    // parse deployments
    let deployments = parse_deployments(&status_output);

    if deployments.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No previous deployment available for rollback.",
        ));
    }

    // determine target deployment
    let target_index = args.index.unwrap_or(1); // default to second deployment (index 1)

    if target_index >= deployments.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Invalid deployment index {}. Available: 0-{}",
                target_index,
                deployments.len() - 1
            ),
        ));
    }

    let target = &deployments[target_index];
    let current = &deployments[0];

    println!("Rollback plan:");
    println!("  Current:  {} (index 0)", current.id);
    println!("  Target:   {} (index {})", target.id, target_index);
    println!();

    if !args.yes {
        println!(
            "This will set deployment {} as the default boot target.",
            target_index
        );
        println!("The system will boot into this deployment on next reboot.");
        println!();
        print!("Continue? [y/N] ");

        // flush stdout
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Rollback cancelled.");
            return Ok(());
        }
    }

    // perform rollback using ostree admin set-default
    println!("Setting deployment {} as default...", target_index);

    let status = Command::new("ostree")
        .args(["admin", "set-default", &target_index.to_string()])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to set default deployment. You may need to run as root.",
        ));
    }

    println!();
    println!("Rollback successful!");
    println!("Deployment {} will be active after reboot.", target.id);
    println!();
    println!("Reboot now with: systemctl reboot");

    Ok(())
}

struct Deployment {
    id: String,
}

fn parse_deployments(status: &str) -> Vec<Deployment> {
    let mut deployments = Vec::new();

    for line in status.lines() {
        // deployment lines start with * (current) or space followed by os name
        if !line.starts_with('*') && !line.starts_with(' ') {
            continue;
        }
        // skip deeply indented lines (details)
        if line.starts_with("    ") {
            continue;
        }

        let is_current = line.starts_with('*');

        // parse: "* 2nex <checksum>.<serial>" or "  2nex <checksum>.<serial>"
        let parts: Vec<&str> = line.split_whitespace().collect();

        let id = if is_current && parts.len() > 2 {
            parts[2].trim_end_matches("(staged)")
        } else if !is_current && parts.len() > 1 {
            parts[1].trim_end_matches("(staged)")
        } else {
            continue;
        };

        deployments.push(Deployment { id: id.to_string() });
    }

    deployments
}
