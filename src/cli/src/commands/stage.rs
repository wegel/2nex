use clap::Args;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

// staging must be on the same filesystem as /nex/store for hardlinks to work
pub const STAGING_STATE_DIR: &str = "/nex/staging";
const NEX_PKG_DIR: &str = "/nex/pkg";
const USR_BIN_DIR: &str = "/usr/bin";

#[derive(Args)]
pub struct StageArgs {
    /// Force staging even if already staged
    #[clap(long)]
    pub force: bool,
}

pub fn run(args: &StageArgs) -> io::Result<()> {
    // check if already staged
    if is_staged() && !args.force {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Already in staging mode. Use 'nex discard' to exit or --force to restage.",
        ));
    }

    // clean up any existing staging state if forcing
    if args.force && is_staged() {
        println!("Cleaning up existing staging state...");
        cleanup_staging()?;
    }

    println!("Entering staging mode...");

    // create staging directories
    // physical_root = /nex/staging/upper
    // checkout writes to physical_root/nex/pkg = /nex/staging/upper/nex/pkg
    fs::create_dir_all(STAGING_STATE_DIR)?;
    let upper_nex_pkg = format!("{}/upper/nex/pkg", STAGING_STATE_DIR);
    let upper_nex_env = format!("{}/upper/nex/env", STAGING_STATE_DIR);
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);
    let work_nex_pkg = format!("{}/work/nex_pkg", STAGING_STATE_DIR);
    let work_nex_env = format!("{}/work/nex_env", STAGING_STATE_DIR);
    let work_usr_bin = format!("{}/work/usr_bin", STAGING_STATE_DIR);

    fs::create_dir_all(&upper_nex_pkg)?;
    fs::create_dir_all(&upper_nex_env)?;
    fs::create_dir_all(&upper_usr_bin)?;
    fs::create_dir_all(&work_nex_pkg)?;
    fs::create_dir_all(&work_nex_env)?;
    fs::create_dir_all(&work_usr_bin)?;

    // NOTE: /nex/pkg and /nex/env overlays are mounted AFTER checkout by mount_nex_overlays()
    // This avoids overlayfs cache issues when writing directly to upperdir

    // mount overlayfs on /usr/bin (needed for symlinks)
    mount_overlay(USR_BIN_DIR, &upper_usr_bin, &work_usr_bin, USR_BIN_DIR)?;
    println!("  Overlay mounted on {}", USR_BIN_DIR);

    // write state file
    fs::write(
        format!("{}/active", STAGING_STATE_DIR),
        format!("staged_at={}\n", chrono_now()),
    )?;

    println!("Staging mode active. Changes will be isolated until 'nex commit' or 'nex discard'.");
    println!("Use 'nex install <pkg>' to install packages.");

    Ok(())
}

pub fn is_staged() -> bool {
    Path::new(&format!("{}/active", STAGING_STATE_DIR)).exists()
}

/// Returns the physical upper directory path when in staging mode.
/// Materializer will append nex/pkg or nex/env to this path.
pub fn staging_upper_dir() -> Option<String> {
    if is_staged() {
        Some(format!("{}/upper", STAGING_STATE_DIR))
    } else {
        None
    }
}

/// Mount /nex/pkg and /nex/env overlays after checkout completes.
/// These are mounted AFTER writing to upperdir to avoid overlayfs cache issues.
pub fn mount_nex_overlays() -> io::Result<()> {
    if !is_staged() {
        return Ok(());
    }

    let upper_nex_pkg = format!("{}/upper/nex/pkg", STAGING_STATE_DIR);
    let upper_nex_env = format!("{}/upper/nex/env", STAGING_STATE_DIR);
    let work_nex_pkg = format!("{}/work/nex_pkg", STAGING_STATE_DIR);
    let work_nex_env = format!("{}/work/nex_env", STAGING_STATE_DIR);

    // mount /nex/pkg overlay (create target if it doesn't exist)
    if !Path::new(NEX_PKG_DIR).exists() {
        fs::create_dir_all(NEX_PKG_DIR)?;
    }
    mount_overlay(NEX_PKG_DIR, &upper_nex_pkg, &work_nex_pkg, NEX_PKG_DIR)?;

    // mount /nex/env overlay (create target if it doesn't exist)
    let nex_env_dir = "/nex/env";
    if !Path::new(nex_env_dir).exists() {
        fs::create_dir_all(nex_env_dir)?;
    }
    mount_overlay(nex_env_dir, &upper_nex_env, &work_nex_env, nex_env_dir)?;

    Ok(())
}

fn mount_overlay(lower: &str, upper: &str, work: &str, target: &str) -> io::Result<()> {
    let options = format!("lowerdir={},upperdir={},workdir={}", lower, upper, work);

    let status = Command::new("mount")
        .args(["-t", "overlay", "overlay", "-o", &options, target])
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "Failed to mount overlay on {}",
            target
        )));
    }

    Ok(())
}

pub fn cleanup_staging() -> io::Result<()> {
    // unmount overlays (in reverse order)
    unmount_overlay(USR_BIN_DIR)?;
    unmount_overlay("/nex/env")?;
    unmount_overlay(NEX_PKG_DIR)?;

    // clear staging state, which is safe only once every overlay above it is
    // gone: otherwise this recurses into a live upperdir
    clear_staging_state()?;

    Ok(())
}

/// Empty the staging directory without removing the directory itself.
///
/// `/nex/staging` is a mount point that `base/nex-systemd.yaml` creates, and
/// `rmdir` needs write permission on the target's parent rather than on the
/// target. The parent here is `/nex`, on the read-only deployment root, so
/// removing the directory fails with `EROFS` however empty it is. Only its
/// contents are ours to delete.
fn clear_staging_state() -> io::Result<()> {
    let staging = Path::new(STAGING_STATE_DIR);
    if !staging.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(staging)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

/// Unmount a staging overlay, lazily when the mount point is busy.
///
/// `/nex/pkg` is always busy on a `nex_structure` system. FHS paths are
/// symlinks into the capsules beneath it, so every running process holds
/// something there: PID 1 keeps an open descriptor on
/// `/nex/pkg/core/init/systemd/<version>/<hash>/usr/lib/systemd/systemd-executor`,
/// and nothing can stop PID 1 to release it. A lazy unmount detaches the mount
/// immediately and lets the kernel release it when the last user goes away,
/// which is exactly this case.
///
/// Failures used to be discarded, so a busy mount stayed mounted and the caller
/// then tried to delete through it, reporting a read-only filesystem instead of
/// the real cause.
pub(super) fn unmount_overlay(target: &str) -> io::Result<()> {
    if !is_mounted(target) {
        return Ok(());
    }

    if Command::new("umount").arg(target).status()?.success() {
        return Ok(());
    }

    let lazy = Command::new("umount").arg("-l").arg(target).status()?;
    if lazy.success() {
        return Ok(());
    }

    Err(io::Error::other(format!(
        "could not unmount the staging overlay on {}",
        target
    )))
}

fn is_mounted(target: &str) -> bool {
    let Ok(mounts) = fs::read_to_string("/proc/mounts") else {
        return false;
    };
    mounts
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .any(|mount_point| mount_point == target)
}

fn chrono_now() -> String {
    // simple timestamp without external dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
