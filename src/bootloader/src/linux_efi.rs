//! Linux EFI stub initrd loading protocol
//!
//! implements the LoadFile2 protocol with LINUX_EFI_INITRD_MEDIA_GUID
//! so the Linux kernel can load our generated initramfs.

use core::ffi::c_void;
use uefi::prelude::*;
use uefi::{guid, Guid};

/// GUID from Linux kernel: drivers/firmware/efi/libstub/efi-stub-helper.c
pub const LINUX_EFI_INITRD_MEDIA_GUID: Guid = guid!("5568e427-68fc-4f3d-ac74-ca555231cc68");

/// EFI_LOAD_FILE2_PROTOCOL GUID
pub const EFI_LOAD_FILE2_PROTOCOL_GUID: Guid = guid!("4006c0c1-fcb3-403e-996d-4a6c8724e06d");

/// LoadFile2 protocol structure
#[repr(C)]
pub struct LoadFile2Protocol {
    pub load_file: unsafe extern "efiapi" fn(
        this: *const LoadFile2Protocol,
        file_path: *const c_void,
        boot_policy: bool,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
}

/// device path header
#[repr(C, packed)]
pub struct DevicePathProtocol {
    pub r#type: u8,
    pub sub_type: u8,
    pub length: [u8; 2],
}

/// vendor device path
#[repr(C, packed)]
pub struct VendorDevicePath {
    pub header: DevicePathProtocol,
    pub guid: Guid,
}

/// initrd device path (vendor + end)
#[repr(C, packed)]
pub struct InitrdDevicePath {
    pub vendor: VendorDevicePath,
    pub end: DevicePathProtocol,
}

// device path constants
pub const MEDIA_DEVICE_PATH: u8 = 0x04;
pub const MEDIA_VENDOR_DP: u8 = 0x03;
pub const END_DEVICE_PATH_TYPE: u8 = 0x7F;
pub const END_ENTIRE_DEVICE_PATH_SUBTYPE: u8 = 0xFF;

impl InitrdDevicePath {
    pub fn new() -> Self {
        Self {
            vendor: VendorDevicePath {
                header: DevicePathProtocol {
                    r#type: MEDIA_DEVICE_PATH,
                    sub_type: MEDIA_VENDOR_DP,
                    length: [20, 0], // sizeof(VendorDevicePath)
                },
                guid: LINUX_EFI_INITRD_MEDIA_GUID,
            },
            end: DevicePathProtocol {
                r#type: END_DEVICE_PATH_TYPE,
                sub_type: END_ENTIRE_DEVICE_PATH_SUBTYPE,
                length: [4, 0],
            },
        }
    }
}

/// check if a device path is the initrd device path
pub unsafe fn is_initrd_device_path(path: *const c_void) -> bool {
    if path.is_null() {
        return false;
    }

    let dp = path as *const DevicePathProtocol;

    // for initrd, the remaining path should be just the End node
    if (*dp).r#type == END_DEVICE_PATH_TYPE && (*dp).sub_type == END_ENTIRE_DEVICE_PATH_SUBTYPE {
        return true;
    }

    // also check if it's a vendor device path with our GUID
    if (*dp).r#type == MEDIA_DEVICE_PATH && (*dp).sub_type == MEDIA_VENDOR_DP {
        let vendor_dp = path as *const VendorDevicePath;
        let guid_ptr = (vendor_dp as *const u8).add(4) as *const Guid;
        let guid = core::ptr::read_unaligned(guid_ptr);

        if guid == LINUX_EFI_INITRD_MEDIA_GUID {
            return true;
        }
    }

    false
}
