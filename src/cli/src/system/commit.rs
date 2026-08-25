//! Store commits for finished system root filesystems.

use std::io;
use std::path::Path;

use crate::manifest::SystemManifest;
use crate::store::{commit_tree, encode_metadata_list};

/// Commit the system target directory and attach system metadata.
pub fn commit_system_rootfs(
    manifest: &SystemManifest,
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
    dependency_commits: &[String],
    base_commit: Option<&str>,
    checksum: &str,
) -> io::Result<()> {
    let target_dir = Path::new(base_dir).join("target");
    if !target_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "System target directory missing after build",
        ));
    }

    let branch_name = format!(
        "systems/{}/{}",
        manifest.system.slug, manifest.system.version
    );
    commit_tree(
        repo_path,
        &branch_name,
        &target_dir,
        &system_metadata(
            manifest,
            package_commits,
            dependency_commits,
            base_commit,
            checksum,
        )?,
    )?;
    Ok(())
}

fn system_metadata(
    manifest: &SystemManifest,
    package_commits: &[String],
    dependency_commits: &[String],
    base_commit: Option<&str>,
    checksum: &str,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = vec![
        ("nex.system.name".to_string(), manifest.system.name.clone()),
        ("nex.system.slug".to_string(), manifest.system.slug.clone()),
        (
            "nex.system.version".to_string(),
            manifest.system.version.clone(),
        ),
        ("nex.build.checksum".to_string(), checksum.to_string()),
    ];
    push_optional_metadata(manifest, &mut metadata);
    if let Some(commit) = base_commit {
        metadata.push(("nex.system.base".to_string(), commit.to_string()));
    }
    push_commit_lists(&mut metadata, package_commits, dependency_commits)?;
    Ok(metadata)
}

fn push_optional_metadata(manifest: &SystemManifest, metadata: &mut Vec<(String, String)>) {
    if let Some(desc) = &manifest.system.description {
        metadata.push(("nex.system.description".to_string(), desc.clone()));
    }
    if let Some(arch) = &manifest.system.architecture {
        metadata.push(("nex.system.arch".to_string(), arch.clone()));
    }
    if let Some(boot) = &manifest.system.boot_method {
        metadata.push(("nex.system.boot_method".to_string(), boot.clone()));
    }
}

fn push_commit_lists(
    metadata: &mut Vec<(String, String)>,
    package_commits: &[String],
    dependency_commits: &[String],
) -> io::Result<()> {
    if let Some(encoded) = encode_metadata_list(package_commits)? {
        metadata.push(("nex.system.packages".to_string(), encoded));
    }
    if let Some(encoded) = encode_metadata_list(dependency_commits)? {
        metadata.push(("nex.system.dependencies".to_string(), encoded));
    }
    Ok(())
}

#[cfg(test)]
#[path = "commit_tests.rs"]
mod commit_tests;
