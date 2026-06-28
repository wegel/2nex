use clap::Args;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Args)]
pub struct DeploymentsArgs {
    /// Alternate deployments directory (default: /nex/deployments)
    #[clap(long)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct DeploymentEntry {
    checksum: String,
    serial: u64,
    path: PathBuf,
    mtime_secs: Option<u64>,
}

pub fn run(args: &DeploymentsArgs) -> io::Result<()> {
    let deployments_dir = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from("/nex/deployments"));
    if !deployments_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("deployments dir not found: {}", deployments_dir.display()),
        ));
    }

    let current = current_deployment_from_cmdline();

    let mut entries = Vec::new();
    for dirent in fs::read_dir(&deployments_dir)? {
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

        let mtime_secs = dirent
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        entries.push(DeploymentEntry {
            checksum,
            serial,
            path,
            mtime_secs,
        });
    }

    entries.sort_by(|a, b| {
        a.serial
            .cmp(&b.serial)
            .then_with(|| a.checksum.cmp(&b.checksum))
    });

    if entries.is_empty() {
        println!("No deployments found in {}", deployments_dir.display());
        return Ok(());
    }

    println!("Deployments: {}", deployments_dir.display());
    if let Some(cur) = &current {
        println!("Current: {}", cur.display());
    }
    println!();

    for entry in entries {
        let marker = if current
            .as_ref()
            .and_then(|p| p.file_name().and_then(|s| s.to_str()))
            == entry.path.file_name().and_then(|s| s.to_str())
        {
            "*"
        } else {
            " "
        };

        let ts = entry
            .mtime_secs
            .map(|s| format!("mtime={}", s))
            .unwrap_or_else(|| "mtime=?".to_string());
        println!("{} {}.{} ({})", marker, entry.checksum, entry.serial, ts);
    }

    Ok(())
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
    if serial_str.is_empty()
        || !serial_str
            .as_bytes()
            .iter()
            .all(|c| matches!(c, b'0'..=b'9'))
    {
        return None;
    }
    let serial = serial_str.parse::<u64>().ok()?;
    Some((checksum.to_string(), serial))
}

fn current_deployment_from_cmdline() -> Option<PathBuf> {
    let cmdline = fs::read_to_string("/proc/cmdline").ok()?;
    let mut zub = None;
    for part in cmdline.split_whitespace() {
        if let Some(v) = part.strip_prefix("zub=") {
            zub = Some(v);
            break;
        }
    }

    let zub = zub?;
    let path = Path::new(zub);
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        Some(PathBuf::from("/").join(path))
    }
}
