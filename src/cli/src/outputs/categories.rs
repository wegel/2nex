//! Generated output category assignment for package files.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use walkdir::WalkDir;

use crate::manifest::OutputSpec;
use crate::utils::determine_category;

#[cfg(test)]
#[path = "categories_tests.rs"]
mod categories_tests;

/// Group rootfs files into generated outputs using default category rules.
pub fn categorize_files(rootfs_dir: &Path) -> io::Result<HashMap<String, Vec<String>>> {
    categorize_files_with_existing_outputs(rootfs_dir, &HashMap::new())
}

/// Group rootfs files while preserving paths already named by existing outputs.
pub fn categorize_files_with_existing_outputs(
    rootfs_dir: &Path,
    existing_outputs: &HashMap<String, OutputSpec>,
) -> io::Result<HashMap<String, Vec<String>>> {
    let known_paths = known_output_paths(existing_outputs);
    let mut outputs = HashMap::new();

    for entry in WalkDir::new(rootfs_dir) {
        let entry = entry.map_err(io::Error::other)?;
        if entry.file_type().is_file() || entry.file_type().is_symlink() {
            let relative_path_str = relative_output_path(entry.path(), rootfs_dir)?;
            let category = path_category(&relative_path_str, &known_paths);
            outputs
                .entry(category)
                .or_insert_with(Vec::new)
                .push(relative_path_str);
        }
    }

    sort_output_paths(&mut outputs);
    Ok(outputs)
}

fn known_output_paths(existing_outputs: &HashMap<String, OutputSpec>) -> HashMap<String, String> {
    let mut known_paths = HashMap::new();
    let mut output_names: Vec<&String> = existing_outputs.keys().collect();
    output_names.sort();

    for output_name in output_names {
        if let Some(output) = existing_outputs.get(output_name) {
            for file in &output.files {
                known_paths
                    .entry(file.path.clone())
                    .or_insert_with(|| output_name.clone());
            }
        }
    }

    known_paths
}

fn path_category(relative_path: &str, known_paths: &HashMap<String, String>) -> String {
    known_paths
        .get(relative_path)
        .cloned()
        .unwrap_or_else(|| determine_category(relative_path))
}

fn sort_output_paths(outputs: &mut HashMap<String, Vec<String>>) {
    for paths in outputs.values_mut() {
        paths.sort();
    }
}

fn relative_output_path(path: &Path, rootfs_dir: &Path) -> io::Result<String> {
    let relative_path = path.strip_prefix(rootfs_dir).map_err(io::Error::other)?;
    let relative_path = relative_path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("output path is not valid UTF-8: {}", path.display()),
        )
    })?;
    Ok(format!("/{}", relative_path))
}
