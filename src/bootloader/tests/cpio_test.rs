//! Test the CPIO builder by generating an archive and verifying with cpio tool
//!
//! Run with: cargo test --package nex-bootloader-test

use std::io::Write;
use std::process::Command;

/// CPIO archive builder (copied from bootloader for testing)
struct CpioBuilder {
    data: Vec<u8>,
}

impl CpioBuilder {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn add_dir(&mut self, path: &str, mode: u32) {
        self.add_entry(path, &[], mode | 0o040000, 0);
    }

    fn add_file(&mut self, path: &str, content: &[u8], mode: u32) {
        self.ensure_parent_dirs(path);
        self.add_entry(path, content, mode | 0o100000, content.len());
    }

    fn ensure_parent_dirs(&mut self, path: &str) {
        let mut current_path = String::new();
        let parts: Vec<&str> = path.split('/').collect();

        for part in &parts[..parts.len().saturating_sub(1)] {
            if !part.is_empty() {
                if !current_path.is_empty() {
                    current_path.push('/');
                }
                current_path.push_str(part);
                self.add_entry(&current_path, &[], 0o040755, 0);
            }
        }
    }

    fn add_entry(&mut self, path: &str, content: &[u8], mode: u32, size: usize) {
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

        self.data.extend_from_slice(header.as_bytes());
        self.data.extend_from_slice(path.as_bytes());
        self.data.push(0);

        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }

        self.data.extend_from_slice(content);

        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
    }

    fn finalize(mut self) -> Vec<u8> {
        self.add_entry("TRAILER!!!", &[], 0, 0);
        self.data
    }
}

#[test]
fn test_cpio_header_format() {
    let mut builder = CpioBuilder::new();
    builder.add_file("test.txt", b"hello world", 0o644);
    let archive = builder.finalize();

    // check magic number
    assert_eq!(&archive[0..6], b"070701");

    // check alignment
    assert_eq!(archive.len() % 4, 0);
}

#[test]
fn test_cpio_with_system_tool() {
    let mut builder = CpioBuilder::new();
    builder.add_file("lib/modules/test.ko", b"fake module content", 0o644);
    builder.add_file("etc/boot-modules.conf", b"/lib/modules/test.ko\n", 0o644);
    let archive = builder.finalize();

    // write to temp file
    let temp_path = "/tmp/test_cpio.cpio";
    let mut file = std::fs::File::create(temp_path).unwrap();
    file.write_all(&archive).unwrap();
    drop(file);

    // verify with cpio -t
    let output = Command::new("cpio")
        .args(["-t", "-F", temp_path])
        .output()
        .expect("failed to run cpio");

    let listing = String::from_utf8_lossy(&output.stdout);
    println!("cpio listing:\n{}", listing);

    assert!(output.status.success(), "cpio -t failed: {:?}", output.stderr);
    assert!(listing.contains("lib/modules/test.ko"));
    assert!(listing.contains("etc/boot-modules.conf"));

    // clean up
    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_cpio_extract() {
    let mut builder = CpioBuilder::new();
    let content = b"module binary data here";
    builder.add_file("lib/modules/mymod.ko", content, 0o644);
    let archive = builder.finalize();

    // write archive
    let temp_path = "/tmp/test_extract.cpio";
    std::fs::write(temp_path, &archive).unwrap();

    // create extract dir
    let extract_dir = "/tmp/test_cpio_extract";
    std::fs::create_dir_all(extract_dir).ok();

    // extract with cpio
    let output = Command::new("sh")
        .args(["-c", &format!("cd {} && cpio -id < {}", extract_dir, temp_path)])
        .output()
        .expect("failed to run cpio extract");

    assert!(output.status.success(), "cpio extract failed: {:?}",
        String::from_utf8_lossy(&output.stderr));

    // verify content
    let extracted = std::fs::read(format!("{}/lib/modules/mymod.ko", extract_dir)).unwrap();
    assert_eq!(&extracted, content);

    // clean up
    std::fs::remove_file(temp_path).ok();
    std::fs::remove_dir_all(extract_dir).ok();
}

fn main() {
    println!("run with: cargo test");
}
