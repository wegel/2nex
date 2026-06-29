use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use tempfile::TempDir;

use super::calculate_output_checksum;

#[test]
fn output_checksum_changes_when_symlink_target_changes() {
    let first = TempDir::new().expect("test setup should succeed");
    let second = TempDir::new().expect("test setup should succeed");
    symlink("target-one", first.path().join("tool")).expect("test setup should succeed");
    symlink("target-two", second.path().join("tool")).expect("test setup should succeed");

    let first_checksum =
        calculate_output_checksum(first.path()).expect("test setup should succeed");
    let second_checksum =
        calculate_output_checksum(second.path()).expect("test setup should succeed");

    assert_ne!(first_checksum, second_checksum);
}

#[test]
fn output_checksum_distinguishes_file_from_symlink() {
    let first = TempDir::new().expect("test setup should succeed");
    let second = TempDir::new().expect("test setup should succeed");
    fs::write(first.path().join("tool"), "target").expect("test setup should succeed");
    symlink("target", second.path().join("tool")).expect("test setup should succeed");

    let first_checksum =
        calculate_output_checksum(first.path()).expect("test setup should succeed");
    let second_checksum =
        calculate_output_checksum(second.path()).expect("test setup should succeed");

    assert_ne!(first_checksum, second_checksum);
}

#[test]
fn output_checksum_changes_when_file_mode_changes() {
    let first = TempDir::new().expect("test setup should succeed");
    let second = TempDir::new().expect("test setup should succeed");
    fs::write(first.path().join("tool"), "tool").expect("test setup should succeed");
    fs::write(second.path().join("tool"), "tool").expect("test setup should succeed");
    fs::set_permissions(
        second.path().join("tool"),
        fs::Permissions::from_mode(0o755),
    )
    .expect("test setup should succeed");

    let first_checksum =
        calculate_output_checksum(first.path()).expect("test setup should succeed");
    let second_checksum =
        calculate_output_checksum(second.path()).expect("test setup should succeed");

    assert_ne!(first_checksum, second_checksum);
}

#[test]
fn output_checksum_changes_when_empty_directory_is_added() {
    let first = TempDir::new().expect("test setup should succeed");
    let second = TempDir::new().expect("test setup should succeed");
    fs::write(first.path().join("tool"), "tool").expect("test setup should succeed");
    fs::write(second.path().join("tool"), "tool").expect("test setup should succeed");
    fs::create_dir(second.path().join("empty")).expect("test setup should succeed");

    let first_checksum =
        calculate_output_checksum(first.path()).expect("test setup should succeed");
    let second_checksum =
        calculate_output_checksum(second.path()).expect("test setup should succeed");

    assert_ne!(first_checksum, second_checksum);
}

#[test]
fn output_checksum_rejects_fifo_entries() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let fifo_path = temp_dir.path().join("queue");
    let status = Command::new("mkfifo").arg(&fifo_path).status()?;
    assert!(status.success(), "mkfifo should create a test FIFO");

    let error = calculate_output_checksum(temp_dir.path())
        .expect_err("output checksum should reject unsupported file types");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("unsupported output file type"));
    Ok(())
}
