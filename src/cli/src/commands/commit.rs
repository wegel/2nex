use clap::Args;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use super::stage::{cleanup_staging, is_staged};
use crate::store;

const STAGING_STATE_DIR: &str = super::stage::STAGING_STATE_DIR;
const NEX_REPO: &str = "/nex/repo";

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

    // check if we're on a nex system with repo
    if !Path::new(NEX_REPO).exists() {
        // no system repo - just merge the overlay and exit staging
        println!("No system repo found. Merging overlay changes directly...");
        merge_overlay_changes()?;
        cleanup_staging()?;
        println!("Changes applied.");
        return Ok(());
    }

    // commit staged changes to a new deployment ref
    create_deployment(&message)?;

    // cleanup staging
    cleanup_staging()?;

    println!("Deployment created successfully.");

    Ok(())
}

fn merge_overlay_changes() -> io::Result<()> {
    // merge overlay upper dir contents to the actual locations

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
        return Err(io::Error::other(format!(
            "Failed to copy {} to {}",
            src, dst
        )));
    }

    Ok(())
}

fn create_deployment(message: &str) -> io::Result<()> {
    // get current deployment info
    let current_ref = get_current_deployment_ref()?;
    println!("  Current deployment: {}", current_ref);

    let staging_dir = format!("{}/commit_staging", STAGING_STATE_DIR);
    fs::create_dir_all(&staging_dir)?;

    // checkout current deployment
    println!("  Checking out current deployment...");
    store::checkout_into(NEX_REPO, &current_ref, Path::new(&staging_dir), false)?;

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
    let new_ref = format!("nex/deployments/{}", timestamp_id());

    let metadata = vec![
        ("nex.deployment.message".to_string(), message.to_string()),
        ("nex.deployment.parent".to_string(), current_ref),
    ];

    store::commit_tree(NEX_REPO, &new_ref, Path::new(&staging_dir), &metadata)?;

    println!("  Created deployment: {}", new_ref);

    // cleanup staging dir
    fs::remove_dir_all(&staging_dir)?;

    Ok(())
}

fn get_current_deployment_ref() -> io::Result<String> {
    // try to find the latest deployment ref
    let store = store::Store::open(NEX_REPO)?;
    let refs = store.refs(Some("nex/deployments/"))?;

    if let Some(latest) = refs.iter().max() {
        return Ok(latest.clone());
    }

    // fallback: try nex/base
    if store.exists("nex/base") {
        return Ok("nex/base".to_string());
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
