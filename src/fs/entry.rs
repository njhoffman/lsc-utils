//! Filesystem entry: a single item produced by directory scanning.
//!
//! Holds the bare data the renderer needs (name, kind, lstat metadata,
//! optional symlink target). Icon and color resolution happen at render
//! time so this struct stays cheap to construct.

use std::ffi::OsString;
use std::fs::Metadata;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Fifo,
    Socket,
    BlockDevice,
    CharDevice,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: OsString,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub meta: Metadata,
    /// Resolved target path of a symlink, if any. None for non-symlinks or
    /// if `readlink` failed.
    pub link_target: Option<PathBuf>,
}

impl FileEntry {
    /// Build a FileEntry from a path. Uses `lstat` (does not follow symlinks).
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let name = path
            .file_name()
            .map(OsString::from)
            .unwrap_or_else(|| path.as_os_str().to_owned());
        let meta = std::fs::symlink_metadata(&path)
            .with_context(|| format!("lstat {}", path.display()))?;
        let kind = classify(&meta);
        let link_target = if meta.file_type().is_symlink() {
            std::fs::read_link(&path).ok()
        } else {
            None
        };
        Ok(Self {
            name,
            path,
            kind,
            meta,
            link_target,
        })
    }

    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Directory)
    }

    pub fn is_hidden(&self) -> bool {
        self.name
            .to_string_lossy()
            .as_bytes()
            .first()
            .is_some_and(|b| *b == b'.')
    }

    /// `true` if the executable bit is set on any of u/g/o.
    pub fn is_executable(&self) -> bool {
        use std::os::unix::fs::MetadataExt;
        self.meta.mode() & 0o111 != 0
    }

    pub fn name_lossy(&self) -> std::borrow::Cow<'_, str> {
        self.name.to_string_lossy()
    }
}

fn classify(meta: &Metadata) -> EntryKind {
    let ft = meta.file_type();
    if ft.is_symlink() {
        EntryKind::Symlink
    } else if ft.is_dir() {
        EntryKind::Directory
    } else if ft.is_fifo() {
        EntryKind::Fifo
    } else if ft.is_socket() {
        EntryKind::Socket
    } else if ft.is_block_device() {
        EntryKind::BlockDevice
    } else if ft.is_char_device() {
        EntryKind::CharDevice
    } else {
        EntryKind::File
    }
}

/// Build a synthetic entry for a path that wasn't reached via directory scan
/// (e.g. when the user passed an explicit file path on the command line).
pub fn from_user_path(path: impl AsRef<Path>) -> Result<FileEntry> {
    FileEntry::from_path(path.as_ref().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn classifies_regular_file() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("a.txt");
        std::fs::write(&p, "x").unwrap();
        let e = FileEntry::from_path(p).unwrap();
        assert_eq!(e.kind, EntryKind::File);
        assert!(!e.is_dir());
    }

    #[test]
    fn classifies_directory() {
        let tmp = TempDir::new().unwrap();
        let d = tmp.path().join("sub");
        std::fs::create_dir(&d).unwrap();
        let e = FileEntry::from_path(d).unwrap();
        assert_eq!(e.kind, EntryKind::Directory);
        assert!(e.is_dir());
    }

    #[test]
    fn classifies_symlink_without_following() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("real.txt");
        std::fs::write(&target, "x").unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let e = FileEntry::from_path(link).unwrap();
        assert_eq!(e.kind, EntryKind::Symlink);
        assert_eq!(e.link_target.as_deref(), Some(target.as_path()));
    }

    #[test]
    fn detects_hidden() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join(".hidden");
        std::fs::write(&p, "").unwrap();
        let e = FileEntry::from_path(p).unwrap();
        assert!(e.is_hidden());
    }

    #[test]
    fn missing_path_errors_with_context() {
        let err = FileEntry::from_path(PathBuf::from("/nonexistent/zzzzzz")).unwrap_err();
        assert!(err.to_string().contains("lstat"));
    }
}
