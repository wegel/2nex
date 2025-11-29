//! OSTree repository facade.

use std::io;
use std::path::{Path, PathBuf};

use super::checkout::{self, CheckoutOptions};
use super::commit::{self, CommitInfo};
use super::dirtree;
use super::objects::{ObjectStore, ObjectType};
use super::refs;

/// native OSTree bare-user repository access.
pub struct OstreeRepo {
    pub path: PathBuf,
    pub objects: ObjectStore,
}

impl OstreeRepo {
    /// open an existing OSTree repository.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        // verify it's a valid ostree repo
        if !path.join("config").exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("not an OSTree repository: {} (missing config)", path.display()),
            ));
        }
        if !path.join("objects").exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("not an OSTree repository: {} (missing objects)", path.display()),
            ));
        }

        let objects = ObjectStore::new(path.join("objects"));
        Ok(Self { path, objects })
    }

    /// list all refs, optionally filtered by pattern.
    pub fn refs(&self, pattern: Option<&str>) -> io::Result<Vec<String>> {
        refs::list_refs(&self.path, pattern)
    }

    /// resolve a ref to its commit checksum.
    pub fn resolve_ref(&self, ref_name: &str) -> io::Result<String> {
        refs::resolve_ref(&self.path, ref_name)
    }

    /// checkout a commit to target directory.
    pub fn checkout(&self, commit: &str, target: &Path, union: bool) -> io::Result<()> {
        let options = CheckoutOptions { union };
        checkout::checkout(&self.path, &self.objects, commit, target, &options)
    }

    /// get commit metadata.
    pub fn get_commit_info(&self, commit: &str) -> io::Result<CommitInfo> {
        let checksum = refs::resolve_ref_or_checksum(&self.path, commit)?;
        let data = self.objects.read_object(&checksum, ObjectType::Commit)?;
        commit::parse_commit(&data)
    }

    /// get a specific metadata key from a commit.
    pub fn get_metadata(&self, commit: &str, key: &str) -> io::Result<Option<String>> {
        let info = self.get_commit_info(commit)?;
        Ok(info.metadata.get(key).cloned())
    }

    /// list files in a commit (recursive directory listing).
    pub fn ls(&self, commit: &str) -> io::Result<Vec<String>> {
        let checksum = refs::resolve_ref_or_checksum(&self.path, commit)?;
        let commit_data = self.objects.read_object(&checksum, ObjectType::Commit)?;
        let commit_info = commit::parse_commit(&commit_data)?;

        let mut files = Vec::new();
        self.ls_tree_recursive(&commit_info.root_tree, PathBuf::new(), &mut files)?;
        Ok(files)
    }

    /// recursively list files in a dirtree.
    fn ls_tree_recursive(
        &self,
        tree_checksum: &str,
        prefix: PathBuf,
        files: &mut Vec<String>,
    ) -> io::Result<()> {
        let tree_data = self.objects.read_object(tree_checksum, ObjectType::DirTree)?;
        let dirtree = dirtree::parse_dirtree(&tree_data)?;

        for file in &dirtree.files {
            let path = prefix.join(&file.name);
            files.push(path.to_string_lossy().to_string());
        }

        for dir in &dirtree.dirs {
            let subdir = prefix.join(&dir.name);
            // add directory entry
            files.push(format!("{}/", subdir.to_string_lossy()));
            // recurse
            self.ls_tree_recursive(&dir.tree_checksum, subdir, files)?;
        }

        Ok(())
    }

    /// read a file from a commit.
    pub fn cat(&self, commit: &str, path: &str) -> io::Result<Vec<u8>> {
        let checksum = refs::resolve_ref_or_checksum(&self.path, commit)?;
        let commit_data = self.objects.read_object(&checksum, ObjectType::Commit)?;
        let commit_info = commit::parse_commit(&commit_data)?;

        // navigate to the file
        self.cat_from_tree(&commit_info.root_tree, path)
    }

    /// find and read a file from a dirtree.
    fn cat_from_tree(&self, tree_checksum: &str, path: &str) -> io::Result<Vec<u8>> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        if parts.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty path"));
        }

        let tree_data = self.objects.read_object(tree_checksum, ObjectType::DirTree)?;
        let dirtree = dirtree::parse_dirtree(&tree_data)?;

        if parts.len() == 1 {
            // looking for a file in this directory
            let filename = parts[0];
            for file in &dirtree.files {
                if file.name == filename {
                    return self.objects.read_file(&file.checksum);
                }
            }
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("file not found: {}", path),
            ));
        }

        // looking in a subdirectory
        let dirname = parts[0];
        let rest = parts[1..].join("/");

        for dir in &dirtree.dirs {
            if dir.name == dirname {
                return self.cat_from_tree(&dir.tree_checksum, &rest);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("directory not found: {}", dirname),
        ))
    }

    /// check if a commit/ref exists.
    pub fn exists(&self, commit_or_ref: &str) -> bool {
        refs::resolve_ref_or_checksum(&self.path, commit_or_ref).is_ok()
    }
}
