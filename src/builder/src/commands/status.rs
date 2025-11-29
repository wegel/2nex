use clap::Args;
use std::io;
use std::process::Command;

#[derive(Args)]
pub struct StatusArgs {
    /// Show detailed information about deployments
    #[clap(long, short = 'v')]
    pub verbose: bool,
}

pub fn run(args: &StatusArgs) -> io::Result<()> {
    // try ostree admin status first
    let output = Command::new("ostree").args(["admin", "status"]).output();

    match output {
        Ok(out) if out.status.success() => {
            let status = String::from_utf8_lossy(&out.stdout);
            print_ostree_status(&status, args.verbose);
        }
        _ => {
            // not an OSTree system or ostree not available
            println!("System deployments:");
            println!("  (not an OSTree-managed system)");
            println!();
            println!("Package state is tracked in /nex/var/installed.json");

            // try to show installed packages
            if let Ok(state) = super::state::InstalledState::load() {
                let count = state.packages.len();
                if count > 0 {
                    println!();
                    println!("Installed packages: {}", count);
                    for (key, pkg) in &state.packages {
                        let version_count = pkg.versions.len();
                        let current = pkg.current.as_deref().unwrap_or("(none)");
                        println!(
                            "  {} - current: {}, versions: {}",
                            key, current, version_count
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_ostree_status(status: &str, verbose: bool) {
    println!("System deployments:");
    println!();

    let mut deployment_num = 0;
    let mut in_deployment = false;

    for line in status.lines() {
        // deployment lines start with * (current) or space
        if line.starts_with('*') || (line.starts_with(' ') && !line.starts_with("    ")) {
            if in_deployment {
                println!();
            }

            let is_current = line.starts_with('*');
            let is_staged = line.contains("(staged)");
            deployment_num += 1;

            // parse deployment info
            // format: "* 2nex <checksum>.<serial> (staged)" or "  2nex <checksum>.<serial>"
            let parts: Vec<&str> = line.split_whitespace().collect();

            let marker = if is_current {
                if is_staged {
                    "→"
                } else {
                    "*"
                }
            } else {
                " "
            };

            let status_str = if is_staged {
                " (staged - pending reboot)"
            } else if is_current {
                " (booted)"
            } else {
                ""
            };

            if parts.len() >= 2 {
                let os_name = if is_current { parts[1] } else { parts[0] };
                let deployment_id = if is_current && parts.len() > 2 {
                    parts[2]
                } else if !is_current && parts.len() > 1 {
                    parts[1]
                } else {
                    "unknown"
                };

                println!(
                    "{} {} {}{}",
                    marker, deployment_num, deployment_id, status_str
                );

                if verbose {
                    println!("    OS: {}", os_name);
                }
            }

            in_deployment = true;
        } else if verbose && in_deployment && line.starts_with("    ") {
            // indented lines are deployment details
            println!("   {}", line.trim());
        }
    }

    if deployment_num == 0 {
        println!("  No deployments found");
    } else {
        println!();
        println!("Legend: * = booted, → = staged (pending reboot)");
        println!();
        println!("Use 'nex rollback' to set a previous deployment as default");
    }
}
