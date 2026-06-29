use std::collections::HashMap;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStringExt;

use tempfile::TempDir;

use crate::manifest::{FileEntry, OutputSpec};

use super::categorize_files_with_existing_outputs;

#[test]
fn generated_outputs_preserve_existing_output_names() {
    let temp_dir = TempDir::new().expect("test setup should succeed");
    let root = temp_dir.path();
    fs::create_dir_all(root.join("usr/lib/modules/1/kernel/drivers/net"))
        .expect("test setup should succeed");
    fs::create_dir_all(root.join("usr/lib/modules/1/kernel/drivers/gpu"))
        .expect("test setup should succeed");
    fs::write(
        root.join("usr/lib/modules/1/kernel/drivers/net/e1000e.ko"),
        "net",
    )
    .expect("test setup should succeed");
    fs::write(
        root.join("usr/lib/modules/1/kernel/drivers/gpu/amdgpu.ko"),
        "gpu",
    )
    .expect("test setup should succeed");

    let mut existing = HashMap::new();
    existing.insert(
        "drv-eth-intel".to_string(),
        OutputSpec {
            files: vec![FileEntry {
                path: "/usr/lib/modules/1/kernel/drivers/net/e1000e.ko".to_string(),
                needs: Vec::new(),
            }],
        },
    );

    let categorized =
        categorize_files_with_existing_outputs(root, &existing).expect("test setup should succeed");

    assert_eq!(
        categorized
            .get("drv-eth-intel")
            .expect("test setup should succeed"),
        &vec!["/usr/lib/modules/1/kernel/drivers/net/e1000e.ko".to_string()]
    );
    assert_eq!(
        categorized.get("lib").expect("test setup should succeed"),
        &vec!["/usr/lib/modules/1/kernel/drivers/gpu/amdgpu.ko".to_string()]
    );
}

#[test]
fn generated_outputs_reject_non_utf8_paths() {
    let temp_dir = TempDir::new().expect("test setup should succeed");
    let root = temp_dir.path();
    let bad_name = std::ffi::OsString::from_vec(b"bad-\xff".to_vec());
    fs::write(root.join(bad_name), "bad").expect("test setup should succeed");

    let error = categorize_files_with_existing_outputs(root, &HashMap::new()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("not valid UTF-8"));
}
