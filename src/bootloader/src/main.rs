//! 2nex zub-aware UEFI bootloader
//!
//! boots the default zub deployment from an ext4 partition.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use uefi::prelude::*;

mod cpio;
mod disk;
mod ext4;
mod initrd;
mod kernel;
mod linux_efi;
mod luks2;
mod passphrase;
mod tpm;
mod zub;

/// bootloader entry point
#[entry]
fn main() -> Status {
    // initialize UEFI services and allocator
    uefi::helpers::init().expect("failed to init UEFI helpers");

    log::info!("2nex bootloader v{}", env!("CARGO_PKG_VERSION"));

    match boot_sequence() {
        Ok(()) => Status::SUCCESS,
        Err(e) => {
            log::error!("boot failed: {:?}", e);
            // wait for keypress before reboot
            log::info!("press any key to reboot...");
            uefi::boot::stall(5_000_000); // 5 seconds
            Status::LOAD_ERROR
        }
    }
}

fn boot_sequence() -> Result<(), BootError> {
    log::info!("searching for root partition...");

    // find root partition (may be LUKS2 encrypted)
    let root_disk = disk::find_root_partition()?;
    log::info!("found root partition: {}", root_disk.partuuid);

    // check if partition is LUKS2 encrypted
    let fs = if luks2::is_luks2(&root_disk).unwrap_or(false) {
        log::info!("detected LUKS2 encrypted partition");

        // unlock LUKS2 volume
        let volume = luks2::unlock(&root_disk).map_err(|e| {
            log::error!("LUKS2 unlock failed: {}", e);
            BootError::Luks2Error
        })?;

        // mount ext4 from decrypted volume
        ext4::mount_from_reader(alloc::boxed::Box::new(volume.reader))?
    } else {
        // mount ext4 directly (unencrypted)
        ext4::mount(&root_disk)?
    };
    log::info!("mounted ext4 filesystem");

    // find default zub deployment
    let deployment = zub::find_default_deployment(&fs)?;
    log::info!(
        "found deployment: {}.{}",
        deployment.checksum,
        deployment.serial
    );

    // load kernel from deployment
    let kernel_data = kernel::load_from_deployment(&fs, &deployment)?;
    log::info!("loaded kernel ({} bytes)", kernel_data.kernel.len());

    // check for extra boot modules and install initrd protocol if needed
    if let Some(initramfs) = load_boot_modules(&fs, &deployment) {
        log::info!("installing initrd protocol ({} bytes)", initramfs.len());
        if let Err(e) = initrd::install_initrd_protocol(initramfs) {
            log::warn!("failed to install initrd protocol: {:?}", e);
        }
    }

    // build kernel command line
    let cmdline = build_cmdline(&root_disk, &deployment);
    log::info!("cmdline: {}", cmdline);

    // boot the kernel
    kernel::boot(kernel_data, &cmdline)
}

/// load extra boot modules from /etc/boot-modules.conf if present
fn load_boot_modules(fs: &ext4::Ext4Fs, deployment: &zub::Deployment) -> Option<Vec<u8>> {
    let config_path = alloc::format!("{}/etc/boot-modules.conf", deployment.path);

    // read config file
    let config_content = match fs.read_file(&config_path) {
        Ok(data) => data,
        Err(_) => return None, // no config file, no extra modules needed
    };

    let config_str = core::str::from_utf8(&config_content).ok()?;
    log::info!("boot-modules: found config with {} bytes", config_content.len());

    // parse module paths (one per line, skip empty/comments)
    let module_paths: Vec<&str> = config_str
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    if module_paths.is_empty() {
        return None;
    }

    log::info!("boot-modules: loading {} module(s)", module_paths.len());

    // load each module
    let mut modules: Vec<(&str, Vec<u8>)> = Vec::new();
    for module_path in &module_paths {
        // module path is relative to deployment root
        let full_path = if module_path.starts_with('/') {
            alloc::format!("{}{}", deployment.path, module_path)
        } else {
            alloc::format!("{}/{}", deployment.path, module_path)
        };

        match fs.read_file(&full_path) {
            Ok(data) => {
                // extract just the filename for the cpio
                let filename = module_path.rsplit('/').next().unwrap_or(module_path);
                log::info!("boot-modules: loaded {} ({} bytes)", filename, data.len());
                modules.push((filename, data));
            }
            Err(e) => {
                log::warn!("boot-modules: failed to load {}: {:?}", module_path, e);
            }
        }
    }

    if modules.is_empty() {
        return None;
    }

    // build cpio archive
    Some(cpio::build_module_initramfs(&modules))
}

fn build_cmdline(disk: &disk::RootPartition, deployment: &zub::Deployment) -> String {
    alloc::format!(
        "root=PARTUUID={} zub={} ro console=ttyS0,115200n8 earlycon=uart8250,io,0x3f8,115200n8",
        disk.partuuid,
        deployment.path
    )
}

#[derive(Debug)]
pub enum BootError {
    DiskNotFound,
    PartitionNotFound,
    Ext4Error(ext4::Ext4Error),
    Luks2Error,
    NoDeployment,
    KernelNotFound,
    KernelLoadError,
    BootFailed,
}

impl From<ext4::Ext4Error> for BootError {
    fn from(e: ext4::Ext4Error) -> Self {
        BootError::Ext4Error(e)
    }
}
