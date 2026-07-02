//! UEFI disk access and partition discovery

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::mem;
use uefi::boot;
use uefi::proto::device_path::{DevicePath, DevicePathNodeEnum};
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::block::BlockIO;
use uefi::proto::media::partition::{GptPartitionType, PartitionInfo};
use uefi::{guid, Handle, Identify};

use crate::BootError;

// GPT partition type GUIDs
const LINUX_FILESYSTEM_TYPE: GptPartitionType =
    GptPartitionType(guid!("0fc63daf-8483-4772-8e79-3d69d8477de4"));

// expected partition name for root filesystem
const ROOT_PARTITION_NAME: &str = "nex";
const GPT_HEADER_LBA: u64 = 1;
const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
const GPT_ENTRY_SIZE_MIN: u32 = 128;

/// represents a discovered root partition
pub struct RootPartition {
    pub handle: Handle,
    pub partuuid: String,
    pub block_size: u32,
    pub num_blocks: u64,
    pub start_lba: u64,
}

impl RootPartition {
    /// read blocks from the partition
    #[allow(dead_code)]
    pub fn read_blocks(&self, lba: u64, buffer: &mut [u8]) -> Result<(), BootError> {
        let block_io = uefi::boot::open_protocol_exclusive::<BlockIO>(self.handle)
            .map_err(|_| BootError::DiskNotFound)?;

        block_io
            .read_blocks(block_io.media().media_id(), self.start_lba + lba, buffer)
            .map_err(|_| BootError::DiskNotFound)?;

        Ok(())
    }
}

/// find the root ext4 partition named "nex"
pub fn find_root_partition() -> Result<RootPartition, BootError> {
    let boot_prefix = boot_device_prefix();

    // get all handles with BlockIO protocol
    let handles =
        uefi::boot::locate_handle_buffer(uefi::boot::SearchType::ByProtocol(&BlockIO::GUID))
            .map_err(|_| BootError::DiskNotFound)?;

    let mut fallback: Option<RootPartition> = None;

    for &handle in handles.iter() {
        // try to get partition info
        if let Ok(partition_info) = uefi::boot::open_protocol_exclusive::<PartitionInfo>(handle) {
            // check if this is a GPT partition
            if let Some(gpt) = partition_info.gpt_partition_entry() {
                // copy fields to avoid unaligned access on packed struct
                let type_guid = gpt.partition_type_guid;
                let unique_guid = gpt.unique_partition_guid;
                let partition_name = gpt.partition_name;

                // skip non-Linux filesystem partitions (e.g., ESP)
                if type_guid != LINUX_FILESYSTEM_TYPE {
                    log::debug!("skipping partition with type {:?}", type_guid);
                    continue;
                }

                // check partition name matches "nex"
                let name = partition_name_to_string(&partition_name);
                if name != ROOT_PARTITION_NAME {
                    log::debug!("skipping partition with name {:?}", name);
                    continue;
                }

                let partuuid = format_guid(&unique_guid);

                // get block IO for this partition
                if let Ok(block_io) = uefi::boot::open_protocol_exclusive::<BlockIO>(handle) {
                    let media = block_io.media();

                    // skip if not present or no media
                    if !media.is_media_present() {
                        continue;
                    }

                    let block_size = media.block_size();
                    let num_blocks = media.last_block() + 1;

                    log::debug!(
                        "found Linux partition: {} ({} blocks of {} bytes)",
                        partuuid,
                        num_blocks,
                        block_size
                    );

                    let root = RootPartition {
                        handle,
                        partuuid,
                        block_size,
                        num_blocks,
                        start_lba: 0,
                    };

                    if let Some(prefix) = boot_prefix.as_ref() {
                        if let Some(part_prefix) = device_path_prefix(handle) {
                            if &part_prefix == prefix {
                                log::debug!("selected root partition on boot device");
                                return Ok(root);
                            }
                        }
                    }

                    if fallback.is_none() {
                        fallback = Some(root);
                    }
                }
            }
        }
    }

    if let Some(root) = fallback {
        log::debug!("selected root partition (fallback)");
        return Ok(root);
    }

    if let Some(root) = find_root_partition_by_gpt_scan()? {
        log::debug!("selected root partition from raw GPT scan");
        return Ok(root);
    }

    Err(BootError::PartitionNotFound)
}

fn find_root_partition_by_gpt_scan() -> Result<Option<RootPartition>, BootError> {
    let boot_prefix = boot_device_prefix();
    let handles =
        uefi::boot::locate_handle_buffer(uefi::boot::SearchType::ByProtocol(&BlockIO::GUID))
            .map_err(|_| BootError::DiskNotFound)?;

    let mut fallback: Option<RootPartition> = None;

    for &handle in handles.iter() {
        let block_io = match uefi::boot::open_protocol_exclusive::<BlockIO>(handle) {
            Ok(block_io) => block_io,
            Err(_) => continue,
        };

        let media = block_io.media();
        if !media.is_media_present() || media.is_logical_partition() {
            continue;
        }

        let block_size = media.block_size();
        let media_id = media.media_id();
        let mut header = vec![0u8; block_size as usize];
        if block_io
            .read_blocks(media_id, GPT_HEADER_LBA, &mut header)
            .is_err()
        {
            continue;
        }

        let Some(gpt) = GptHeader::parse(&header) else {
            continue;
        };

        let Some(root) = scan_gpt_entries(handle, &block_io, media_id, block_size, &gpt) else {
            continue;
        };

        if let Some(prefix) = boot_prefix.as_ref() {
            if let Some(part_prefix) = device_path_prefix(handle) {
                if &part_prefix == prefix {
                    return Ok(Some(root));
                }
            }
        }

        if fallback.is_none() {
            fallback = Some(root);
        }
    }

    Ok(fallback)
}

fn scan_gpt_entries(
    handle: Handle,
    block_io: &BlockIO,
    media_id: u32,
    block_size: u32,
    header: &GptHeader,
) -> Option<RootPartition> {
    if header.entry_size < GPT_ENTRY_SIZE_MIN {
        return None;
    }

    let entry_size = header.entry_size as usize;
    let entry_count = header.entry_count as usize;
    let entries_size = entry_size.checked_mul(entry_count)?;
    let block_size_usize = block_size as usize;
    let read_size = entries_size.div_ceil(block_size_usize) * block_size_usize;
    let mut entries = vec![0u8; read_size];

    if block_io
        .read_blocks(media_id, header.entries_lba, &mut entries)
        .is_err()
    {
        return None;
    }

    for chunk in entries[..entries_size].chunks_exact(entry_size) {
        let Some(entry) = GptEntry::parse(chunk) else {
            continue;
        };

        if entry.partition_type_guid != LINUX_FILESYSTEM_TYPE {
            continue;
        }

        if entry.name != ROOT_PARTITION_NAME {
            continue;
        }

        let num_blocks = entry.end_lba.checked_sub(entry.start_lba)?.checked_add(1)?;
        log::debug!(
            "raw GPT scan found root partition: {} (start {}, {} blocks of {} bytes)",
            entry.partuuid,
            entry.start_lba,
            num_blocks,
            block_size
        );

        return Some(RootPartition {
            handle,
            partuuid: entry.partuuid,
            block_size,
            num_blocks,
            start_lba: entry.start_lba,
        });
    }

    None
}

struct GptHeader {
    entries_lba: u64,
    entry_count: u32,
    entry_size: u32,
}

impl GptHeader {
    fn parse(block: &[u8]) -> Option<Self> {
        if block.get(..GPT_SIGNATURE.len())? != GPT_SIGNATURE {
            return None;
        }

        Some(Self {
            entries_lba: read_u64_le(block, 72)?,
            entry_count: read_u32_le(block, 80)?,
            entry_size: read_u32_le(block, 84)?,
        })
    }
}

struct GptEntry {
    partition_type_guid: GptPartitionType,
    partuuid: String,
    start_lba: u64,
    end_lba: u64,
    name: String,
}

impl GptEntry {
    fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < GPT_ENTRY_SIZE_MIN as usize {
            return None;
        }

        let partition_type_guid = guid_from_bytes(bytes.get(0..16)?)?;
        if guid_is_zero(&partition_type_guid) {
            return None;
        }

        let unique_guid = guid_from_bytes(bytes.get(16..32)?)?;
        let name = utf16_name_from_bytes(bytes.get(56..128)?);

        Some(Self {
            partition_type_guid: GptPartitionType(partition_type_guid),
            partuuid: format_guid(&unique_guid),
            start_lba: read_u64_le(bytes, 32)?,
            end_lba: read_u64_le(bytes, 40)?,
            name,
        })
    }
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let raw: [u8; 4] = bytes
        .get(offset..offset + mem::size_of::<u32>())?
        .try_into()
        .ok()?;
    Some(u32::from_le_bytes(raw))
}

fn read_u64_le(bytes: &[u8], offset: usize) -> Option<u64> {
    let raw: [u8; 8] = bytes
        .get(offset..offset + mem::size_of::<u64>())?
        .try_into()
        .ok()?;
    Some(u64::from_le_bytes(raw))
}

fn guid_from_bytes(bytes: &[u8]) -> Option<uefi::Guid> {
    let raw: [u8; 16] = bytes.try_into().ok()?;
    Some(uefi::Guid::from_bytes(raw))
}

fn guid_is_zero(guid: &uefi::Guid) -> bool {
    guid.to_bytes().iter().all(|byte| *byte == 0)
}

fn utf16_name_from_bytes(bytes: &[u8]) -> String {
    let mut code_units = Vec::new();

    for pair in bytes.chunks_exact(2) {
        let unit = u16::from_le_bytes([pair[0], pair[1]]);
        if unit == 0 {
            break;
        }
        code_units.push(unit);
    }

    char::decode_utf16(code_units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

fn boot_device_prefix() -> Option<Vec<u8>> {
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).ok()?;
    let device = loaded_image.device()?;
    device_path_prefix(device)
}

fn device_path_prefix(handle: Handle) -> Option<Vec<u8>> {
    let dp = boot::open_protocol_exclusive::<DevicePath>(handle).ok()?;
    let mut prefix_len: usize = 0;
    for node in dp.node_iter() {
        if let Ok(node_enum) = node.as_enum() {
            if matches!(node_enum, DevicePathNodeEnum::MediaHardDrive(_)) {
                break;
            }
        }
        prefix_len += usize::from(node.length());
    }

    dp.as_bytes().get(..prefix_len).map(|b| b.to_vec())
}

fn format_guid(guid: &uefi::Guid) -> String {
    // format GUID as lowercase string without braces
    let bytes = guid.to_bytes();
    alloc::format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[3], bytes[2], bytes[1], bytes[0],
        bytes[5], bytes[4],
        bytes[7], bytes[6],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// convert GPT partition name (Char16, null-terminated) to String
fn partition_name_to_string(name: &[uefi::Char16; 36]) -> String {
    // find null terminator and decode UTF-16
    let len = name.iter().position(|&c| u16::from(c) == 0).unwrap_or(36);
    char::decode_utf16(name[..len].iter().map(|&c| u16::from(c)))
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}
