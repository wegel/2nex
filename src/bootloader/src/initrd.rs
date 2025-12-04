//! Initrd/initramfs loading for Linux kernel
//!
//! registers a LoadFile2 protocol handler so the Linux kernel can
//! fetch our generated initramfs during boot.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, Ordering};
use uefi::prelude::*;
use uefi::proto::device_path::DevicePath;
use uefi::{Error, Identify};

use crate::linux_efi::{
    self, InitrdDevicePath, LoadFile2Protocol, EFI_LOAD_FILE2_PROTOCOL_GUID,
};

/// global storage for initramfs data - accessed by LoadFile2 handler
static INITRD_DATA: AtomicPtr<Vec<u8>> = AtomicPtr::new(core::ptr::null_mut());

/// LoadFile2 protocol handler for initramfs
unsafe extern "efiapi" fn load_file2_handler(
    _this: *const LoadFile2Protocol,
    file_path: *const c_void,
    _boot_policy: bool,
    buffer_size: *mut usize,
    buffer: *mut c_void,
) -> Status {
    // verify this is a request for initrd
    if !linux_efi::is_initrd_device_path(file_path) {
        return Status::NOT_FOUND;
    }

    // get initrd data
    let initrd_ptr = INITRD_DATA.load(Ordering::SeqCst);
    if initrd_ptr.is_null() {
        return Status::NOT_FOUND;
    }

    let initrd = &*initrd_ptr;

    // if buffer is null, kernel wants to know the size
    if buffer.is_null() {
        *buffer_size = initrd.len();
        return Status::BUFFER_TOO_SMALL;
    }

    // check buffer size
    if *buffer_size < initrd.len() {
        *buffer_size = initrd.len();
        return Status::BUFFER_TOO_SMALL;
    }

    // copy data
    core::ptr::copy_nonoverlapping(initrd.as_ptr(), buffer as *mut u8, initrd.len());
    *buffer_size = initrd.len();

    Status::SUCCESS
}

/// install initrd protocol so Linux kernel can load the initramfs
pub fn install_initrd_protocol(initramfs_data: Vec<u8>) -> Result<Handle, Error> {
    // store initrd data globally (kernel will read it during boot)
    let data_box = Box::new(initramfs_data);
    let data_ptr = Box::into_raw(data_box);
    INITRD_DATA.store(data_ptr, Ordering::SeqCst);

    // create device path
    let device_path = Box::new(InitrdDevicePath::new());
    let device_path_ptr = Box::into_raw(device_path) as *mut c_void;

    // create protocol instance
    let protocol = Box::new(LoadFile2Protocol {
        load_file: load_file2_handler,
    });
    let protocol_ptr = Box::into_raw(protocol) as *mut c_void;

    // install device path protocol on a new handle
    let handle = unsafe {
        uefi::boot::install_protocol_interface(None, &DevicePath::GUID, device_path_ptr)?
    };

    // install LoadFile2 protocol on the same handle
    unsafe {
        uefi::boot::install_protocol_interface(
            Some(handle),
            &EFI_LOAD_FILE2_PROTOCOL_GUID,
            protocol_ptr,
        )?;
    }

    log::info!("initrd: installed LoadFile2 protocol ({} bytes)",
        unsafe { (*INITRD_DATA.load(Ordering::SeqCst)).len() });

    Ok(handle)
}
