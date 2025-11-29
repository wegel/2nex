//! OSTree dirtree object parsing.

use std::io;

use gvariant::{aligned_bytes::copy_to_align, gv, Marker, Structure};

/// a file entry in a dirtree.
pub struct FileEntry {
    pub name: String,
    pub checksum: String,
}

/// a directory entry in a dirtree.
pub struct DirEntry {
    pub name: String,
    pub tree_checksum: String,
    pub meta_checksum: String,
}

/// parsed dirtree contents.
pub struct DirTree {
    pub files: Vec<FileEntry>,
    pub dirs: Vec<DirEntry>,
}

/// parse a dirtree object.
/// dirtree format: (a(say)a(sayay))
///   - a(say): files array - [(filename, checksum)]
///   - a(sayay): dirs array - [(dirname, tree_checksum, meta_checksum)]
pub fn parse_dirtree(data: &[u8]) -> io::Result<DirTree> {
    let aligned = copy_to_align(data);

    let tree = gv!("(a(say)a(sayay))").cast(aligned.as_ref());
    let (files_arr, dirs_arr) = tree.to_tuple();

    let mut files = Vec::new();
    for f in files_arr {
        let (name, checksum_bytes) = f.to_tuple();
        files.push(FileEntry {
            name: name.to_str().to_string(),
            checksum: hex::encode(checksum_bytes.as_ref()),
        });
    }

    let mut dirs = Vec::new();
    for d in dirs_arr {
        let (name, tree_cs, meta_cs) = d.to_tuple();
        dirs.push(DirEntry {
            name: name.to_str().to_string(),
            tree_checksum: hex::encode(tree_cs.as_ref()),
            meta_checksum: hex::encode(meta_cs.as_ref()),
        });
    }

    Ok(DirTree { files, dirs })
}
