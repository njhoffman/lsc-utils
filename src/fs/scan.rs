//! Directory scanning + filtering.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::entry::FileEntry;
use crate::options::Filter;

/// Read directory `dir` and return entries filtered per `filter`.
///
/// - `all=true` includes `.` and `..` synthetically (matching colorls).
/// - `almost_all=true` includes dotfiles but excludes `.` and `..`.
/// - Neither set: hides any entry starting with `.`.
/// - `only_dirs` / `only_files` partition by kind.
pub fn scan_directory(dir: &Path, filter: &Filter) -> Result<Vec<FileEntry>> {
    let mut out = Vec::new();
    let read = std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))?;
    for ent in read {
        let ent = ent.with_context(|| format!("read_dir entry under {}", dir.display()))?;
        let path = ent.path();
        // We tolerate per-entry stat failures (e.g. broken symlink chains): the
        // entry is skipped with a debug trace so a single bad inode doesn't
        // abort the whole listing.
        match FileEntry::from_path(path.clone()) {
            Ok(e) => out.push(e),
            Err(err) => {
                tracing::debug!(path = %path.display(), error = %err, "skipping entry");
            }
        }
    }

    if filter.all {
        // Synthesize `.` and `..` so they participate in sorting and rendering.
        push_synthetic(&mut out, dir, ".")?;
        if let Some(parent) = dir.parent() {
            push_synthetic(&mut out, parent, "..")?;
        }
    } else if !filter.almost_all {
        out.retain(|e| !e.is_hidden());
    }

    if filter.only_dirs && !filter.only_files {
        out.retain(|e| e.is_dir());
    } else if filter.only_files && !filter.only_dirs {
        out.retain(|e| !e.is_dir());
    }

    Ok(out)
}

fn push_synthetic(out: &mut Vec<FileEntry>, path: &Path, name: &str) -> Result<()> {
    let mut e = FileEntry::from_path(path.to_path_buf())
        .with_context(|| format!("synthetic {} for {}", name, path.display()))?;
    e.name = name.into();
    e.path = PathBuf::from(name);
    out.push(e);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use tempfile::TempDir;

    fn names(entries: &[FileEntry]) -> BTreeSet<String> {
        entries
            .iter()
            .map(|e| e.name_lossy().into_owned())
            .collect()
    }

    fn fixture() -> TempDir {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("visible.txt"), "").unwrap();
        std::fs::write(tmp.path().join(".hidden"), "").unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();
        std::fs::create_dir(tmp.path().join(".dotdir")).unwrap();
        tmp
    }

    #[test]
    fn default_filter_hides_dotfiles() {
        let tmp = fixture();
        let entries = scan_directory(tmp.path(), &Filter::default()).unwrap();
        let n = names(&entries);
        assert!(n.contains("visible.txt"));
        assert!(n.contains("subdir"));
        assert!(!n.contains(".hidden"));
        assert!(!n.contains(".dotdir"));
    }

    #[test]
    fn almost_all_shows_dotfiles_but_not_dot_dotdot() {
        let tmp = fixture();
        let entries = scan_directory(
            tmp.path(),
            &Filter {
                almost_all: true,
                ..Filter::default()
            },
        )
        .unwrap();
        let n = names(&entries);
        assert!(n.contains(".hidden"));
        assert!(n.contains(".dotdir"));
        assert!(!n.contains("."));
        assert!(!n.contains(".."));
    }

    #[test]
    fn all_includes_dot_and_dotdot() {
        let tmp = fixture();
        let entries = scan_directory(
            tmp.path(),
            &Filter {
                all: true,
                ..Filter::default()
            },
        )
        .unwrap();
        let n = names(&entries);
        assert!(n.contains("."));
        // `..` only appears when the dir has a parent (always true for tempdirs).
        assert!(n.contains(".."));
        assert!(n.contains(".hidden"));
    }

    #[test]
    fn only_dirs_filters_files() {
        let tmp = fixture();
        let entries = scan_directory(
            tmp.path(),
            &Filter {
                only_dirs: true,
                ..Filter::default()
            },
        )
        .unwrap();
        let n = names(&entries);
        assert!(n.contains("subdir"));
        assert!(!n.contains("visible.txt"));
    }

    #[test]
    fn only_files_filters_dirs() {
        let tmp = fixture();
        let entries = scan_directory(
            tmp.path(),
            &Filter {
                only_files: true,
                ..Filter::default()
            },
        )
        .unwrap();
        let n = names(&entries);
        assert!(n.contains("visible.txt"));
        assert!(!n.contains("subdir"));
    }

    #[test]
    fn missing_dir_errors_with_context() {
        let err = scan_directory(Path::new("/nonexistent/zzzzz"), &Filter::default()).unwrap_err();
        assert!(err.to_string().contains("read_dir"));
    }
}
