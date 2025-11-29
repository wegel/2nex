//! 2nex OSTree-aware UEFI bootloader
//!
//! boots the default OSTree deployment from an ext4 partition.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use uefi::prelude::*;

mod disk;
mod ext4;
mod kernel;
mod ostree;

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

    // find ext4 root partition
    let root_disk = disk::find_root_partition()?;
    log::info!("found root partition");

    // mount ext4 filesystem
    let fs = ext4::mount(&root_disk)?;
    log::info!("mounted ext4 filesystem");

    // find default OSTree deployment
    let deployment = ostree::find_default_deployment(&fs)?;
    log::info!(
        "found deployment: {}.{}",
        deployment.checksum,
        deployment.serial
    );

    // load kernel from deployment
    let kernel_data = kernel::load_from_deployment(&fs, &deployment)?;
    log::info!("loaded kernel ({} bytes)", kernel_data.kernel.len());

    // build kernel command line
    let cmdline = build_cmdline(&root_disk, &deployment);
    log::info!("cmdline: {}", cmdline);

    // boot the kernel
    kernel::boot(kernel_data, &cmdline)
}

fn build_cmdline(disk: &disk::RootPartition, deployment: &ostree::Deployment) -> String {
    alloc::format!(
        "root=PARTUUID={} ostree={} ro console=ttyS0,115200n8 earlycon=uart8250,io,0x3f8,115200n8",
        disk.partuuid,
        deployment.path
    )
}

#[derive(Debug)]
pub enum BootError {
    DiskNotFound,
    PartitionNotFound,
    Ext4Error(ext4::Ext4Error),
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
