//! Deploy built system refs into an installed Nex sysroot.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args;
use nix::unistd::Uid;
use walkdir::WalkDir;

use crate::store::Store;

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

    let mut remount = RemountGuard::new(sysroot)?;

    prepare_temp_deployment(&mut remount, &temp_path)?;

    if let Err(error) = checkout_and_publish(&store, &args.system_ref, &temp_path, &deployment_path)
    {
        let _ = fs::remove_dir_all(&temp_path);
        return Err(error);
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
    store.checkout(system_ref, temp_path, false)?;
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

    fn remount_ro(&mut self) -> io::Result<()> {
        if !self.touched_rw {
            return Ok(());
        }
        let status = Command::new("mount")
            .arg("-o")
            .arg("remount,ro")
            .arg(self.mountpoint)
            .status()?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "failed to remount {} ro",
                self.mountpoint.display()
            )));
        }
        Ok(())
    }
}

impl Drop for RemountGuard<'_> {
    fn drop(&mut self) {
        let _ = self.remount_ro();
    }
}

#[cfg(test)]
#[path = "deploy_tests.rs"]
mod deploy_tests;
