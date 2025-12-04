//! ext4 filesystem access via ext4-view-rs
//!
//! bridges UEFI BlockIO to ext4-view's Ext4Read trait.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use crate::disk::RootPartition;

/// wrapper around ext4-view's Ext4 type
pub struct Ext4Fs {
    inner: ext4_view::Ext4,
}

/// error type for ext4 operations
#[derive(Debug)]
pub enum Ext4Error {
    Io(String),
    Parse(ext4_view::Ext4Error),
    NotFound,
    NotADirectory,
}

impl fmt::Display for Ext4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ext4Error::Io(msg) => write!(f, "I/O error: {}", msg),
            Ext4Error::Parse(e) => write!(f, "ext4 parse error: {:?}", e),
            Ext4Error::NotFound => write!(f, "not found"),
            Ext4Error::NotADirectory => write!(f, "not a directory"),
        }
    }
}

/// directory entry from ext4 filesystem
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

impl Ext4Fs {
    /// read a file from the filesystem
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, Ext4Error> {
        log::debug!("ext4: read_file({})", path);

        self.inner.read(path).map_err(|e| Ext4Error::Parse(e))
    }

    /// list directory contents
    pub fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>, Ext4Error> {
        log::debug!("ext4: read_dir({})", path);

        let dir = self.inner.read_dir(path).map_err(|e| Ext4Error::Parse(e))?;

        let mut entries = Vec::new();
        for (i, entry) in dir.enumerate() {
            match entry {
                Ok(e) => {
                    let file_type = match e.file_type() {
                        Ok(ft) => ft,
                        Err(err) => {
                            log::debug!("ext4: entry {} file_type error: {:?}", i, err);
                            continue;
                        }
                    };
                    let name_str = match e.file_name().as_str() {
                        Ok(s) => s,
                        Err(err) => {
                            log::debug!("ext4: entry {} name error: {:?}", i, err);
                            continue;
                        }
                    };
                    log::debug!("ext4: entry: {} (dir={})", name_str, file_type.is_dir());
                    entries.push(DirEntry {
                        name: String::from(name_str),
                        is_dir: file_type.is_dir(),
                    });
                }
                Err(err) => {
                    log::debug!("ext4: entry {} read error: {:?}", i, err);
                }
            }
        }

        log::debug!("ext4: read_dir found {} entries", entries.len());
        Ok(entries)
    }

    /// check if a path exists
    #[allow(dead_code)]
    pub fn exists(&self, path: &str) -> bool {
        self.inner.exists(path).unwrap_or(false)
    }
}

/// mount an ext4 filesystem from a partition
pub fn mount(partition: &RootPartition) -> Result<Ext4Fs, Ext4Error> {
    log::info!("ext4: mounting partition {}", partition.partuuid);

    let reader = UefiBlockReader::new(partition);

    let inner = ext4_view::Ext4::load(Box::new(reader)).map_err(|e| Ext4Error::Parse(e))?;

    log::info!("ext4: filesystem mounted successfully");

    Ok(Ext4Fs { inner })
}

/// mount an ext4 filesystem from a custom reader (e.g., decrypting reader)
pub fn mount_from_reader(reader: Box<dyn ext4_view::Ext4Read>) -> Result<Ext4Fs, Ext4Error> {
    log::info!("ext4: mounting from custom reader");

    let inner = ext4_view::Ext4::load(reader).map_err(|e| Ext4Error::Parse(e))?;

    log::info!("ext4: filesystem mounted successfully");

    Ok(Ext4Fs { inner })
}

/// UEFI block device reader implementing ext4-view's Ext4Read trait
struct UefiBlockReader {
    handle: uefi::Handle,
    block_size: u32,
    #[allow(dead_code)]
    num_blocks: u64,
}

impl UefiBlockReader {
    fn new(partition: &RootPartition) -> Self {
        Self {
            handle: partition.handle,
            block_size: partition.block_size,
            num_blocks: partition.num_blocks,
        }
    }
}

/// I/O error type for ext4-view
#[derive(Debug)]
struct UefiIoError(String);

impl fmt::Display for UefiIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UEFI I/O error: {}", self.0)
    }
}

impl ext4_view::IoError for UefiIoError {}

impl ext4_view::Ext4Read for UefiBlockReader {
    fn read(&mut self, start_byte: u64, dst: &mut [u8]) -> Result<(), Box<dyn ext4_view::IoError>> {
        use uefi::proto::media::block::BlockIO;

        if dst.is_empty() {
            return Ok(());
        }

        // open BlockIO protocol for this handle
        let block_io = uefi::boot::open_protocol_exclusive::<BlockIO>(self.handle).map_err(
            |e| -> Box<dyn ext4_view::IoError> {
                Box::new(UefiIoError(alloc::format!("open BlockIO failed: {:?}", e)))
            },
        )?;

        let media_id = block_io.media().media_id();
        let block_size = self.block_size as u64;

        // calculate which blocks we need to read
        let start_block = start_byte / block_size;
        let end_byte = start_byte + dst.len() as u64;
        let end_block = (end_byte + block_size - 1) / block_size;
        let num_blocks = end_block - start_block;

        // allocate buffer for full blocks
        let buf_size = (num_blocks * block_size) as usize;
        let mut buf = vec![0u8; buf_size];

        // read blocks
        block_io
            .read_blocks(media_id, start_block, &mut buf)
            .map_err(|e| -> Box<dyn ext4_view::IoError> {
                Box::new(UefiIoError(alloc::format!("read_blocks failed: {:?}", e)))
            })?;

        // copy the requested portion to dst
        let offset = (start_byte % block_size) as usize;
        dst.copy_from_slice(&buf[offset..offset + dst.len()]);

        Ok(())
    }
}
