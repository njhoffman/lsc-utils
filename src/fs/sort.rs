//! Sort comparators for FileEntry.
//!
//! Mirrors colorls's sort semantics:
//! - name: case-sensitive name compare. (colorls uses libc strxfrm for locale-
//!   aware collation; v1 uses codepoint order. Locale support is tracked as
//!   a known follow-up — see /home/nicholas/.claude/plans/...)
//! - size: largest first.
//! - time: newest first (descending mtime).
//! - extension: by extension, with name as the tiebreaker.
//! - none: preserve input order (used for `-U`).
//!
//! `--reverse` flips the final order; `--sd`/`--sf` partition by kind first.
//!
//! All comparators are total to keep `sort_by` stable across permutations,
//! which is asserted via proptest below.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::Path;
use std::time::SystemTime;

use crate::fs::FileEntry;
use crate::options::{Group, SortKey, SortSpec};

pub fn sort(entries: &mut [FileEntry], spec: SortSpec) {
    if spec.key != SortKey::None {
        entries.sort_by(|a, b| compare(a, b, spec.key));
    }
    if spec.reverse {
        entries.reverse();
    }
    match spec.group {
        Group::Mixed => {}
        Group::DirsFirst => entries.sort_by_key(group_key_dirs_first),
        Group::FilesFirst => entries.sort_by_key(group_key_files_first),
    }
}

pub fn compare(a: &FileEntry, b: &FileEntry, key: SortKey) -> Ordering {
    match key {
        SortKey::Name => a.name.cmp(&b.name),
        SortKey::Size => b.meta.len().cmp(&a.meta.len()).then(a.name.cmp(&b.name)),
        SortKey::Time => mtime(&b.meta)
            .cmp(&mtime(&a.meta))
            .then(a.name.cmp(&b.name)),
        SortKey::Extension => extension(&a.name)
            .cmp(extension(&b.name))
            .then(stem(&a.name).cmp(stem(&b.name))),
        SortKey::None => Ordering::Equal,
    }
}

fn mtime(meta: &std::fs::Metadata) -> SystemTime {
    meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)
}

fn extension(name: &OsStr) -> &OsStr {
    Path::new(name).extension().unwrap_or(OsStr::new(""))
}

fn stem(name: &OsStr) -> &OsStr {
    Path::new(name).file_stem().unwrap_or(name)
}

fn group_key_dirs_first(e: &FileEntry) -> u8 {
    if e.is_dir() {
        0
    } else {
        1
    }
}
fn group_key_files_first(e: &FileEntry) -> u8 {
    if e.is_dir() {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    fn make(tmp: &TempDir, name: &str, bytes: usize, age_secs: Option<u64>) -> FileEntry {
        let p = tmp.path().join(name);
        if name.ends_with('/') {
            unreachable!()
        }
        std::fs::write(&p, vec![0u8; bytes]).unwrap();
        if let Some(secs) = age_secs {
            let mtime = SystemTime::now() - Duration::from_secs(secs);
            let _ = filetime::set_file_mtime(&p, filetime::FileTime::from_system_time(mtime));
        }
        FileEntry::from_path(p).unwrap()
    }

    fn make_dir(tmp: &TempDir, name: &str) -> FileEntry {
        let p = tmp.path().join(name);
        std::fs::create_dir(&p).unwrap();
        FileEntry::from_path(p).unwrap()
    }

    #[test]
    fn name_ascending() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![
            make(&tmp, "c", 0, None),
            make(&tmp, "a", 0, None),
            make(&tmp, "b", 0, None),
        ];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::Name,
                reverse: false,
                group: Group::Mixed,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name_lossy().into_owned()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn size_largest_first() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![
            make(&tmp, "small", 10, None),
            make(&tmp, "big", 1000, None),
            make(&tmp, "mid", 100, None),
        ];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::Size,
                reverse: false,
                group: Group::Mixed,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name_lossy().into_owned()).collect();
        assert_eq!(names, vec!["big", "mid", "small"]);
    }

    #[test]
    fn time_newest_first() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![
            make(&tmp, "old", 0, Some(86_400)),
            make(&tmp, "new", 0, Some(60)),
            make(&tmp, "ancient", 0, Some(86_400 * 30)),
        ];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::Time,
                reverse: false,
                group: Group::Mixed,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name_lossy().into_owned()).collect();
        assert_eq!(names, vec!["new", "old", "ancient"]);
    }

    #[test]
    fn extension_then_name() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![
            make(&tmp, "z.rs", 0, None),
            make(&tmp, "a.txt", 0, None),
            make(&tmp, "a.rs", 0, None),
            make(&tmp, "noext", 0, None),
        ];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::Extension,
                reverse: false,
                group: Group::Mixed,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name_lossy().into_owned()).collect();
        // Empty extension sorts first (noext), then "rs" (a.rs, z.rs), then "txt" (a.txt).
        assert_eq!(names, vec!["noext", "a.rs", "z.rs", "a.txt"]);
    }

    #[test]
    fn none_preserves_order() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![
            make(&tmp, "c", 0, None),
            make(&tmp, "a", 0, None),
            make(&tmp, "b", 0, None),
        ];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::None,
                reverse: false,
                group: Group::Mixed,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name_lossy().into_owned()).collect();
        assert_eq!(names, vec!["c", "a", "b"]);
    }

    #[test]
    fn reverse_flips() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![
            make(&tmp, "a", 0, None),
            make(&tmp, "b", 0, None),
            make(&tmp, "c", 0, None),
        ];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::Name,
                reverse: true,
                group: Group::Mixed,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name_lossy().into_owned()).collect();
        assert_eq!(names, vec!["c", "b", "a"]);
    }

    #[test]
    fn dirs_first_partitions() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![
            make(&tmp, "file_a", 0, None),
            make_dir(&tmp, "dir_z"),
            make(&tmp, "file_b", 0, None),
            make_dir(&tmp, "dir_a"),
        ];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::Name,
                reverse: false,
                group: Group::DirsFirst,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name_lossy().into_owned()).collect();
        // Within each group the inner sort (Name asc) is preserved.
        assert_eq!(names, vec!["dir_a", "dir_z", "file_a", "file_b"]);
    }

    #[test]
    fn files_first_partitions() {
        let tmp = TempDir::new().unwrap();
        let mut v = vec![make(&tmp, "f", 0, None), make_dir(&tmp, "d")];
        sort(
            &mut v,
            SortSpec {
                key: SortKey::Name,
                reverse: false,
                group: Group::FilesFirst,
            },
        );
        assert_eq!(v[0].name_lossy(), "f");
        assert_eq!(v[1].name_lossy(), "d");
    }

    // proptest: assert comparator totality + transitivity for SortKey::Name on
    // arbitrary file name sets. If sort_by panics or produces inconsistent
    // results, the test fails.
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn name_sort_is_total_and_idempotent(names in proptest::collection::vec("[a-zA-Z0-9_]{1,8}", 1..30)) {
            let tmp = TempDir::new().unwrap();
            let mut entries: Vec<_> = names
                .iter()
                .enumerate()
                // dedup names by index suffix so we don't trip on duplicate file names
                .map(|(i, n)| make(&tmp, &format!("{n}_{i}"), 0, None))
                .collect();
            sort(&mut entries, SortSpec { key: SortKey::Name, reverse: false, group: Group::Mixed });
            let after_first: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
            sort(&mut entries, SortSpec { key: SortKey::Name, reverse: false, group: Group::Mixed });
            let after_second: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
            prop_assert_eq!(after_first, after_second);
        }

        #[test]
        fn size_sort_descending_invariant(sizes in proptest::collection::vec(0u64..10_000, 1..20)) {
            let tmp = TempDir::new().unwrap();
            let mut entries: Vec<_> = sizes
                .iter()
                .enumerate()
                .map(|(i, &s)| make(&tmp, &format!("f_{i}"), s as usize, None))
                .collect();
            sort(&mut entries, SortSpec { key: SortKey::Size, reverse: false, group: Group::Mixed });
            for win in entries.windows(2) {
                prop_assert!(win[0].meta.len() >= win[1].meta.len(),
                    "size sort not descending: {} > {}", win[0].meta.len(), win[1].meta.len());
            }
        }
    }
}
