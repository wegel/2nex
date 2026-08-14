use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;

use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;
use walkdir::WalkDir;

/// Classifies an output path into the manifest output category the builder
/// expects. The logic matches the manifest schema so both the runtime scanner
/// and the manifest pretty-printer can share the same categorization rules.
pub fn determine_category(file_path: &str) -> String {
    if file_path.ends_with(".so") || file_path.contains(".so.") {
        "lib".to_string()
    } else if is_kernel_module_sdk_path(file_path) {
        "module-sdk".to_string()
    } else if file_path.contains("/include/") {
        "dev".to_string()
    } else if file_path.ends_with(".pc") || file_path.contains("/pkgconfig/") {
        "dev".to_string()
    } else if file_path.ends_with(".la") {
        "dev".to_string()
    } else if file_path.ends_with(".a") {
        "static".to_string()
    } else if file_path.contains("/share/man/") {
        "man".to_string()
    } else if file_path.contains("/share/info/") {
        "info".to_string()
    } else if file_path.contains("/share/doc") {
        "doc".to_string()
    } else if file_path.contains("/locale/") {
        "locale".to_string()
    } else if file_path.contains("/bin/") {
        "bin".to_string()
    } else if file_path.contains("/libexec/") {
        "bin".to_string()
    } else if file_path == "/boot" || file_path.starts_with("/boot/") {
        "boot".to_string()
    } else if file_path.contains("/lib/") || file_path.contains("/lib64/") {
        "lib".to_string()
    } else if file_path.contains("/conf/")
        || file_path.contains("/etc/")
        || file_path.ends_with(".conf")
    {
        "conf".to_string()
    } else if file_path.contains("/share/fonts/")
        || file_path.ends_with(".ttf")
        || file_path.ends_with(".otf")
        || file_path.ends_with(".ttc")
        || file_path.ends_with(".otc")
    {
        "fonts".to_string()
    } else {
        "misc".to_string()
    }
}

/// Return a display prefix for a hash-like string without panicking on short input.
pub fn short_hash(value: &str) -> &str {
    value.get(..12).unwrap_or(value)
}

fn is_kernel_module_sdk_path(file_path: &str) -> bool {
    if file_path.starts_with("/usr/src/linux-") {
        return true;
    }

    if let Some(suffix) = file_path.strip_prefix("/usr/lib/modules/") {
        let mut parts = suffix.split('/');
        if parts.next().is_some() {
            return matches!(parts.next(), Some("build" | "source"));
        }
    }

    false
}

/// Fetch one file from a committed Git repository snapshot.
pub fn fetch_git_file(
    repo_root: &Path,
    revision: &str,
    relative_path: &Path,
) -> io::Result<Vec<u8>> {
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Git source path must stay inside its repository: {}",
                relative_path.display()
            ),
        ));
    }

    let object = format!("{}:{}", revision, relative_path.to_string_lossy());
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["cat-file", "blob", &object])
        .output()
        .map_err(|error| io::Error::other(format!("failed to run git: {}", error)))?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "failed to fetch {} from Git revision {} in {}: {}",
                relative_path.display(),
                revision,
                repo_root.display(),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(output.stdout)
}

/// Calculate the git blob SHA of a file's content.
pub fn hash_file_content(path: &Path) -> io::Result<String> {
    let output = Command::new("git")
        .arg("hash-object")
        .arg(path)
        .output()
        .map_err(|e| io::Error::other(format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "failed to hash file {}: {}",
            path.display(),
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// recursively copy a directory
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

/// fetch file content from URL or local path
pub fn fetch_url_or_file(url_or_path: &str) -> io::Result<String> {
    if url_or_path.starts_with("http://") || url_or_path.starts_with("https://") {
        let output = Command::new("curl")
            .args(["-L", "-f", "-s", url_or_path])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::other(format!(
                "Failed to download {}",
                url_or_path
            )));
        }

        String::from_utf8(output.stdout).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e))
        })
    } else {
        fs::read_to_string(url_or_path)
    }
}

/// create a deterministic tarball from a directory.
/// uses fixed mtime, uid/gid 0, sorted file order, and gzip -9.
pub fn create_deterministic_tarball(source_dir: &Path, output_path: &Path) -> io::Result<()> {
    use std::path::PathBuf;

    // fixed timestamp: 2024-01-01 00:00:00 UTC
    let mtime = 1704067200u64;

    // collect and sort all entries
    let mut entries: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(source_dir).min_depth(0).into_iter() {
        let entry = entry.map_err(|e| io::Error::other(e.to_string()))?;
        entries.push(entry.path().to_path_buf());
    }

    // sort entries (directories before their contents, lexicographic)
    entries.sort();

    // create tarball
    let file = File::create(output_path)?;
    let encoder = GzEncoder::new(file, Compression::best());
    let mut tar = Builder::new(encoder);

    // get parent for relative path calculation
    let parent = source_dir.parent().unwrap_or(source_dir);

    for path in &entries {
        let relative = path
            .strip_prefix(parent)
            .map_err(|e| io::Error::other(e.to_string()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let metadata = fs::symlink_metadata(path)?;

        if metadata.is_dir() {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_mtime(mtime);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mode(0o755);
            tar.append_data(&mut header, relative, io::empty())?;
        } else if metadata.is_symlink() {
            let link_target = fs::read_link(path)?;
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mtime(mtime);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mode(0o777);
            tar.append_link(&mut header, relative, &link_target)?;
        } else if metadata.is_file() {
            let mut file = File::open(path)?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;

            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(contents.len() as u64);
            header.set_mtime(mtime);
            header.set_uid(0);
            header.set_gid(0);
            // preserve executable bit
            let mode = if metadata.permissions().mode() & 0o111 != 0 {
                0o755
            } else {
                0o644
            };
            header.set_mode(mode);
            tar.append_data(&mut header, relative, &contents[..])?;
        }
    }

    tar.finish()?;
    Ok(())
}

/// create an uncompressed deterministic tarball from a directory.
/// uses fixed mtime, uid/gid 0, sorted file order.
/// used for dev sources where speed matters more than size.
pub fn create_deterministic_tarball_uncompressed(
    source_dir: &Path,
    output_path: &Path,
) -> io::Result<()> {
    use std::io::BufWriter;
    use std::path::PathBuf;

    let mtime = 1704067200u64;

    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(source_dir).min_depth(0).into_iter() {
        let entry = entry.map_err(|e| io::Error::other(e.to_string()))?;
        entries.push(entry.path().to_path_buf());
    }
    entries.sort();

    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);
    let mut tar = Builder::new(writer);

    let parent = source_dir.parent().unwrap_or(source_dir);

    for path in &entries {
        let relative = path
            .strip_prefix(parent)
            .map_err(|e| io::Error::other(e.to_string()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let metadata = fs::symlink_metadata(path)?;

        if metadata.is_dir() {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_mtime(mtime);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mode(0o755);
            tar.append_data(&mut header, relative, io::empty())?;
        } else if metadata.is_symlink() {
            let link_target = fs::read_link(path)?;
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mtime(mtime);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mode(0o777);
            tar.append_link(&mut header, relative, &link_target)?;
        } else if metadata.is_file() {
            let mut file = File::open(path)?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;

            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(contents.len() as u64);
            header.set_mtime(mtime);
            header.set_uid(0);
            header.set_gid(0);
            let mode = if metadata.permissions().mode() & 0o111 != 0 {
                0o755
            } else {
                0o644
            };
            header.set_mode(mode);
            tar.append_data(&mut header, relative, &contents[..])?;
        }
    }

    tar.finish()?;
    Ok(())
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(not(unix))]
trait PermissionsExt {
    fn mode(&self) -> u32 {
        0o644
    }
}

#[cfg(not(unix))]
impl PermissionsExt for std::fs::Permissions {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determine_category_handles_common_layouts() {
        assert_eq!(determine_category("/usr/lib/libfoo.so"), "lib");
        assert_eq!(determine_category("/usr/include/foo.h"), "dev");
        assert_eq!(determine_category("/etc/foo.conf"), "conf");
        assert_eq!(determine_category("/usr/share/doc/foo/readme"), "doc");
        assert_eq!(determine_category("/usr/bin/foo"), "bin");
        assert_eq!(determine_category("/boot/initramfs.cpio"), "boot");
        assert_eq!(
            determine_category("/usr/share/fonts/TTF/HackNerdFont-Regular.ttf"),
            "fonts"
        );
        assert_eq!(
            determine_category("/usr/src/linux-6.12.58/Module.symvers"),
            "module-sdk"
        );
        assert_eq!(
            determine_category("/usr/lib/modules/6.12.58/build"),
            "module-sdk"
        );
        assert_eq!(
            determine_category("/usr/lib/modules/6.12.58/source/include/linux/module.h"),
            "module-sdk"
        );
    }

    #[test]
    fn short_hash_handles_short_values() {
        assert_eq!(short_hash("abc"), "abc");
        assert_eq!(short_hash("1234567890123456"), "123456789012");
    }
}
