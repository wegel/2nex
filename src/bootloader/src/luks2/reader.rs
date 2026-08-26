//! decrypting block reader for LUKS2 volumes
//!
//! wraps UEFI BlockIO and decrypts sectors on the fly using AES-XTS.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
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

type ReadError = Box<dyn core::error::Error + Send + Sync + 'static>;

struct ReadLayout {
    physical_start: u64,
    aligned_start: u64,
    disk_start_block: u64,
    disk_read_size: usize,
    sector_offset_in_disk: usize,
    sector_read_size: usize,
}

impl ReadLayout {
    fn new(reader: &DecryptingReader, start_byte: u64, output_size: usize) -> Self {
        let disk_block_size = reader.disk_block_size as u64;
        let sector_size = reader.sector_size as u64;
        let physical_start = reader.data_offset + start_byte;
        let physical_end = physical_start + output_size as u64;
        let start_sector = physical_start / sector_size;
        let end_sector = physical_end.div_ceil(sector_size);
        let aligned_start = start_sector * sector_size;
        let sector_read_size = ((end_sector - start_sector) * sector_size) as usize;
        let partition_start_block = aligned_start / disk_block_size;
        let disk_start_block = reader.start_lba + partition_start_block;
        let disk_end_block =
            reader.start_lba + (aligned_start + sector_read_size as u64).div_ceil(disk_block_size);
        Self {
            physical_start,
            aligned_start,
            disk_start_block,
            disk_read_size: ((disk_end_block - disk_start_block) * disk_block_size) as usize,
            sector_offset_in_disk: (aligned_start - partition_start_block * disk_block_size)
                as usize,
            sector_read_size,
        }
    }
}

impl DecryptingReader {
    fn read_encrypted_sectors(&self, layout: &ReadLayout) -> Result<Vec<u8>, ReadError> {
        let block_io = uefi::boot::open_protocol_exclusive::<BlockIO>(self.handle).map_err(
            |error| -> ReadError {
                Box::new(DecryptIoError(alloc::format!("open BlockIO: {:?}", error)))
            },
        )?;
        let mut disk_buf = vec![0u8; layout.disk_read_size];
        block_io
            .read_blocks(
                block_io.media().media_id(),
                layout.disk_start_block,
                &mut disk_buf,
            )
            .map_err(|error| -> ReadError {
                Box::new(DecryptIoError(alloc::format!("read_blocks: {:?}", error)))
            })?;
        let start = layout.sector_offset_in_disk;
        let end = start + layout.sector_read_size;
        Ok(disk_buf[start..end].to_vec())
    }

    fn decrypt_into(
        &mut self,
        start_byte: u64,
        output: &mut [u8],
        layout: &ReadLayout,
        mut sectors: Vec<u8>,
    ) {
        let logical_sector = start_byte / self.sector_size as u64;
        self.cipher
            .decrypt_sectors(self.iv_tweak + logical_sector, &mut sectors);
        let start = (layout.physical_start - layout.aligned_start) as usize;
        output.copy_from_slice(&sectors[start..start + output.len()]);
    }
}

impl ext4_view::Ext4Read for DecryptingReader {
    fn read(&mut self, start_byte: u64, dst: &mut [u8]) -> Result<(), ReadError> {
        if dst.is_empty() {
            return Ok(());
        }
        let layout = ReadLayout::new(self, start_byte, dst.len());
        let sectors = self.read_encrypted_sectors(&layout)?;
        self.decrypt_into(start_byte, dst, &layout, sectors);
        Ok(())
    }
}
