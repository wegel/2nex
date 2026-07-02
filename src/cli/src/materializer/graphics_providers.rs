//! Runtime graphics provider files for Nex capsules.
//!
//! ELF scanning finds loader libraries such as libEGL and libgbm, but GLVND,
//! GBM, and Vulkan load vendor providers from JSON and plugin directories.
//! A system that selects an Nvidia provider must place those provider files in
//! graphics application capsules.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::manifest::types::Manifest;
use crate::manifest::ManifestIndex;
use crate::refs::PackageRef;

use super::flatten_export::flatten_library_preserving_path;

#[derive(Debug)]
struct GraphicsProvider {
    package_dir: PathBuf,
    commit: String,
    files: Vec<String>,
}

pub fn flatten_graphics_provider_files(
    repo_path: &str,
    nex_pkg_dir: &Path,
    manifest_index: &ManifestIndex,
) -> io::Result<usize> {
    let providers = graphics_providers(nex_pkg_dir, manifest_index)?;
    if providers.is_empty() {
        return Ok(0);
    }
    if providers.len() > 1 {
        return Err(multiple_provider_error(&providers));
    }

    let provider = &providers[0];
    let mut flattened_count = 0;
    for package_dir in package_capsule_dirs(nex_pkg_dir) {
        if package_dir == provider.package_dir || !uses_graphics_loader(&package_dir) {
            continue;
        }
        for file in &provider.files {
            if flatten_library_preserving_path(
                repo_path,
                &provider.commit,
                file,
                &package_dir,
                &[],
            )? {
                flattened_count += 1;
            }
        }
    }

    Ok(flattened_count)
}

fn graphics_providers(
    nex_pkg_dir: &Path,
    manifest_index: &ManifestIndex,
) -> io::Result<Vec<GraphicsProvider>> {
    let mut providers = Vec::new();
    for package_dir in package_capsule_dirs(nex_pkg_dir) {
        for commit in package_root_commits(&package_dir)? {
            if !is_nvidia_provider_commit(&commit) {
                continue;
            }
            let manifest = manifest_for_commit(&commit, manifest_index)?;
            let files = provider_files_for_commit(&commit, manifest)?;
            if !files.is_empty() {
                providers.push(GraphicsProvider {
                    package_dir: package_dir.clone(),
                    commit,
                    files,
                });
            }
        }
    }
    Ok(providers)
}

fn package_capsule_dirs(nex_pkg_dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(nex_pkg_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| path.join(".nex-app-root").exists())
        .collect()
}

fn package_root_commits(pkg_dir: &Path) -> io::Result<Vec<String>> {
    Ok(std::fs::read_to_string(pkg_dir.join(".nex-app-root"))?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn is_nvidia_provider_commit(commit: &str) -> bool {
    PackageRef::parse(commit).is_ok_and(|package_ref| {
        package_ref.namespace == "libs/graphics" && package_ref.slug.starts_with("nvidia-")
    })
}

fn manifest_for_commit<'a>(
    commit: &str,
    manifest_index: &'a ManifestIndex,
) -> io::Result<&'a Manifest> {
    let package_ref = PackageRef::parse(commit).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid package commit ref {}: {}", commit, error),
        )
    })?;
    manifest_index
        .get_manifest(&package_ref.namespace, &package_ref.slug)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no manifest was found for graphics provider {}", commit),
            )
        })
}

fn provider_files_for_commit(commit: &str, manifest: &Manifest) -> io::Result<Vec<String>> {
    let mut files = BTreeSet::new();
    for output_name in output_names_for_commit(commit, manifest)? {
        let Some(output) = manifest.outputs.get(&output_name) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "graphics provider {} names missing output '{}'",
                    manifest.package.slug, output_name
                ),
            ));
        };
        for file in &output.files {
            if is_graphics_provider_path(&file.path) {
                files.insert(file.path.clone());
            }
        }
    }
    Ok(files.into_iter().collect())
}

fn output_names_for_commit(commit: &str, manifest: &Manifest) -> io::Result<Vec<String>> {
    let commit_parts: Vec<&str> = commit.split('/').collect();
    let commit_type = commit_parts
        .get(commit_parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("");
    let commit_name = commit_parts.last().copied().unwrap_or("");

    match commit_type {
        "bundles" => manifest
            .bundles
            .get(commit_name)
            .map(|bundle| bundle.includes.clone())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "graphics provider {} names missing bundle '{}'",
                        manifest.package.slug, commit_name
                    ),
                )
            }),
        "outputs" => Ok(vec![commit_name.to_string()]),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "graphics provider commit {} is not an output or bundle",
                commit
            ),
        )),
    }
}

fn is_graphics_provider_path(path: &str) -> bool {
    path.starts_with("/usr/lib/libEGL_nvidia.so")
        || path.starts_with("/usr/lib/libGLESv1_CM_nvidia.so")
        || path.starts_with("/usr/lib/libGLESv2_nvidia.so")
        || path.starts_with("/usr/lib/libGLX_nvidia.so")
        || path.starts_with("/usr/lib/libnvidia")
        || path.starts_with("/usr/lib/gbm/")
        || path.starts_with("/usr/share/egl/egl_external_platform.d/")
        || path.starts_with("/usr/share/glvnd/egl_vendor.d/")
        || path.starts_with("/usr/share/vulkan/icd.d/")
        || path.starts_with("/usr/share/vulkan/implicit_layer.d/")
}

fn uses_graphics_loader(package_dir: &Path) -> bool {
    [
        "usr/lib/libEGL.so.1",
        "usr/lib/libgbm.so.1",
        "usr/lib/libGLX.so.0",
        "usr/lib/libvulkan.so.1",
    ]
    .iter()
    .any(|path| path_exists_or_link(&package_dir.join(path)))
}

fn path_exists_or_link(path: &Path) -> bool {
    path.exists() || path.symlink_metadata().is_ok()
}

fn multiple_provider_error(providers: &[GraphicsProvider]) -> io::Error {
    let provider_names = providers
        .iter()
        .map(|provider| provider.commit.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "multiple Nvidia graphics providers are installed: {}; select exactly one provider",
            provider_names
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::{is_graphics_provider_path, is_nvidia_provider_commit, uses_graphics_loader};

    #[test]
    fn detects_nvidia_provider_commits() {
        assert!(is_nvidia_provider_commit(
            "x86_64/pkg/libs/graphics/nvidia-580/580.159.04/bundles/runtime"
        ));
        assert!(is_nvidia_provider_commit(
            "x86_64/pkg/libs/graphics/nvidia-current/595.84/bundles/runtime"
        ));
        assert!(!is_nvidia_provider_commit(
            "x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/lib"
        ));
    }

    #[test]
    fn selects_loader_provider_files() {
        assert!(is_graphics_provider_path(
            "/usr/share/glvnd/egl_vendor.d/10_nvidia.json"
        ));
        assert!(is_graphics_provider_path(
            "/usr/share/egl/egl_external_platform.d/15_nvidia_gbm.json"
        ));
        assert!(is_graphics_provider_path("/usr/lib/gbm/nvidia-drm_gbm.so"));
        assert!(is_graphics_provider_path("/usr/lib/libEGL_nvidia.so.0"));
        assert!(is_graphics_provider_path(
            "/usr/lib/libnvidia-allocator.so.1"
        ));
        assert!(!is_graphics_provider_path("/usr/bin/nvidia-smi"));
        assert!(!is_graphics_provider_path("/usr/lib/libEGL.so.1"));
    }

    #[test]
    fn detects_graphics_loader_capsules() {
        let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
        let lib_dir = temp_dir.path().join("usr/lib");
        std::fs::create_dir_all(&lib_dir).expect("test setup should succeed");
        std::fs::write(lib_dir.join("libEGL.so.1"), b"").expect("test setup should succeed");

        assert!(uses_graphics_loader(temp_dir.path()));
    }
}
