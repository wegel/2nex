//! nex zub-aware UEFI bootloader
//!
//! boots the default zub deployment from an ext4 partition.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use uefi::boot;
use uefi::fs::{FileSystem, Path, PathBuf};
use uefi::prelude::*;
use uefi::proto::device_path::media::FilePath as DevicePathFilePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::CString16;

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

    log::info!("nex bootloader v{}", env!("CARGO_PKG_VERSION"));

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

    // check for extra initrd data and install initrd protocol if needed
    if let Some(initramfs) = load_extra_initrd(&fs, &deployment) {
        log::info!(
            "installing extra initrd protocol ({} bytes)",
            initramfs.len()
        );
        if let Err(e) = initrd::install_initrd_protocol(initramfs) {
            log::warn!("failed to install initrd protocol: {:?}", e);
        }
    }

    // build kernel command line (allow optional ESP override)
    let extra_cmdline = read_kcmdline_from_esp();
    let cmdline = build_cmdline(&root_disk, &deployment, extra_cmdline.as_deref());
    log::info!("cmdline: {}", cmdline);

    // boot the kernel
    kernel::boot(kernel_data, &cmdline)
}

/// load deployment-selected extra initrd data in Linux-required order
fn load_extra_initrd(fs: &ext4::Ext4Fs, deployment: &zub::Deployment) -> Option<Vec<u8>> {
    let mut initrd = Vec::new();

    for (vendor, path) in [
        ("AMD", "/boot/amd-ucode.cpio"),
        ("Intel", "/boot/intel-ucode.cpio"),
    ] {
        if let Some(microcode) = load_microcode_initrd(fs, deployment, vendor, path) {
            initrd.extend_from_slice(&microcode);
        }
    }

    if let Some(base_initramfs) = load_deployment_initramfs(fs, deployment) {
        initrd.extend_from_slice(&base_initramfs);
    }

    if let Some(modules) = load_boot_modules(fs, deployment) {
        initrd.extend_from_slice(&modules);
    }

    if initrd.is_empty() {
        None
    } else {
        Some(initrd)
    }
}

/// load an early microcode cpio if the deployment declares it
fn load_microcode_initrd(
    fs: &ext4::Ext4Fs,
    deployment: &zub::Deployment,
    vendor: &str,
    path: &str,
) -> Option<Vec<u8>> {
    let microcode_path = alloc::format!("{}{}", deployment.path, path);

    match fs.read_file(&microcode_path) {
        Ok(data) => {
            log::info!(
                "microcode: loaded {} early cpio ({} bytes)",
                vendor,
                data.len()
            );
            Some(data)
        }
        Err(_) => None,
    }
}

fn load_deployment_initramfs(fs: &ext4::Ext4Fs, deployment: &zub::Deployment) -> Option<Vec<u8>> {
    let path = alloc::format!("{}/boot/initramfs.cpio", deployment.path);

    match fs.read_file(&path) {
        Ok(data) => {
            log::info!("initramfs: loaded deployment cpio ({} bytes)", data.len());
            Some(data)
        }
        Err(e) => {
            log::warn!("initramfs: failed to load {}: {:?}", path, e);
            None
        }
    }
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
    log::info!(
        "boot-modules: found config with {} bytes",
        config_content.len()
    );

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

fn build_cmdline(
    disk: &disk::RootPartition,
    deployment: &zub::Deployment,
    extra: Option<&str>,
) -> String {
    let mut cmdline = alloc::format!("root=PARTUUID={} zub={} ro", disk.partuuid, deployment.path);

    if let Some(extra) = extra {
        let extra = extra.trim();
        if !extra.is_empty() {
            cmdline.push(' ');
            cmdline.push_str(extra);
        }
    }

    cmdline
}

fn read_kcmdline_from_esp() -> Option<String> {
    let fs = match boot::get_image_file_system(boot::image_handle()) {
        Ok(fs) => fs,
        Err(e) => {
            log::warn!("kcmdline: failed to get image filesystem: {:?}", e);
            return None;
        }
    };
    let mut fs = FileSystem::new(fs);

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(mut dir) = bootloader_dir() {
        if let Ok(name) = CString16::try_from("kcmdline.txt") {
            dir.push(name.as_ref());
            candidates.push(dir);
        }
    }

    if let Ok(fallback) = CString16::try_from("\\EFI\\BOOT\\kcmdline.txt") {
        candidates.push(PathBuf::from(fallback.as_ref()));
    }

    for path in candidates {
        match fs.read_to_string(&*path) {
            Ok(contents) => {
                if let Some(extra) = normalize_kcmdline(&contents) {
                    log::info!("kcmdline: loaded from {}", path);
                    return Some(extra);
                }
                log::warn!("kcmdline: {} is empty after trimming", path);
            }
            Err(e) => {
                log::debug!("kcmdline: failed to read {}: {:?}", path, e);
            }
        }
    }

    None
}

fn bootloader_dir() -> Option<PathBuf> {
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).ok()?;
    let file_path = loaded_image.file_path()?;

    let mut file_path_cstr: Option<CString16> = None;
    for node in file_path.node_iter() {
        if let Ok(file_path_node) = <&DevicePathFilePath>::try_from(node) {
            if let Ok(cstr) = CString16::try_from(&file_path_node.path_name()) {
                file_path_cstr = Some(cstr);
            }
        }
    }

    let file_path_cstr = file_path_cstr?;
    let path = Path::new(file_path_cstr.as_ref());
    if let Some(parent) = path.parent() {
        return Some(parent);
    }

    if let Ok(root) = CString16::try_from("\\") {
        return Some(PathBuf::from(root.as_ref()));
    }

    None
}

fn normalize_kcmdline(contents: &str) -> Option<String> {
    let mut out = String::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(line);
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
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
