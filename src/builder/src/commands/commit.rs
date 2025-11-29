use clap::Args;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use super::stage::{cleanup_staging, is_staged};

const STAGING_STATE_DIR: &str = "/run/nex/staging";
const OSTREE_REPO: &str = "/ostree/repo";
const OSTREE_DEPLOY_DIR: &str = "/ostree/deploy/2nex";

#[derive(Args)]
pub struct CommitArgs {
    /// Commit message
    #[clap(long, short = 'm')]
    pub message: Option<String>,
}

pub fn run(args: &CommitArgs) -> io::Result<()> {
    if !is_staged() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Not in staging mode. Nothing to commit.",
        ));
    }

    // check if there are any changes
    let upper_nex = format!("{}/upper/nex", STAGING_STATE_DIR);
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);

    let has_nex_changes = fs::read_dir(&upper_nex)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
    let has_bin_changes = fs::read_dir(&upper_usr_bin)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);

    if !has_nex_changes && !has_bin_changes {
        println!("No changes to commit.");
        cleanup_staging()?;
        return Ok(());
    }

    let message = args
        .message
        .clone()
        .unwrap_or_else(|| "Package changes".to_string());
    println!("Committing changes: {}", message);

    // check if we're on an OSTree system
    if !Path::new(OSTREE_REPO).exists() {
        // not an OSTree system - just merge the overlay and exit staging
        println!("Not an OSTree system. Merging overlay changes directly...");
        merge_overlay_changes()?;
        cleanup_staging()?;
        println!("Changes applied. (Note: changes are not atomic without OSTree)");
        return Ok(());
    }

    // on an OSTree system, create a new deployment
    create_ostree_deployment(&message)?;

    // cleanup staging
    cleanup_staging()?;

    println!("Deployment created successfully.");
    println!("Reboot to activate the new deployment, or use 'ostree admin set-default' to switch.");

    Ok(())
}

fn merge_overlay_changes() -> io::Result<()> {
    // for non-OSTree systems, we need to:
    // 1. unmount the overlay
    // 2. copy upper dir contents to the actual locations

    let upper_nex = format!("{}/upper/nex", STAGING_STATE_DIR);
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);

    // unmount overlays first
    let _ = Command::new("umount").arg("/usr/bin").status();
    let _ = Command::new("umount").arg("/nex/pkg").status();

    // copy changes from upper to real locations
    if Path::new(&upper_nex).exists() {
        copy_dir_contents(&upper_nex, "/nex/pkg")?;
    }
    if Path::new(&upper_usr_bin).exists() {
        copy_dir_contents(&upper_usr_bin, "/usr/bin")?;
    }

    Ok(())
}

fn copy_dir_contents(src: &str, dst: &str) -> io::Result<()> {
    let status = Command::new("cp")
        .args(["-a", &format!("{}/.", src), dst])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to copy {} to {}", src, dst),
        ));
    }

    Ok(())
}

fn create_ostree_deployment(message: &str) -> io::Result<()> {
    // get current deployment info
    let current_ref = get_current_deployment_ref()?;
    println!("  Current deployment: {}", current_ref);

    // create a new commit with the overlay changes
    // this is complex because we need to:
    // 1. checkout current deployment
    // 2. apply overlay changes
    // 3. commit as new ref
    // 4. deploy the new ref

    let staging_dir = format!("{}/commit_staging", STAGING_STATE_DIR);
    fs::create_dir_all(&staging_dir)?;

    // checkout current deployment
    println!("  Checking out current deployment...");
    let status = Command::new("ostree")
        .args([
            "checkout",
            "--repo",
            OSTREE_REPO,
            "--user-mode",
            &current_ref,
            &staging_dir,
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to checkout current deployment",
        ));
    }

    // apply overlay changes
    println!("  Applying staged changes...");
    let upper_nex = format!("{}/upper/nex", STAGING_STATE_DIR);
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);

    if Path::new(&upper_nex).exists() {
        let nex_target = format!("{}/nex/pkg", staging_dir);
        fs::create_dir_all(&nex_target)?;
        copy_dir_contents(&upper_nex, &nex_target)?;
    }

    if Path::new(&upper_usr_bin).exists() {
        let bin_target = format!("{}/usr/bin", staging_dir);
        fs::create_dir_all(&bin_target)?;
        copy_dir_contents(&upper_usr_bin, &bin_target)?;
    }

    // commit the new tree
    println!("  Creating new commit...");
    let new_ref = format!("2nex/deployments/{}", timestamp_id());

    let status = Command::new("ostree")
        .args([
            "commit",
            "--repo",
            OSTREE_REPO,
            "--branch",
            &new_ref,
            "--subject",
            message,
            &staging_dir,
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to commit new deployment",
        ));
    }

    // deploy the new ref
    println!("  Deploying {}...", new_ref);
    let status = Command::new("ostree")
        .args(["admin", "deploy", "--os=2nex", &new_ref])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to deploy new commit. You may need to run as root.",
        ));
    }

    // cleanup staging dir
    fs::remove_dir_all(&staging_dir)?;

    Ok(())
}

fn get_current_deployment_ref() -> io::Result<String> {
    // try to get from ostree admin status
    let output = Command::new("ostree").args(["admin", "status"]).output()?;

    if output.status.success() {
        let status = String::from_utf8_lossy(&output.stdout);
        // parse the first deployment line
        for line in status.lines() {
            if line.contains("2nex") && !line.starts_with(' ') {
                // format is typically: "* 2nex <checksum>.<serial> (staged)"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Ok(parts[1].to_string());
                }
            }
        }
    }

    // fallback: try to read from deploy directory
    let deploy_dir = Path::new(OSTREE_DEPLOY_DIR).join("deploy");
    if deploy_dir.exists() {
        if let Ok(entries) = fs::read_dir(&deploy_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') {
                    return Ok(format!("2nex/deploy/{}", name));
                }
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not determine current deployment",
    ))
}

fn timestamp_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
