//! decrypting block reader for LUKS2 volumes
//!
//! wraps UEFI BlockIO and decrypts sectors on the fly using AES-XTS.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use core::fmt;
use uefi::proto::media::block::BlockIO;

use super::crypto::AesXts;

/// decrypting block reader implementing ext4_view::Ext4Read
pub struct DecryptingReader {
    handle: uefi::Handle,
    disk_block_size: u32,
    start_lba: u64,
    cipher: AesXts,
    /// offset where encrypted data begins (after LUKS header)
    data_offset: u64,
    /// LUKS sector size (from segment, usually 512 or 4096)
    sector_size: u32,
    /// starting IV tweak value
    iv_tweak: u64,
}

impl DecryptingReader {
    pub fn new(
        handle: uefi::Handle,
        disk_block_size: u32,
        start_lba: u64,
        master_key: &[u8],
        data_offset: u64,
        sector_size: u32,
        iv_tweak: u64,
    ) -> Result<Self, super::Luks2Error> {
        let cipher = AesXts::new(master_key, sector_size as usize)
            .map_err(|_| super::Luks2Error::InvalidKeySize)?;

        Ok(Self {
            handle,
            disk_block_size,
            start_lba,
            cipher,
            data_offset,
            sector_size,
            iv_tweak,
        })
    }
}

#[derive(Debug)]
struct DecryptIoError(String);

impl fmt::Display for DecryptIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "decrypt I/O: {}", self.0)
    }
}

impl core::error::Error for DecryptIoError {}

impl ext4_view::Ext4Read for DecryptingReader {
    fn read(
        &mut self,
        start_byte: u64,
        dst: &mut [u8],
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        if dst.is_empty() {
            return Ok(());
        }

        // open BlockIO protocol
        let block_io = uefi::boot::open_protocol_exclusive::<BlockIO>(self.handle).map_err(
            |e| -> Box<dyn core::error::Error + Send + Sync + 'static> {
                Box::new(DecryptIoError(alloc::format!("open BlockIO: {:?}", e)))
            },
        )?;

        let media_id = block_io.media().media_id();
        let disk_block_size = self.disk_block_size as u64;
        let sector_size = self.sector_size as u64;

        // translate logical offset to physical (add data area offset)
        let physical_start = self.data_offset + start_byte;
        let physical_end = physical_start + dst.len() as u64;

        // align to sector boundaries for decryption
        let start_sector = physical_start / sector_size;
        let end_sector = (physical_end + sector_size - 1) / sector_size;
        let num_sectors = end_sector - start_sector;

        // calculate physical byte range to read (sector-aligned)
        let aligned_start = start_sector * sector_size;
        let read_size = (num_sectors * sector_size) as usize;

        // align read to disk block boundaries
        let partition_start_block = aligned_start / disk_block_size;
        let disk_start_block = self.start_lba + partition_start_block;
        let disk_end_block = self.start_lba
            + (aligned_start + read_size as u64 + disk_block_size - 1) / disk_block_size;
        let disk_read_blocks = disk_end_block - disk_start_block;
        let disk_read_size = (disk_read_blocks * disk_block_size) as usize;

        // allocate buffer for disk-aligned read
        let mut disk_buf = vec![0u8; disk_read_size];

        // read from disk
        block_io
            .read_blocks(media_id, disk_start_block, &mut disk_buf)
            .map_err(|e| -> Box<dyn core::error::Error + Send + Sync + 'static> {
                Box::new(DecryptIoError(alloc::format!("read_blocks: {:?}", e)))
            })?;

        // extract sector-aligned portion from disk buffer
        let sector_offset_in_disk =
            (aligned_start - partition_start_block * disk_block_size) as usize;
        let mut sector_buf = vec![0u8; read_size];
        sector_buf
            .copy_from_slice(&disk_buf[sector_offset_in_disk..sector_offset_in_disk + read_size]);

        // decrypt sectors in place
        // the tweak is the logical sector number relative to the start of the encrypted area
        let logical_sector = start_byte / sector_size;
        self.cipher
            .decrypt_sectors(self.iv_tweak + logical_sector, &mut sector_buf);

        // copy the requested portion to destination
        let offset_in_sector = (physical_start - aligned_start) as usize;
        dst.copy_from_slice(&sector_buf[offset_in_sector..offset_in_sector + dst.len()]);

        Ok(())
    }
}
