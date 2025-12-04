//! Linux kernel loading and booting

use alloc::vec::Vec;
use uefi::boot::LoadImageSource;
use uefi::proto::loaded_image::LoadedImage;

use crate::ext4::Ext4Fs;
use crate::zub::{self, Deployment};
use crate::BootError;

/// kernel image data
pub struct KernelData {
    /// raw kernel EFI stub image
    pub kernel: Vec<u8>,
}

/// load kernel from a zub deployment
pub fn load_from_deployment(fs: &Ext4Fs, deployment: &Deployment) -> Result<KernelData, BootError> {
    // find kernel path
    let kernel_path = zub::find_kernel_path(fs, deployment).ok_or_else(|| {
        log::error!("kernel: no vmlinuz found in deployment");
        BootError::KernelNotFound
    })?;

    log::info!("kernel: loading {}", kernel_path);

    // read kernel image
    let kernel = fs.read_file(&kernel_path).map_err(|e| {
        log::error!("kernel: failed to read {}: {:?}", kernel_path, e);
        BootError::KernelLoadError
    })?;

    Ok(KernelData { kernel })
}

/// boot the loaded kernel
pub fn boot(data: KernelData, cmdline: &str) -> Result<(), BootError> {
    log::info!("kernel: booting with cmdline: {}", cmdline);

    let kernel_size = data.kernel.len();
    log::debug!("kernel: image size = {} bytes", kernel_size);

    // load kernel as EFI application using LoadImage with buffer source
    let source = LoadImageSource::FromBuffer {
        buffer: &data.kernel,
        file_path: None,
    };

    let image_handle = uefi::boot::load_image(uefi::boot::image_handle(), source).map_err(|e| {
        log::error!("kernel: LoadImage failed: {:?}", e);
        BootError::KernelLoadError
    })?;

    log::debug!("kernel: image loaded successfully");

    // convert cmdline to UCS-2 (null-terminated) - keep buffer alive until StartImage
    let cmdline_ucs2: Vec<u16> = cmdline.encode_utf16().chain(core::iter::once(0)).collect();

    // set kernel command line via LoadedImage protocol
    {
        let mut loaded_image = uefi::boot::open_protocol_exclusive::<LoadedImage>(image_handle)
            .map_err(|e| {
                log::error!("kernel: failed to open LoadedImage: {:?}", e);
                BootError::KernelLoadError
            })?;

        // set load options - the buffer must remain valid until StartImage
        unsafe {
            loaded_image.set_load_options(
                cmdline_ucs2.as_ptr() as *const u8,
                (cmdline_ucs2.len() * 2) as u32,
            );
        }
    }

    log::debug!(
        "kernel: command line set ({} bytes)",
        cmdline_ucs2.len() * 2
    );

    // start the kernel - this should not return
    log::info!("kernel: calling StartImage...");

    match uefi::boot::start_image(image_handle) {
        Ok(_) => {
            // should never reach here - kernel takes over
            log::error!("kernel: StartImage returned unexpectedly");
            Err(BootError::BootFailed)
        }
        Err(e) => {
            log::error!("kernel: StartImage failed: {:?}", e);
            Err(BootError::BootFailed)
        }
    }
}
