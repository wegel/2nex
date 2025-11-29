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

/// access to OSTree object store with optional fallback directories.
pub struct ObjectStore {
    objects_dir: PathBuf,
    fallback_dirs: Vec<PathBuf>,
}

impl ObjectStore {
    pub fn new(objects_dir: PathBuf) -> Self {
        Self {
            objects_dir,
            fallback_dirs: vec![],
        }
    }

    /// add a fallback directory to search for objects.
    pub fn with_fallback(mut self, fallback: PathBuf) -> Self {
        self.fallback_dirs.push(fallback);
        self
    }

    /// get path to an object file in primary store (for writes).
    /// checksum format: 64 hex chars (sha256)
    /// returns: objects/{first2}/{rest}.{type}
    pub fn object_path(&self, checksum: &str, obj_type: ObjectType) -> PathBuf {
        let (prefix, rest) = checksum.split_at(2);
        self.objects_dir
            .join(prefix)
            .join(format!("{}.{}", rest, obj_type.extension()))
    }

    /// find object in primary or fallback stores (for reads).
    /// returns the path to the object if found in any store.
    pub fn find_object(&self, checksum: &str, obj_type: ObjectType) -> Option<PathBuf> {
        let (prefix, rest) = checksum.split_at(2);
        let filename = format!("{}.{}", rest, obj_type.extension());

        // check primary first
        let primary = self.objects_dir.join(prefix).join(&filename);
        if primary.exists() {
            return Some(primary);
        }

        // then fallbacks
        for fallback in &self.fallback_dirs {
            let path = fallback.join(prefix).join(&filename);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// check if an object exists in primary store.
    pub fn exists(&self, checksum: &str, obj_type: ObjectType) -> bool {
        self.object_path(checksum, obj_type).exists()
    }

    /// check if an object exists in primary or any fallback store.
    pub fn exists_any(&self, checksum: &str, obj_type: ObjectType) -> bool {
        self.find_object(checksum, obj_type).is_some()
    }

    /// read object raw bytes from primary or fallback stores.
    pub fn read_object(&self, checksum: &str, obj_type: ObjectType) -> io::Result<Vec<u8>> {
        let path = self.find_object(checksum, obj_type).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("object not found: {} ({})", checksum, obj_type.extension()),
            )
        })?;
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
