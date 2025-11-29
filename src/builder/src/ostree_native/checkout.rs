//! OSTree checkout (file extraction).

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use super::commit::parse_commit;
use super::dirtree::parse_dirtree;
use super::objects::{ObjectStore, ObjectType};
use super::refs;

// file type bits from stat mode
const S_IFMT: u32 = 0o170000; // file type mask
const S_IFLNK: u32 = 0o120000; // symlink
const S_IFREG: u32 = 0o100000; // regular file

/// checkout options.
pub struct CheckoutOptions {
    /// merge with existing files instead of failing on conflicts
    pub union: bool,
}

impl Default for CheckoutOptions {
    fn default() -> Self {
        Self { union: true }
    }
}

/// checkout a commit to a target directory.
pub fn checkout(
    repo_path: &Path,
    objects: &ObjectStore,
    commit_ref: &str,
    target: &Path,
    options: &CheckoutOptions,
) -> io::Result<()> {
    // resolve ref to checksum if needed
    let commit_checksum = refs::resolve_ref_or_checksum(repo_path, commit_ref)?;

    // read and parse commit object
    let commit_data = objects.read_object(&commit_checksum, ObjectType::Commit)?;
    let commit_info = parse_commit(&commit_data)?;

    // checkout the root dirtree
    checkout_tree(objects, &commit_info.root_tree, target, options)?;

    Ok(())
}

/// recursively checkout a dirtree to target directory.
fn checkout_tree(
    objects: &ObjectStore,
    tree_checksum: &str,
    target: &Path,
    options: &CheckoutOptions,
) -> io::Result<()> {
    // read and parse dirtree
    let tree_data = objects.read_object(tree_checksum, ObjectType::DirTree)?;
    let dirtree = parse_dirtree(&tree_data)?;

    // create target directory
    fs::create_dir_all(target)?;

    // checkout files
    for file in &dirtree.files {
        let dest = target.join(&file.name);
        if options.union && dest.exists() {
            continue;
        }
        checkout_file(objects, &file.checksum, &dest)?;
    }

    // recurse into subdirectories
    for dir in &dirtree.dirs {
        let subdir = target.join(&dir.name);
        checkout_tree(objects, &dir.tree_checksum, &subdir, options)?;
    }

    Ok(())
}

/// checkout a single file using hardlinks (zero-copy).
/// in bare-user mode, file metadata is stored in user.ostreemeta xattr.
fn checkout_file(objects: &ObjectStore, checksum: &str, dest: &Path) -> io::Result<()> {
    let src_path = objects.object_path(checksum, ObjectType::File);

    // read file mode from user.ostreemeta xattr
    let mode = read_ostreemeta_mode(&src_path)?;
    let file_type = mode & S_IFMT;

    if file_type == S_IFLNK {
        // symlink: content is the target path
        let data = fs::read(&src_path)?;
        let target = String::from_utf8_lossy(&data);
        let target = target.trim_end_matches('\0');

        if dest.exists() || dest.symlink_metadata().is_ok() {
            fs::remove_file(dest)?;
        }
        symlink(target, dest)?;
    } else if file_type == S_IFREG {
        // regular file: hardlink from repo object store
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        if dest.exists() || dest.symlink_metadata().is_ok() {
            fs::remove_file(dest)?;
        }

        // hardlink - fails if cross-device (we want to know about this)
        fs::hard_link(&src_path, dest).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "failed to hardlink {} -> {}: {} (cross-device mounts not supported)",
                    src_path.display(),
                    dest.display(),
                    e
                ),
            )
        })?;

        // note: do NOT set_permissions on hardlinks - it would modify the repo object
        // the repo object should already have the correct permissions from commit time
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "unsupported file type {:o} for {}",
                file_type,
                src_path.display()
            ),
        ));
    }

    Ok(())
}

/// read the mode from user.ostreemeta xattr.
/// format: 12 bytes - uid(4) + gid(4) + mode(4), big-endian
fn read_ostreemeta_mode(path: &Path) -> io::Result<u32> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let path_cstr = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let name_cstr = CString::new("user.ostreemeta").unwrap();

    let mut buf = [0u8; 12];

    // use libc to read xattr
    let result = unsafe {
        nix::libc::getxattr(
            path_cstr.as_ptr(),
            name_cstr.as_ptr(),
            buf.as_mut_ptr() as *mut nix::libc::c_void,
            buf.len(),
        )
    };

    if result < 0 {
        let err = io::Error::last_os_error();
        return Err(io::Error::new(
            err.kind(),
            format!(
                "failed to read user.ostreemeta from {}: {}",
                path.display(),
                err
            ),
        ));
    }

    if result < 12 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "user.ostreemeta too short ({} bytes) on {}",
                result,
                path.display()
            ),
        ));
    }

    // mode is at offset 8, big-endian u32
    let mode = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
    Ok(mode)
}
