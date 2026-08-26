//! URL, local-file, and Git input helpers.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn download(url: &str, destination: &Path) -> io::Result<()> {
    curl(url, Some(destination)).map(|_| ())
}

pub(crate) fn text(reference: &str, manifest: &Path) -> io::Result<String> {
    if reference.starts_with("http://") || reference.starts_with("https://") {
        String::from_utf8(curl(reference, None)?.stdout)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    } else {
        fs::read_to_string(local_path(manifest, Path::new(reference)))
    }
}

fn curl(url: &str, destination: Option<&Path>) -> io::Result<std::process::Output> {
    let mut command = Command::new("curl");
    command.args(["--location", "--fail", "--silent", "--show-error"]);
    if let Some(path) = destination {
        command.arg("--output").arg(path);
    }
    let output = command.arg(url).output()?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(io::Error::other(format!(
            "curl failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

pub fn local_path(manifest: &Path, file: &Path) -> PathBuf {
    if file.is_absolute() {
        return file.to_path_buf();
    }
    let parent = manifest.parent().unwrap_or_else(|| Path::new("."));
    parent
        .ancestors()
        .map(|directory| directory.join(file))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| parent.join(file))
}

pub(crate) fn git_bundle(manifest: &Path, commit: &str, destination: &Path) -> io::Result<()> {
    let repository = manifest
        .ancestors()
        .find(|path| path.join(".git").exists())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "manifest is not in a Git tree"))?;
    let resolved = git_output(
        repository,
        &["rev-parse", "--verify"],
        &[&format!("{commit}^{{commit}}")],
    )?;
    let worktree = destination.with_extension("worktree");
    git_status(
        repository,
        &["worktree", "add", "--detach", "--quiet"],
        &[worktree.as_os_str(), resolved.as_ref()],
    )?;
    let bundle = git_status(
        &worktree,
        &["-c", "pack.threads=1", "bundle", "create"],
        &[destination.as_os_str(), "HEAD".as_ref()],
    );
    let cleanup = git_status(
        repository,
        &["worktree", "remove", "--force"],
        &[worktree.as_os_str()],
    );
    bundle.and(cleanup)
}

pub(crate) fn git_status(
    directory: &Path,
    arguments: &[&str],
    values: &[&std::ffi::OsStr],
) -> io::Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .args(values)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "git {} failed in {}: {}",
            arguments.join(" "),
            directory.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

pub(crate) fn clone_git(url: &str, revision: &str, checkout: &Path) -> io::Result<()> {
    fs::create_dir(checkout)?;
    git_status(checkout, &["init", "--quiet"], &[])?;
    git_status(checkout, &["remote", "add", "origin"], &[url.as_ref()])?;
    git_status(
        checkout,
        &["fetch", "--quiet", "--depth", "1", "origin"],
        &[revision.as_ref()],
    )?;
    git_status(checkout, &["checkout", "--quiet", "FETCH_HEAD"], &[])?;
    fs::remove_dir_all(checkout.join(".git"))
}

fn git_output(directory: &Path, arguments: &[&str], values: &[&str]) -> io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .args(values)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(io::Error::other(format!(
            "git {} failed in {}: {}",
            arguments.join(" "),
            directory.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(test)]
#[path = "fetch_tests.rs"]
mod tests;
