use clap::Args;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::stage::{cleanup_staging, is_staged};
use crate::store;

const STAGING_STATE_DIR: &str = super::stage::STAGING_STATE_DIR;
/// The machine store. Resolved rather than hardcoded so a machine installed
/// before the rename, which still has `/nex/repo`, keeps working.
fn nex_repo() -> String {
    crate::repo::system_store_path()
        .to_string_lossy()
        .to_string()
}

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
    if !Path::new(&nex_repo()).exists() {
        // no system repo - just merge the overlay and exit staging
        println!("No system repo found. Merging overlay changes directly...");
        merge_overlay_changes()?;
        cleanup_staging()?;
        println!("Changes applied.");
        return Ok(());
    }

    // commit staged changes to a new deployment ref
    let new_ref = create_deployment(&message)?;

    // cleanup staging before activating: activation ends by remounting the
    // sysroot read-only, which takes the staging mount with it
    cleanup_staging()?;

    activate_deployment(&new_ref)?;

    println!("Deployment created successfully.");

    Ok(())
}

fn merge_overlay_changes() -> io::Result<()> {
    // merge overlay upper dir contents to the actual locations

    let upper_nex = format!("{}/upper/nex", STAGING_STATE_DIR);
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);

    // unmount overlays first, lazily when busy: `/nex/pkg` is always busy on a
    // nex_structure system, since FHS paths are symlinks into its capsules and
    // PID 1 holds one open. Swallowing the failure here would copy into a live
    // overlay instead of the real location.
    super::stage::unmount_overlay("/usr/bin")?;
    super::stage::unmount_overlay("/nex/pkg")?;
    super::stage::unmount_overlay("/nex/env")?;

    // copy changes from upper to real locations
    // `upper/nex` holds one directory per mounted overlay, `pkg` and `env`, so
    // its contents belong at `/nex`, not at `/nex/pkg`. Copying them a level too
    // deep produces `/nex/pkg/pkg/...` and leaves every `/usr/bin` symlink
    // dangling.
    if Path::new(&upper_nex).exists() {
        copy_dir_contents(&upper_nex, "/nex")?;
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

fn create_deployment(message: &str) -> io::Result<String> {
    // get current deployment info
    let current_ref = get_current_deployment_ref()?;
    println!("  Current deployment: {}", current_ref);

    let staging_dir = format!("{}/commit_staging", STAGING_STATE_DIR);
    fs::create_dir_all(&staging_dir)?;

    // checkout current deployment
    println!("  Checking out current deployment...");
    store::checkout_into(&nex_repo(), &current_ref, Path::new(&staging_dir), false)?;

    // apply overlay changes
    println!("  Applying staged changes...");
    let upper_nex = format!("{}/upper/nex", STAGING_STATE_DIR);
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);

    // As in `merge_overlay_changes`: the children of `upper/nex` are the
    // overlay names, `pkg` and `env`, so they belong under `nex/`, not under
    // `nex/pkg/`.
    if Path::new(&upper_nex).exists() {
        let nex_target = format!("{}/nex", staging_dir);
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

    store::commit_tree(&nex_repo(), &new_ref, Path::new(&staging_dir), &metadata)?;

    println!("  Created deployment: {}", new_ref);

    // cleanup staging dir
    fs::remove_dir_all(&staging_dir)?;

    Ok(new_ref)
}

/// Make a committed deployment bootable.
///
/// A store ref is not bootable on its own: the bootloader reads the on-disk
/// `nex/deployments` directory and picks the highest serial, so a commit that
/// stops at the ref survives in the store and never boots, which makes the
/// "keep it" half of staging inert. `nex deploy` already does this, so reuse it
/// rather than growing a second implementation. The ref is named by timestamp
/// and carries no checksum metadata, hence `allow_commit_hash`.
///
/// This must run after every staging teardown. `deploy` remounts the sysroot
/// read-write and back, and `/sysroot`, `/nex/staging`, and `/nex/deployments`
/// are separate mounts of one block device, so restoring read-only takes them
/// all with it. Anything that still needs to write would fail with `EROFS`.
fn activate_deployment(new_ref: &str) -> io::Result<()> {
    println!("  Activating for next boot...");
    super::deploy::run(&super::deploy::DeployArgs {
        system_ref: new_ref.to_string(),
        sysroot: PathBuf::from("/sysroot"),
        repo: Some(crate::repo::system_store_path()),
        dry_run: false,
        force: false,
        allow_commit_hash: true,
    })
}

fn get_current_deployment_ref() -> io::Result<String> {
    // try to find the latest deployment ref
    let store = store::Store::open(nex_repo())?;
    // The trailing `*` is required. `Store::refs` filters with a glob pattern
    // (`zub::list_refs_matching`), and a pattern with no wildcard matches only
    // that exact literal, so a bare prefix returns nothing however many refs
    // exist beneath it.
    let refs = store.refs(Some("nex/deployments/*"))?;

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
