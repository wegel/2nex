//! UEFI disk access and partition discovery

use alloc::string::String;
use uefi::proto::media::block::BlockIO;
use uefi::proto::media::partition::{GptPartitionType, PartitionInfo};
use uefi::{guid, Handle, Identify};

use crate::BootError;

// GPT partition type GUIDs
const LINUX_FILESYSTEM_TYPE: GptPartitionType =
    GptPartitionType(guid!("0fc63daf-8483-4772-8e79-3d69d8477de4"));

// expected partition name for root filesystem
const ROOT_PARTITION_NAME: &str = "nex";

/// represents a discovered root partition
pub struct RootPartition {
    pub handle: Handle,
    pub partuuid: String,
    pub block_size: u32,
    pub num_blocks: u64,
}

impl RootPartition {
    /// read blocks from the partition
    #[allow(dead_code)]
    pub fn read_blocks(&self, lba: u64, buffer: &mut [u8]) -> Result<(), BootError> {
        let block_io = uefi::boot::open_protocol_exclusive::<BlockIO>(self.handle)
            .map_err(|_| BootError::DiskNotFound)?;

        block_io
            .read_blocks(block_io.media().media_id(), lba, buffer)
            .map_err(|_| BootError::DiskNotFound)?;

        Ok(())
    }
}

/// find the root ext4 partition named "nex"
pub fn find_root_partition() -> Result<RootPartition, BootError> {
    // get all handles with BlockIO protocol
    let handles =
        uefi::boot::locate_handle_buffer(uefi::boot::SearchType::ByProtocol(&BlockIO::GUID))
            .map_err(|_| BootError::DiskNotFound)?;

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

                    return Ok(RootPartition {
                        handle,
                        partuuid,
                        block_size,
                        num_blocks,
                    });
                }
            }
        }
    }

    Err(BootError::PartitionNotFound)
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
