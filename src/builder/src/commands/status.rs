use clap::Args;
use std::io;

use crate::repo::detect_context;

#[derive(Args)]
pub struct StatusArgs {
    /// Show detailed information about packages
    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Show system-wide status (requires root)
    #[clap(long)]
    pub system: bool,
}

pub fn run(args: &StatusArgs) -> io::Result<()> {
    // detect context
    let ctx = detect_context(args.system)?;

    // show context info
    println!("Context: {}", if ctx.is_system { "system" } else { "user" });
    println!("Repository: {}", ctx.repo_path.display());
    println!("Packages: {}", ctx.pkg_path.display());
    println!();

    // show installed packages from context-aware state
    let state_path = ctx.var_path.join("installed.json");
    println!("Package state: {}", state_path.display());

    if let Ok(state) = super::state::InstalledState::load_for_context(&ctx) {
        let count = state.packages.len();
        if count > 0 {
            println!();
            println!("Installed packages: {}", count);
            for (key, pkg) in &state.packages {
                let version_count = pkg.versions.len();
                let current = pkg.current.as_deref().unwrap_or("(none)");
                if args.verbose {
                    println!("  {} - current: {}", key, current);
                    for (ver_key, ver_info) in &pkg.versions {
                        let marker = if Some(ver_key.as_str()) == pkg.current.as_deref() {
                            "*"
                        } else {
                            " "
                        };
                        println!("    {} {}", marker, ver_key);
                        println!("      installed: {}", ver_info.installed_at);
                        println!("      ref: {}", ver_info.store_ref);
                        if !ver_info.provides.is_empty() {
                            println!("      provides: {}", ver_info.provides.join(", "));
                        }
                    }
                } else {
                    println!(
                        "  {} - current: {}, versions: {}",
                        key, current, version_count
                    );
                }
            }
        } else {
            println!();
            println!("No packages installed.");
        }
    } else {
        println!();
        println!("No packages installed.");
    }

    Ok(())
}
