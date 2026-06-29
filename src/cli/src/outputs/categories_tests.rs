use std::collections::HashMap;
use std::fs;

use tempfile::TempDir;

use crate::manifest::{FileEntry, OutputSpec};

use super::categorize_files_with_existing_outputs;

#[test]
fn generated_outputs_preserve_existing_output_names() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("usr/lib/modules/1/kernel/drivers/net")).unwrap();
    fs::create_dir_all(root.join("usr/lib/modules/1/kernel/drivers/gpu")).unwrap();
    fs::write(
        root.join("usr/lib/modules/1/kernel/drivers/net/e1000e.ko"),
        "net",
    )
    .unwrap();
    fs::write(
        root.join("usr/lib/modules/1/kernel/drivers/gpu/amdgpu.ko"),
        "gpu",
    )
    .unwrap();

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

    let categorized = categorize_files_with_existing_outputs(root, &existing);

    assert_eq!(
        categorized.get("drv-eth-intel").unwrap(),
        &vec!["/usr/lib/modules/1/kernel/drivers/net/e1000e.ko".to_string()]
    );
    assert_eq!(
        categorized.get("lib").unwrap(),
        &vec!["/usr/lib/modules/1/kernel/drivers/gpu/amdgpu.ko".to_string()]
    );
}
