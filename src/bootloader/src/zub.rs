//! Zub deployment discovery

use alloc::string::String;
use alloc::vec::Vec;

use crate::ext4::Ext4Fs;
use crate::BootError;

const NEX_DEPLOYMENTS_DIR: &str = "/nex/deployments";

/// represents a discovered zub deployment
pub struct Deployment {
    /// commit checksum (64 hex chars)
    pub checksum: String,
    /// deployment serial (for rollback ordering)
    pub serial: u32,
    /// full path to deployment root
    pub path: String,
}

/// find the default (most recent) zub deployment
pub fn find_default_deployment(fs: &Ext4Fs) -> Result<Deployment, BootError> {
    log::debug!("zub: scanning {}", NEX_DEPLOYMENTS_DIR);

    // read deployment directory
    let entries = fs.read_dir(NEX_DEPLOYMENTS_DIR).map_err(|e| {
        log::error!("zub: failed to read {}: {:?}", NEX_DEPLOYMENTS_DIR, e);
        BootError::NoDeployment
    })?;

    // parse deployment entries and collect valid ones
    let mut deployments: Vec<Deployment> = entries
        .into_iter()
        .filter_map(|entry| {
            if !entry.is_dir {
                return None;
            }
            parse_deployment_name(&entry.name)
        })
        .collect();

    if deployments.is_empty() {
        log::error!("zub: no valid deployments found in {}", NEX_DEPLOYMENTS_DIR);
        return Err(BootError::NoDeployment);
    }

    // sort by serial descending (highest = most recent)
    deployments.sort_by(|a, b| b.serial.cmp(&a.serial));

    log::info!(
        "zub: found {} deployment(s), using {}.{}",
        deployments.len(),
        deployments[0].checksum,
        deployments[0].serial
    );

    Ok(deployments.remove(0))
}

/// parse a deployment directory name into a Deployment struct
/// format: <checksum>.<serial> where checksum is 64 hex chars
fn parse_deployment_name(name: &str) -> Option<Deployment> {
    // find the last dot
    let dot_pos = name.rfind('.')?;

    let checksum = &name[..dot_pos];
    let serial_str = &name[dot_pos + 1..];

    // validate checksum: must be exactly 64 hex characters
    if checksum.len() != 64 {
        return None;
    }
    if !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    // parse serial number
    let serial: u32 = serial_str.parse().ok()?;

    Some(Deployment {
        checksum: String::from(checksum),
        serial,
        path: alloc::format!("{}/{}", NEX_DEPLOYMENTS_DIR, name),
    })
}

/// find kernel in a deployment
pub fn find_kernel_path(fs: &Ext4Fs, deployment: &Deployment) -> Option<String> {
    // check kernel locations in order:
    // 1. /nex/pkg/core/kernel/linux/<version>/<hash>/boot/vmlinuz-* (nex layout)
    // 2. /boot/vmlinuz-<kver> or /boot/vmlinuz (legacy)

    // try /nex/pkg/core/kernel/linux/<version>/<hash>/boot/vmlinuz-*
    let kernel_pkg_dir = alloc::format!("{}/nex/pkg/core/kernel/linux", deployment.path);
    if let Ok(versions) = fs.read_dir(&kernel_pkg_dir) {
        for version_entry in versions {
            if version_entry.is_dir && !version_entry.name.starts_with('.') {
                let version_dir = alloc::format!("{}/{}", kernel_pkg_dir, version_entry.name);
                if let Ok(hashes) = fs.read_dir(&version_dir) {
                    for hash_entry in hashes {
                        if hash_entry.is_dir && !hash_entry.name.starts_with('.') {
                            let boot_dir =
                                alloc::format!("{}/{}/boot", version_dir, hash_entry.name);
                            if let Ok(boot_entries) = fs.read_dir(&boot_dir) {
                                for boot_entry in boot_entries {
                                    if boot_entry.name.starts_with("vmlinuz") && !boot_entry.is_dir
                                    {
                                        return Some(alloc::format!(
                                            "{}/{}",
                                            boot_dir,
                                            boot_entry.name
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // fallback: try /boot/vmlinuz* (legacy layout)
    let boot_dir = alloc::format!("{}/boot", deployment.path);
    if let Ok(entries) = fs.read_dir(&boot_dir) {
        for entry in entries {
            if entry.name.starts_with("vmlinuz") && !entry.is_dir {
                return Some(alloc::format!("{}/{}", boot_dir, entry.name));
            }
        }
    }

    None
}
