//! Git status integration via libgit2 (`git2` crate, vendored libgit2).
//!
//! Mirrors colorls's `lib/colorls/git.rb`:
//! - Per-file status: ?/A/M/D/R for untracked/added/modified/deleted/renamed.
//! - Per-directory status: aggregate of every descendant's flags.
//! - Unchanged: `  ✓ ` rendered in the `unchanged` Theme color.
//! - Output column is fixed at 4 chars: `rjust(3).ljust(4)`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use bitflags::bitflags;

use crate::config::{ColorMode, Theme};

bitflags! {
    /// Subset of git2::Status that we surface, after collapsing index/worktree
    /// distinctions into a single "this file is in state X" flag.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct GitFlags: u8 {
        const NEW       = 0b00001; // ? untracked
        const ADDED     = 0b00010; // A added (index_new)
        const MODIFIED  = 0b00100; // M modified (worktree or index)
        const DELETED   = 0b01000; // D deleted
        const RENAMED   = 0b10000; // R renamed
    }
}

#[derive(Debug)]
pub struct GitContext {
    workdir: PathBuf,
    statuses: HashMap<PathBuf, GitFlags>,
}

impl GitContext {
    /// Discover the repo containing `dir` and snapshot its status. Returns
    /// `Ok(None)` if `dir` isn't inside any git repo (callers treat that as
    /// "no git column").
    pub fn discover(dir: &Path) -> Result<Option<Self>> {
        let repo = match git2::Repository::discover(dir) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };
        let workdir = match repo.workdir() {
            Some(p) => p.to_path_buf(),
            None => return Ok(None), // bare repo
        };
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .recurse_untracked_dirs(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);
        let statuses = repo.statuses(Some(&mut opts))?;
        let mut map: HashMap<PathBuf, GitFlags> = HashMap::new();
        for entry in statuses.iter() {
            let Some(rel) = entry.path() else { continue };
            let flags = map_status(entry.status());
            if flags.is_empty() {
                continue;
            }
            let rel_path = PathBuf::from(rel);
            // Aggregate this entry's flags onto itself + every ancestor under
            // the workdir, so directory rows reflect their descendants' state.
            let mut cur = rel_path.as_path();
            loop {
                map.entry(cur.to_path_buf())
                    .and_modify(|f| *f |= flags)
                    .or_insert(flags);
                match cur.parent() {
                    Some(p) if !p.as_os_str().is_empty() => cur = p,
                    _ => break,
                }
            }
        }
        Ok(Some(Self {
            workdir,
            statuses: map,
        }))
    }

    /// Lookup the aggregated flags for `abs_path`. Returns empty when the
    /// path is outside the repo.
    pub fn flags_for(&self, abs_path: &Path) -> GitFlags {
        match abs_path.strip_prefix(&self.workdir) {
            Ok(rel) => self.statuses.get(rel).copied().unwrap_or_default(),
            Err(_) => GitFlags::empty(),
        }
    }
}

fn map_status(s: git2::Status) -> GitFlags {
    use git2::Status as S;
    let mut out = GitFlags::empty();
    if s.intersects(S::WT_NEW) {
        out |= GitFlags::NEW;
    }
    if s.intersects(S::INDEX_NEW) {
        out |= GitFlags::ADDED;
    }
    if s.intersects(S::WT_MODIFIED | S::INDEX_MODIFIED | S::WT_TYPECHANGE | S::INDEX_TYPECHANGE) {
        out |= GitFlags::MODIFIED;
    }
    if s.intersects(S::WT_DELETED | S::INDEX_DELETED) {
        out |= GitFlags::DELETED;
    }
    if s.intersects(S::WT_RENAMED | S::INDEX_RENAMED) {
        out |= GitFlags::RENAMED;
    }
    out
}

/// 4-character status column. `  ✓ ` for clean; otherwise per-flag chars
/// right-padded into a 4-char field with each letter individually colored,
/// matching colorls's `colored_status_symbols`.
pub fn render_status(flags: GitFlags, theme: &Theme, color: ColorMode) -> String {
    if flags.is_empty() {
        return theme.paint("unchanged", "  \u{2713} ", color);
    }
    let mut chars: Vec<char> = Vec::with_capacity(5);
    if flags.contains(GitFlags::NEW) {
        chars.push('?');
    }
    if flags.contains(GitFlags::ADDED) {
        chars.push('A');
    }
    if flags.contains(GitFlags::MODIFIED) {
        chars.push('M');
    }
    if flags.contains(GitFlags::DELETED) {
        chars.push('D');
    }
    if flags.contains(GitFlags::RENAMED) {
        chars.push('R');
    }
    let raw: String = chars.iter().collect();
    let padded = format!("{:<4}", format!("{raw:>3}"));
    padded
        .replace('?', &theme.paint("untracked", "?", color))
        .replace('A', &theme.paint("addition", "A", color))
        .replace('M', &theme.paint("modification", "M", color))
        .replace('D', &theme.paint("deletion", "D", color))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ActiveTheme, Config};
    use std::process::Command;
    use tempfile::TempDir;

    fn theme() -> Theme {
        let cfg = Config::load(None).unwrap();
        Theme::from_config(&cfg, ActiveTheme::Dark).unwrap()
    }

    fn run(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git binary required for git tests");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn init_repo(dir: &Path) {
        run(dir, &["init", "-q"]);
        run(dir, &["config", "user.email", "test@example.com"]);
        run(dir, &["config", "user.name", "test"]);
        run(dir, &["config", "commit.gpgsign", "false"]);
    }

    #[test]
    fn render_clean_is_check_mark() {
        let s = render_status(GitFlags::empty(), &theme(), ColorMode::Never);
        assert_eq!(s, "  \u{2713} ");
    }

    #[test]
    fn render_modified_letter_in_4_char_field() {
        let s = render_status(GitFlags::MODIFIED, &theme(), ColorMode::Never);
        assert_eq!(s, "  M ", "got: {s:?}");
    }

    #[test]
    fn render_multiple_flags() {
        let s = render_status(
            GitFlags::NEW | GitFlags::MODIFIED,
            &theme(),
            ColorMode::Never,
        );
        // characters in flag-iteration order: ? then M -> " ?M "
        assert_eq!(s, " ?M ", "got: {s:?}");
    }

    #[test]
    fn discover_outside_repo_returns_none() {
        let tmp = TempDir::new().unwrap();
        let ctx = GitContext::discover(tmp.path()).unwrap();
        assert!(ctx.is_none());
    }

    #[test]
    fn untracked_file_flagged_new() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        std::fs::write(tmp.path().join("hello.txt"), "hi").unwrap();
        let ctx = GitContext::discover(tmp.path()).unwrap().unwrap();
        let path = tmp.path().canonicalize().unwrap().join("hello.txt");
        let flags = ctx.flags_for(&path);
        assert!(flags.contains(GitFlags::NEW), "expected NEW, got {flags:?}");
    }

    #[test]
    fn modified_file_flagged_modified() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        std::fs::write(tmp.path().join("a.txt"), "v1").unwrap();
        run(tmp.path(), &["add", "."]);
        run(tmp.path(), &["commit", "-q", "-m", "init"]);
        std::fs::write(tmp.path().join("a.txt"), "v2").unwrap();
        let ctx = GitContext::discover(tmp.path()).unwrap().unwrap();
        let path = tmp.path().canonicalize().unwrap().join("a.txt");
        let flags = ctx.flags_for(&path);
        assert!(
            flags.contains(GitFlags::MODIFIED),
            "expected MODIFIED, got {flags:?}"
        );
    }

    #[test]
    fn directory_aggregates_descendant_flags() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/inner.txt"), "x").unwrap();
        let ctx = GitContext::discover(tmp.path()).unwrap().unwrap();
        let sub = tmp.path().canonicalize().unwrap().join("sub");
        let flags = ctx.flags_for(&sub);
        assert!(
            flags.contains(GitFlags::NEW),
            "directory should inherit child NEW, got {flags:?}"
        );
    }
}
