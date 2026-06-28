use clap::Args;
use nix::unistd::Uid;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Args)]
pub struct GcArgs {
    /// Sysroot mount point (default: /sysroot)
    #[clap(long, default_value = "/sysroot")]
    pub sysroot: PathBuf,

    /// Number of most-recent deployments to keep (by serial)
    #[clap(long, default_value_t = 2)]
    pub keep: usize,

    /// Also run zub object GC on the system repo (/nex/repo)
    #[clap(long)]
    pub zub: bool,

    /// Print what would be done without deleting
    #[clap(long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
struct Deployment {
    checksum: String,
    serial: u64,
    name: String,
}

pub fn run(args: &GcArgs) -> io::Result<()> {
    let sysroot = &args.sysroot;
    let deployments_dir = sysroot.join("nex/deployments");
    let repo_dir = sysroot.join("nex/repo");

    if !deployments_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "deployments dir not found: {} (is sysroot mounted?)",
                deployments_dir.display()
            ),
        ));
    }

    let current = current_deployment_from_cmdline();
    let mut deployments = list_deployments(&deployments_dir)?;
    deployments.sort_by(|a, b| {
        a.serial
            .cmp(&b.serial)
            .then_with(|| a.checksum.cmp(&b.checksum))
    });

    if deployments.is_empty() {
        println!("No deployments found.");
        return Ok(());
    }

    // Keep N most recent by serial, plus current if present.
    let mut keep_names: HashSet<String> = HashSet::new();
    for d in deployments.iter().rev().take(args.keep) {
        keep_names.insert(d.name.clone());
    }
    if let Some(cur) = &current {
        keep_names.insert(cur.clone());
    }

    let mut remove = Vec::new();
    for d in &deployments {
        if !keep_names.contains(&d.name) {
            remove.push(d.clone());
        }
    }

    println!("Deployments dir: {}", deployments_dir.display());
    println!("Current: {}", current.as_deref().unwrap_or("(unknown)"));
    println!("Keep: {}", args.keep);
    println!();

    if remove.is_empty() {
        println!("Nothing to remove.");
    } else {
        println!("Will remove {} deployment(s):", remove.len());
        for d in &remove {
            println!("  {}", d.name);
        }
    }
    println!();

    if args.dry_run {
        if args.zub {
            println!(
                "(dry-run) would run zub object GC in {}",
                repo_dir.display()
            );
        }
        return Ok(());
    }

    let mut remount = RemountGuard::new(sysroot)?;

    // Delete deployment directories (remount sysroot rw if needed).
    for d in remove {
        let path = deployments_dir.join(&d.name);
        if let Err(e) = fs::remove_dir_all(&path) {
            if e.kind() == io::ErrorKind::ReadOnlyFilesystem {
                remount.remount_rw()?;
                fs::remove_dir_all(&path)?;
            } else {
                return Err(e);
            }
        }
    }

    if args.zub {
        if !repo_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("system repo not found: {}", repo_dir.display()),
            ));
        }
        if remount.is_ro() {
            remount.remount_rw()?;
        }
        let repo = zub::Repo::open(&repo_dir).map_err(|e| io::Error::other(e.to_string()))?;
        let stats = zub::ops::gc(&repo, false).map_err(|e| io::Error::other(e.to_string()))?;
        println!(
            "zub gc: blobs={} trees={} commits={} bytes_freed={}",
            stats.blobs_removed, stats.trees_removed, stats.commits_removed, stats.bytes_freed
        );
    }

    remount.remount_ro()?;
    Ok(())
}

fn list_deployments(dir: &Path) -> io::Result<Vec<Deployment>> {
    let mut out = Vec::new();
    for dirent in fs::read_dir(dir)? {
        let dirent = dirent?;
        let path = dirent.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some((checksum, serial)) = parse_deployment_dir_name(name) else {
            continue;
        };
        out.push(Deployment {
            checksum,
            serial,
            name: name.to_string(),
        });
    }
    Ok(out)
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

fn current_deployment_from_cmdline() -> Option<String> {
    let cmdline = fs::read_to_string("/proc/cmdline").ok()?;
    for part in cmdline.split_whitespace() {
        if let Some(v) = part.strip_prefix("zub=") {
            let base = v.rsplit('/').next().unwrap_or(v);
            if parse_deployment_dir_name(base).is_some() {
                return Some(base.to_string());
            }
        }
    }
    None
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

    fn is_ro(&self) -> bool {
        !self.touched_rw
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
