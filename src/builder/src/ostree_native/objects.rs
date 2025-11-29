//! OSTree object store access.

use std::fs;
use std::io;
use std::path::PathBuf;

/// object types in OSTree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Commit,
    DirTree,
    DirMeta,
    File,
}

impl ObjectType {
    pub fn extension(&self) -> &'static str {
        match self {
            ObjectType::Commit => "commit",
            ObjectType::DirTree => "dirtree",
            ObjectType::DirMeta => "dirmeta",
            ObjectType::File => "file",
        }
    }
}

/// access to OSTree object store.
pub struct ObjectStore {
    objects_dir: PathBuf,
}

impl ObjectStore {
    pub fn new(objects_dir: PathBuf) -> Self {
        Self { objects_dir }
    }

    /// get path to an object file.
    /// checksum format: 64 hex chars (sha256)
    /// returns: objects/{first2}/{rest}.{type}
    pub fn object_path(&self, checksum: &str, obj_type: ObjectType) -> PathBuf {
        let (prefix, rest) = checksum.split_at(2);
        self.objects_dir
            .join(prefix)
            .join(format!("{}.{}", rest, obj_type.extension()))
    }

    /// check if an object exists.
    pub fn exists(&self, checksum: &str, obj_type: ObjectType) -> bool {
        self.object_path(checksum, obj_type).exists()
    }

    /// read object raw bytes.
    pub fn read_object(&self, checksum: &str, obj_type: ObjectType) -> io::Result<Vec<u8>> {
        let path = self.object_path(checksum, obj_type);
        fs::read(&path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "failed to read {} object {}: {}",
                    obj_type.extension(),
                    checksum,
                    e
                ),
            )
        })
    }

    /// read a file object. in bare-user mode, file content is stored directly.
    pub fn read_file(&self, checksum: &str) -> io::Result<Vec<u8>> {
        self.read_object(checksum, ObjectType::File)
    }
}
