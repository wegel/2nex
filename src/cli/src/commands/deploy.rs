//! Deploy built system refs into an installed Nex sysroot.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args;
use nix::unistd::Uid;
use walkdir::WalkDir;

use crate::store::{self, Store};

#[derive(Args)]
pub struct DeployArgs {
    /// System ref to deploy (e.g. systems/desktop-vwl/0.0.2)
    pub system_ref: String,

    /// Sysroot mount point (default: /sysroot)
    #[clap(long, default_value = "/sysroot")]
    pub sysroot: PathBuf,

    /// System zub repo (default: /nex/repo)
    #[clap(long, default_value = "/nex/repo")]
    pub repo: PathBuf,

    /// Print what would be done without writing
    #[clap(long)]
    pub dry_run: bool,

    /// Deploy even when the checksum is already present
    #[clap(long)]
    pub force: bool,

    /// Allow refs without checksum metadata
    #[clap(long)]
    pub allow_commit_hash: bool,
}

pub fn run(args: &DeployArgs) -> io::Result<()> {
    let sysroot = &args.sysroot;
    let deployments_dir = sysroot.join("nex/deployments");
    let repo_path = &args.repo;
    let mut remount = RemountGuard::new(sysroot)?;

    if !deployments_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "deployments dir not found: {} (is sysroot mounted?)",
                deployments_dir.display()
            ),
        ));
    }
    if !repo_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("repo not found: {}", repo_path.display()),
        ));
    }

    if !args.dry_run {
        remount.remount_rw()?;
        remount.remount_path_rw(repo_path)?;
    }

    let store = Store::open(repo_path)?;
    let commit_hash = store.resolve_ref(&args.system_ref)?;

    let checksum = deployment_checksum(&store, &args.system_ref, &commit_hash, args)?;
    reject_existing_checksum(&deployments_dir, &checksum, args.force)?;

    let next_serial = next_serial(&deployments_dir)?;
    let deployment_name = format!("{}.{}", checksum, next_serial);
    let deployment_path = deployments_dir.join(&deployment_name);
    let temp_path = deployments_dir.join(format!(".{}.tmp", deployment_name));

    if deployment_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("deployment already exists: {}", deployment_path.display()),
        ));
    }

    println!("System ref:   {}", args.system_ref);
    println!("Checksum:    {}", checksum);
    println!("Next serial: {}", next_serial);
    println!("Target:      {}", deployment_path.display());
    println!("Staging:     {}", temp_path.display());
    println!();

    if args.dry_run {
        return Ok(());
    }

    prepare_temp_deployment(&mut remount, &temp_path)?;

    if let Err(error) = checkout_and_publish(&store, &args.system_ref, &temp_path, &deployment_path)
    {
        let _ = fs::remove_dir_all(&temp_path);
        return Err(error);
    }

    // Publish the store ref that later operations look for. `nex commit` needs
    // to check the running deployment out again before layering staged changes
    // on top; without this ref it fails with "Could not determine current
    // deployment" on every machine.
    let deployment_ref = format!("nex/deployments/{}", deployment_name);
    if let Err(error) = store::publish_ref(&repo_path.to_string_lossy(), &deployment_ref, &args.system_ref) {
        eprintln!("Warning: could not publish {}: {}", deployment_ref, error);
    }

    remount.remount_ro()?;

    println!("Deployed: {}", deployment_name);
    println!("Reboot to activate (bootloader picks highest serial).");
    Ok(())
}

fn deployment_checksum(
    store: &Store,
    system_ref: &str,
    commit_hash: &str,
    args: &DeployArgs,
) -> io::Result<String> {
    match checksum_metadata(store, system_ref)? {
        Some(checksum) if is_checksum(&checksum) => Ok(checksum),
        Some(checksum) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid checksum metadata for {}: {}", system_ref, checksum),
        )),
        None if args.allow_commit_hash => Ok(commit_hash.to_string()),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} is missing nex.system.checksum or nex.build.checksum metadata; pass --allow-commit-hash to deploy by commit hash",
                system_ref
            ),
        )),
    }
}

fn checksum_metadata(store: &Store, system_ref: &str) -> io::Result<Option<String>> {
    match store.get_metadata(system_ref, "nex.system.checksum")? {
        Some(checksum) => Ok(Some(checksum)),
        None => store.get_metadata(system_ref, "nex.build.checksum"),
    }
}

fn reject_existing_checksum(deployments_dir: &Path, checksum: &str, force: bool) -> io::Result<()> {
    if force {
        return Ok(());
    }

    for dirent in fs::read_dir(deployments_dir)? {
        let dirent = dirent?;
        let Some(name) = dirent.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Some((existing_checksum, _)) = parse_deployment_dir_name(&name) else {
            continue;
        };
        if existing_checksum == checksum {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "deployment for checksum {} already exists; pass --force to deploy another serial",
                    checksum
                ),
            ));
        }
    }

    Ok(())
}

fn prepare_temp_deployment(remount: &mut RemountGuard<'_>, temp_path: &Path) -> io::Result<()> {
    if let Err(error) = replace_temp_deployment(temp_path) {
        if error.kind() == io::ErrorKind::ReadOnlyFilesystem {
            remount.remount_rw()?;
            replace_temp_deployment(temp_path)?;
        } else {
            return Err(error);
        }
    }
    Ok(())
}

fn replace_temp_deployment(temp_path: &Path) -> io::Result<()> {
    if temp_path.exists() {
        fs::remove_dir_all(temp_path)?;
    }
    fs::create_dir_all(temp_path)
}

fn checkout_and_publish(
    store: &Store,
    system_ref: &str,
    temp_path: &Path,
    deployment_path: &Path,
) -> io::Result<()> {
    store.checkout_immutable(system_ref, temp_path, false)?;
    sync_tree(temp_path)?;
    fs::rename(temp_path, deployment_path)?;
    if let Some(parent) = deployment_path.parent() {
        sync_path(parent)?;
    }
    Ok(())
}

fn sync_tree(root: &Path) -> io::Result<()> {
    let mut first_error = None;
    for entry in WalkDir::new(root).contents_first(true) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                first_error.get_or_insert_with(|| io::Error::other(error.to_string()));
                continue;
            }
        };
        if entry.file_type().is_symlink() {
            continue;
        }
        if let Err(error) = sync_path(entry.path()) {
            first_error.get_or_insert(error);
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn sync_path(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

fn is_checksum(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
}

fn next_serial(deployments_dir: &Path) -> io::Result<u64> {
    let mut max_serial: u64 = 0;
    for dirent in fs::read_dir(deployments_dir)? {
        let dirent = dirent?;
        let path = dirent.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some((_, serial)) = parse_deployment_dir_name(name) else {
            continue;
        };
        max_serial = max_serial.max(serial);
    }
    Ok(max_serial.saturating_add(1))
}

fn parse_deployment_dir_name(name: &str) -> Option<(String, u64)> {
    let (checksum, serial_str) = name.split_once('.')?;
    if checksum.len() != 64 {
        return None;
    }
    if !checksum
        .as_bytes()
        .iter()
        .all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    let serial = serial_str.parse::<u64>().ok()?;
    Some((checksum.to_string(), serial))
}

struct RemountGuard<'a> {
    mountpoint: &'a Path,
    is_root: bool,
    touched_rw: bool,
}

impl<'a> RemountGuard<'a> {
    fn new(mountpoint: &'a Path) -> io::Result<Self> {
        Ok(Self {
            mountpoint,
            is_root: Uid::effective().is_root(),
            touched_rw: false,
        })
    }

    fn remount_rw(&mut self) -> io::Result<()> {
        if !is_mountpoint(self.mountpoint)? {
            return Ok(());
        }
        if !self.is_root {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "sysroot is read-only; remount requires root",
            ));
        }
        let status = Command::new("mount")
            .arg("-o")
            .arg("remount,rw")
            .arg(self.mountpoint)
            .status()?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "failed to remount {} rw",
                self.mountpoint.display()
            )));
        }
        self.touched_rw = true;
        Ok(())
    }

    fn remount_path_rw(&self, path: &Path) -> io::Result<()> {
        if !is_mountpoint(path)? {
            return Ok(());
        }
        if !self.is_root {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{} is read-only; remount requires root", path.display()),
            ));
        }
        let status = Command::new("mount")
            .arg("-o")
            .arg("remount,rw")
            .arg(path)
            .status()?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "failed to remount {} rw",
                path.display()
            )));
        }
        Ok(())
    }

    /// Restore the mount to read-only, retrying while it is still busy.
    ///
    /// A lazy unmount elsewhere detaches its mount immediately but the kernel
    /// releases the underlying device asynchronously, and `/`, `/sysroot`,
    /// `/nex/staging`, and `/nex/deployments` are separate mounts of one block
    /// device. A remount issued while that release is still in flight is
    /// refused as busy, so a short retry usually settles it.
    ///
    /// Failure is reported, not fatal. By the time this runs the deployment is
    /// already written and complete, so returning an error here would tell the
    /// caller their operation failed when it succeeded, and would leave the new
    /// deployment behind as if it were debris. The machine stays writable until
    /// the next boot, which is worth a loud warning rather than a false
    /// failure.
    fn remount_ro(&mut self) -> io::Result<()> {
        if !self.touched_rw {
            return Ok(());
        }

        for attempt in 0..5 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            let status = Command::new("mount")
                .arg("-o")
                .arg("remount,ro")
                .arg(self.mountpoint)
                .status()?;
            if status.success() {
                return Ok(());
            }
        }

        eprintln!(
            "Warning: {} is still writable; could not restore it to read-only. \
             The deployment is complete. A reboot restores the read-only mount.",
            self.mountpoint.display()
        );
        Ok(())
    }
}

fn is_mountpoint(path: &Path) -> io::Result<bool> {
    let path = path.to_string_lossy();
    let mountinfo = fs::read_to_string("/proc/self/mountinfo")?;
    for line in mountinfo.lines() {
        let Some(mountpoint) = line.split_whitespace().nth(4) else {
            continue;
        };
        if unescape_mountinfo_path(mountpoint) == path {
            return Ok(true);
        }
    }
    Ok(false)
}

fn unescape_mountinfo_path(path: &str) -> String {
    let mut output = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        let mut octal = String::new();
        for _ in 0..3 {
            let Some(next) = chars.peek().copied() else {
                break;
            };
            if !matches!(next, '0'..='7') {
                break;
            }
            octal.push(next);
            chars.next();
        }
        if octal.len() == 3 {
            if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                output.push(byte as char);
                continue;
            }
        }
        output.push('\\');
        output.push_str(&octal);
    }
    output
}

impl Drop for RemountGuard<'_> {
    fn drop(&mut self) {
        let _ = self.remount_ro();
    }
}

#[cfg(test)]
#[path = "deploy_tests.rs"]
mod deploy_tests;
