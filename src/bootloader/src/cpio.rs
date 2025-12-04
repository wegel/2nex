//! CPIO "newc" format archive generator
//!
//! generates cpio archives in memory for passing as initramfs to the kernel.
//! uses the SVR4 "newc" format which the Linux kernel expects.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// CPIO archive builder
pub struct CpioBuilder {
    data: Vec<u8>,
}

impl CpioBuilder {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// add a directory entry
    #[allow(dead_code)]
    pub fn add_dir(&mut self, path: &str, mode: u32) {
        self.add_entry(path, &[], mode | 0o040000, 0);
    }

    /// add a regular file, auto-creating parent directories
    pub fn add_file(&mut self, path: &str, content: &[u8], mode: u32) {
        self.ensure_parent_dirs(path);
        self.add_entry(path, content, mode | 0o100000, content.len());
    }

    /// ensure all parent directories exist in the archive
    fn ensure_parent_dirs(&mut self, path: &str) {
        let mut current_path = String::new();
        let parts: Vec<&str> = path.split('/').collect();

        for part in &parts[..parts.len().saturating_sub(1)] {
            if !part.is_empty() {
                if !current_path.is_empty() {
                    current_path.push('/');
                }
                current_path.push_str(part);
                // cpio handles duplicate entries gracefully
                self.add_entry(&current_path, &[], 0o040755, 0);
            }
        }
    }

    fn add_entry(&mut self, path: &str, content: &[u8], mode: u32, size: usize) {
        // newc format header (110 bytes of ASCII hex)
        let header = format!(
            "{magic}{ino:08x}{mode:08x}{uid:08x}{gid:08x}{nlink:08x}{mtime:08x}\
             {filesize:08x}{devmajor:08x}{devminor:08x}{rdevmajor:08x}{rdevminor:08x}\
             {namesize:08x}{check:08x}",
            magic = "070701",
            ino = 0,
            mode = mode,
            uid = 0,
            gid = 0,
            nlink = 1,
            mtime = 0,
            filesize = size,
            devmajor = 0,
            devminor = 0,
            rdevmajor = 0,
            rdevminor = 0,
            namesize = path.len() + 1,
            check = 0,
        );

        // write header
        self.data.extend_from_slice(header.as_bytes());

        // write pathname + null terminator
        self.data.extend_from_slice(path.as_bytes());
        self.data.push(0);

        // pad to 4-byte boundary
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }

        // write file content
        self.data.extend_from_slice(content);

        // pad content to 4-byte boundary
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
    }

    /// finalize the archive with TRAILER!!! marker
    pub fn finalize(mut self) -> Vec<u8> {
        self.add_entry("TRAILER!!!", &[], 0, 0);
        self.data
    }
}

/// build an initramfs cpio containing kernel modules
pub fn build_module_initramfs(modules: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut builder = CpioBuilder::new();

    for (name, content) in modules {
        let path = format!("lib/modules/{}", name);
        builder.add_file(&path, content, 0o644);
    }

    builder.finalize()
}
