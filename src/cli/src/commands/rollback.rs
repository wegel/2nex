use clap::Args;
use nix::unistd::Uid;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::WalkDir;

#[derive(Args)]
pub struct RollbackArgs {
    /// Sysroot mount point (default: /sysroot)
    #[clap(long, default_value = "/sysroot")]
    pub sysroot: PathBuf,

    /// Roll back to this deployment (format: <checksum>.<serial>)
    #[clap(long)]
    pub to: Option<String>,

    /// Don't prompt for confirmation
    #[clap(long, short = 'y')]
    pub yes: bool,

    /// Print what would be done without writing
    #[clap(long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
struct Deployment {
    checksum: String,
    serial: u64,
    name: String,
}

pub fn run(args: &RollbackArgs) -> io::Result<()> {
    let sysroot = &args.sysroot;
    let deployments_dir = sysroot.join("nex/deployments");
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
    let deployments = list_deployments(&deployments_dir)?;
    if deployments.len() < 2 && args.to.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "not enough deployments to rollback (need at least 2)",
        ));
    }

    let max_serial = deployments
        .iter()
        .map(|d| d.serial)
        .max()
        .unwrap_or_default();
    let next_serial = max_serial.saturating_add(1);

    let target = match &args.to {
        Some(name) => deployments
            .iter()
            .find(|d| &d.name == name)
            .cloned()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("deployment not found: {}", name),
                )
            })?,
        None => choose_previous(&deployments, current.as_deref())?,
    };

    let new_name = format!("{}.{}", target.checksum, next_serial);
    let src = deployments_dir.join(&target.name);
    let dst = deployments_dir.join(&new_name);
    let temp = deployments_dir.join(format!(".{}.tmp", new_name));

    if dst.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("rollback deployment already exists: {}", dst.display()),
        ));
    }

    println!("Current:  {}", current.as_deref().unwrap_or("(unknown)"));
    println!("Target:   {}", target.name);
    println!("New:      {}", new_name);
    println!("Source:   {}", src.display());
    println!("Dest:     {}", dst.display());
    println!("Staging:  {}", temp.display());
    println!();

    if args.dry_run {
        return Ok(());
    }

    if !args.yes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "rollback requires confirmation; re-run with --yes",
        ));
    }

    let mut remount = RemountGuard::new(sysroot)?;

    prepare_temp_deployment(&mut remount, &temp)?;

    copy_and_publish_rollback(&src, &temp, &dst)?;

    remount.remount_ro()?;

    println!("Rollback deployment created: {}", new_name);
    println!("Reboot to activate (bootloader picks highest serial).");
    Ok(())
}

fn prepare_temp_deployment(remount: &mut RemountGuard<'_>, temp: &Path) -> io::Result<()> {
    if let Err(error) = replace_temp_deployment(temp) {
        if error.kind() == io::ErrorKind::ReadOnlyFilesystem {
            remount.remount_rw()?;
            replace_temp_deployment(temp)?;
        } else {
            return Err(error);
        }
    }

    Ok(())
}

fn replace_temp_deployment(temp: &Path) -> io::Result<()> {
    if temp.exists() {
        fs::remove_dir_all(temp)?;
    }
    fs::create_dir_all(temp)
}

fn copy_and_publish_rollback(src: &Path, temp: &Path, dst: &Path) -> io::Result<()> {
    copy_and_publish_rollback_with_command(Path::new("cp"), src, temp, dst)
}

fn copy_and_publish_rollback_with_command(
    copy_command: &Path,
    src: &Path,
    temp: &Path,
    dst: &Path,
) -> io::Result<()> {
    if let Err(error) = copy_and_publish_prepared_rollback(copy_command, src, temp, dst) {
        let _ = fs::remove_dir_all(temp);
        return Err(error);
    }
    Ok(())
}

fn copy_and_publish_prepared_rollback(
    copy_command: &Path,
    src: &Path,
    temp: &Path,
    dst: &Path,
) -> io::Result<()> {
    hardlink_copy_deployment(copy_command, src, temp)?;
    sync_tree(temp)?;
    fs::rename(temp, dst)?;
    if let Some(parent) = dst.parent() {
        sync_path(parent)?;
    }
    Ok(())
}

fn hardlink_copy_deployment(copy_command: &Path, src: &Path, temp: &Path) -> io::Result<()> {
    let status = Command::new(copy_command)
        .arg("-a")
        .arg("-l")
        .arg(src.join("."))
        .arg(temp)
        .status()?;
    if !status.success() {
        return Err(io::Error::other(
            "failed to create rollback deployment (cp -a -l)",
        ));
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

fn choose_previous(deployments: &[Deployment], current: Option<&str>) -> io::Result<Deployment> {
    let mut sorted = deployments.to_vec();
    sorted.sort_by(|a, b| {
        a.serial
            .cmp(&b.serial)
            .then_with(|| a.checksum.cmp(&b.checksum))
    });

    if let Some(cur) = current {
        if let Some(pos) = sorted.iter().position(|d| d.name == cur) {
            if pos == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "current deployment is the oldest; no rollback target",
                ));
            }
            return Ok(sorted[pos - 1].clone());
        }
    }

    // fallback: second-highest by serial
    if sorted.len() >= 2 {
        Ok(sorted[sorted.len() - 2].clone())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "not enough deployments to rollback",
        ))
    }
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
#[path = "rollback_tests.rs"]
mod rollback_tests;
