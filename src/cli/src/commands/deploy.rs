use clap::Args;
use nix::unistd::Uid;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    // hardlink constraint: /nex/repo and /nex/deployments must share a filesystem
    let repo_dev = fs::metadata(repo_path).map(|m| m.dev())?;
    let deploy_dev = fs::metadata(&deployments_dir).map(|m| m.dev())?;
    if repo_dev != deploy_dev {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "hardlink constraint violated: {} and {} are on different filesystems",
                repo_path.display(),
                deployments_dir.display()
            ),
        ));
    }

    let store = Store::open(repo_path)?;

    // Determine checksum: prefer explicit system checksum metadata, otherwise use commit hash.
    let checksum = match store.get_metadata(&args.system_ref, "nex.system.checksum")? {
        Some(v) => v,
        None => store.resolve_ref(&args.system_ref)?,
    };

    let next_serial = next_serial(&deployments_dir)?;
    let deployment_name = format!("{}.{}", checksum, next_serial);
    let deployment_path = deployments_dir.join(&deployment_name);

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
    println!();

    if args.dry_run {
        return Ok(());
    }

    let mut remount = RemountGuard::new(sysroot)?;

    // Ensure we can write into sysroot.
    if let Err(e) = fs::create_dir_all(&deployment_path) {
        if e.kind() == io::ErrorKind::ReadOnlyFilesystem {
            remount.remount_rw()?;
            fs::create_dir_all(&deployment_path)?;
        } else {
            return Err(e);
        }
    }

    // zub checkout into the deployment directory (hardlinks)
    store.checkout(&args.system_ref, &deployment_path, false)?;

    // Remount back to RO if we toggled it.
    remount.remount_ro()?;

    println!("Deployed: {}", deployment_name);
    println!("Reboot to activate (bootloader picks highest serial).");
    Ok(())
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
