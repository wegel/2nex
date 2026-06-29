use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use super::calculate_output_checksum;

#[test]
fn output_checksum_changes_when_symlink_target_changes() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    symlink("target-one", first.path().join("tool")).unwrap();
    symlink("target-two", second.path().join("tool")).unwrap();

    let first_checksum = calculate_output_checksum(first.path()).unwrap();
    let second_checksum = calculate_output_checksum(second.path()).unwrap();

    assert_ne!(first_checksum, second_checksum);
}

#[test]
fn output_checksum_distinguishes_file_from_symlink() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    fs::write(first.path().join("tool"), "target").unwrap();
    symlink("target", second.path().join("tool")).unwrap();

    let first_checksum = calculate_output_checksum(first.path()).unwrap();
    let second_checksum = calculate_output_checksum(second.path()).unwrap();

    assert_ne!(first_checksum, second_checksum);
}

#[test]
fn output_checksum_changes_when_file_mode_changes() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    fs::write(first.path().join("tool"), "tool").unwrap();
    fs::write(second.path().join("tool"), "tool").unwrap();
    fs::set_permissions(
        second.path().join("tool"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    let first_checksum = calculate_output_checksum(first.path()).unwrap();
    let second_checksum = calculate_output_checksum(second.path()).unwrap();

    assert_ne!(first_checksum, second_checksum);
}

#[test]
fn output_checksum_changes_when_empty_directory_is_added() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    fs::write(first.path().join("tool"), "tool").unwrap();
    fs::write(second.path().join("tool"), "tool").unwrap();
    fs::create_dir(second.path().join("empty")).unwrap();

    let first_checksum = calculate_output_checksum(first.path()).unwrap();
    let second_checksum = calculate_output_checksum(second.path()).unwrap();

    assert_ne!(first_checksum, second_checksum);
}
