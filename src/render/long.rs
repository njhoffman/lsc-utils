//! Long-listing renderer.
//!
//! Output shape per row (joined by 3 spaces, mirroring colorls's
//! `line_array.join('   ')`):
//!
//! ```text
//! [inode] mode [hardlinks] [user] [group] size mtime icon name [⇒ target]
//! ```
//!
//! Git status integration lands in step 7 and slots between mtime and icon.

use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;

use crate::config::{ColorMode, IconKind, Icons, Theme};
use crate::fs::{EntryKind, FileEntry};
use crate::options::LongOptions;
use crate::util::{
    human::{self, SizeBucket},
    mode,
    owner::{group_name, user_name},
    report::{ReportCounts, ReportKind},
    time_fmt::{self, AgeBucket, TimeStyle},
};

const MIN_SIZE_CHARS: usize = 4;

pub fn render(
    entries: &[FileEntry],
    theme: &Theme,
    icons: &Icons,
    color_mode: ColorMode,
    show_icons: bool,
    long: &LongOptions,
    out: &mut dyn Write,
) -> io::Result<(ReportCounts, ColumnWidths)> {
    let widths = compute_widths(entries, long);
    let now = SystemTime::now();
    let mut counts = ReportCounts::default();
    for entry in entries {
        let line = build_row(
            entry,
            theme,
            icons,
            color_mode,
            show_icons,
            long,
            &widths,
            now,
            &mut counts,
        );
        out.write_all(line.as_bytes())?;
        out.write_all(b"\n")?;
    }
    Ok((counts, widths))
}

#[derive(Debug, Default, Clone)]
pub struct ColumnWidths {
    pub link: usize,
    pub user: usize,
    pub group: usize,
    pub size_num: usize,
}

fn compute_widths(entries: &[FileEntry], long: &LongOptions) -> ColumnWidths {
    let mut w = ColumnWidths::default();
    let mut max_size: u64 = 0;
    for e in entries {
        let nlink = e.meta.nlink();
        let nlink_digits = if nlink == 0 {
            1
        } else {
            (nlink.ilog10() + 1) as usize
        };
        if nlink_digits > w.link {
            w.link = nlink_digits;
        }
        if long.show_owner {
            let u = user_name(e.meta.uid());
            if u.len() > w.user {
                w.user = u.len();
            }
        }
        if long.show_group {
            let g = group_name(e.meta.gid());
            if g.len() > w.group {
                w.group = g.len();
            }
        }
        if e.meta.size() > max_size {
            max_size = e.meta.size();
        }
    }
    w.size_num = if long.human_readable {
        MIN_SIZE_CHARS
    } else {
        let needed = if max_size == 0 {
            1
        } else {
            (max_size.ilog10() + 1) as usize
        };
        needed.max(MIN_SIZE_CHARS)
    };
    w
}

#[allow(clippy::too_many_arguments)]
fn build_row(
    entry: &FileEntry,
    theme: &Theme,
    icons: &Icons,
    color_mode: ColorMode,
    show_icons: bool,
    long: &LongOptions,
    widths: &ColumnWidths,
    now: SystemTime,
    counts: &mut ReportCounts,
) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(8);

    if long.show_inode {
        let inode = format!("{:>10}", entry.meta.ino());
        parts.push(theme.paint("inode", &inode, color_mode));
    }

    parts.push(format_mode_colored(&entry.meta, theme, color_mode));

    if long.show_hardlinks {
        let nlink = format!("{:>width$}", entry.meta.nlink(), width = widths.link);
        parts.push(theme.paint("normal", &nlink, color_mode));
    }
    if long.show_owner {
        let uname = format!(
            "{:<width$}",
            user_name(entry.meta.uid()),
            width = widths.user
        );
        parts.push(theme.paint("user", &uname, color_mode));
    }
    if long.show_group {
        let gname = format!(
            "{:<width$}",
            group_name(entry.meta.gid()),
            width = widths.group
        );
        parts.push(theme.paint("normal", &gname, color_mode));
    }
    parts.push(format_size_colored(
        entry.meta.size(),
        long.human_readable,
        widths.size_num,
        theme,
        color_mode,
    ));
    parts.push(format_mtime_colored(
        entry.meta.modified().unwrap_or(now),
        now,
        &long.time_style,
        theme,
        color_mode,
    ));

    let mut row = parts.join("   ");
    let (icon_text, name_text, kind) =
        build_icon_and_name(entry, theme, icons, color_mode, show_icons);
    counts.record(kind);
    row.push(' ');
    if !icon_text.is_empty() {
        row.push_str(&icon_text);
        row.push(' ');
    }
    row.push_str(&name_text);
    if let Some(target) = symlink_suffix(entry, theme, color_mode) {
        row.push_str(&target);
    }
    row
}

fn format_mode_colored(meta: &std::fs::Metadata, theme: &Theme, color: ColorMode) -> String {
    let raw = mode::format_mode(meta);
    let mut buf = String::with_capacity(raw.len() * 8);
    for c in raw.chars() {
        let key = mode::color_key_for_char(c);
        buf.push_str(&theme.paint(key, &c.to_string(), color));
    }
    buf
}

fn format_size_colored(
    bytes: u64,
    human: bool,
    width: usize,
    theme: &Theme,
    color: ColorMode,
) -> String {
    let key = match human::bucket(bytes) {
        SizeBucket::Large => "file_large",
        SizeBucket::Medium => "file_medium",
        SizeBucket::Small => "file_small",
    };
    let rendered = if human {
        let (n, u) = human::pretty(bytes);
        format!("{n:>width$} {u:<3}")
    } else {
        format!("{:>width$}", human::raw(bytes))
    };
    theme.paint(key, &rendered, color)
}

fn format_mtime_colored(
    mtime: SystemTime,
    now: SystemTime,
    style: &TimeStyle,
    theme: &Theme,
    color: ColorMode,
) -> String {
    let formatted =
        time_fmt::format_mtime(mtime, style).unwrap_or_else(|_| "invalid mtime           ".into());
    let key = match time_fmt::age_bucket(mtime, now) {
        AgeBucket::HourOld => "hour_old",
        AgeBucket::DayOld => "day_old",
        AgeBucket::Old => "no_modifier",
    };
    theme.paint(key, &formatted, color)
}

fn build_icon_and_name(
    entry: &FileEntry,
    theme: &Theme,
    icons: &Icons,
    color: ColorMode,
    show_icons: bool,
) -> (String, String, IconKind) {
    let name = entry.name_lossy().into_owned();
    let (glyph, color_key, kind) = match entry.kind {
        EntryKind::Directory => {
            let r = icons.for_directory(&name);
            (r.glyph, "dir".to_owned(), r.kind)
        }
        EntryKind::Symlink => {
            let r = icons.for_file(&name);
            let key = if entry.link_target.is_none() {
                "dead_link"
            } else {
                "link"
            };
            (r.glyph, key.to_owned(), r.kind)
        }
        EntryKind::Fifo | EntryKind::Socket => {
            let r = icons.for_file(&name);
            (r.glyph, "socket".to_owned(), r.kind)
        }
        EntryKind::BlockDevice => {
            let r = icons.for_file(&name);
            (r.glyph, "blockdev".to_owned(), r.kind)
        }
        EntryKind::CharDevice => {
            let r = icons.for_file(&name);
            (r.glyph, "chardev".to_owned(), r.kind)
        }
        EntryKind::File => {
            let r = icons.for_file(&name);
            let key = if entry.is_executable() {
                "executable_file"
            } else if r.kind == IconKind::File {
                "recognized_file"
            } else {
                "unrecognized_file"
            };
            (r.glyph, key.to_owned(), r.kind)
        }
    };
    let icon_text = if show_icons {
        theme.paint(&color_key, &glyph.to_string(), color)
    } else {
        String::new()
    };
    let name_text = theme.paint(&color_key, &name, color);
    (icon_text, name_text, kind)
}

fn symlink_suffix(entry: &FileEntry, theme: &Theme, color: ColorMode) -> Option<String> {
    if entry.kind != EntryKind::Symlink {
        return None;
    }
    let target = entry
        .link_target
        .as_deref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "...".to_string());
    let dead = entry
        .link_target
        .as_deref()
        .is_none_or(|p| std::fs::metadata(p).is_err());
    let info = format!(" \u{21d2} {target}");
    Some(if dead {
        theme.paint("dead_link", &format!("{info} [Dead link]"), color)
    } else {
        theme.paint("link", &info, color)
    })
}

pub fn render_report(
    counts: &ReportCounts,
    kind: ReportKind,
    theme: &Theme,
    color: ColorMode,
) -> String {
    theme.paint("report", &counts.render(kind), color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ActiveTheme, Config};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn ctx() -> (Theme, Icons) {
        let cfg = Config::load(None).unwrap();
        (
            Theme::from_config(&cfg, ActiveTheme::Dark).unwrap(),
            Icons::from_config(&cfg).unwrap(),
        )
    }

    fn long_default() -> LongOptions {
        LongOptions {
            time_style: TimeStyle::Asctime,
            show_owner: true,
            show_group: true,
            show_hardlinks: true,
            show_inode: false,
            human_readable: true,
        }
    }

    #[test]
    fn renders_a_row_with_expected_columns() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("script.py");
        std::fs::write(&p, "print('hi')").unwrap();
        let entry = FileEntry::from_path(p).unwrap();
        let (theme, icons) = ctx();

        let mut buf = Vec::new();
        let (counts, _) = render(
            &[entry],
            &theme,
            &icons,
            ColorMode::Never,
            true,
            &long_default(),
            &mut buf,
        )
        .unwrap();
        let s = String::from_utf8(buf).unwrap();

        assert!(s.contains("script.py"), "missing name in: {s:?}");
        assert!(s.contains(" B"), "missing size unit");
        assert!(s.contains("rw"), "missing mode bits");
        assert_eq!(counts.recognized_files, 1);
        assert_eq!(counts.folders, 0);
    }

    #[test]
    fn report_counts_track_by_icon_kind() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.py"), "").unwrap();
        std::fs::write(tmp.path().join("b.qwerty"), "").unwrap();
        std::fs::create_dir(tmp.path().join("d")).unwrap();
        let entries: Vec<_> = ["a.py", "b.qwerty", "d"]
            .iter()
            .map(|n| FileEntry::from_path(tmp.path().join(n)).unwrap())
            .collect();
        let (theme, icons) = ctx();
        let mut buf = Vec::new();
        let (counts, _) = render(
            &entries,
            &theme,
            &icons,
            ColorMode::Never,
            false,
            &long_default(),
            &mut buf,
        )
        .unwrap();
        assert_eq!(counts.folders, 1);
        assert_eq!(counts.recognized_files, 1);
        assert_eq!(counts.unrecognized_files, 1);
    }

    #[test]
    fn symlink_renders_arrow_target() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("real.txt");
        std::fs::write(&target, "x").unwrap();
        let link = tmp.path().join("alias");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let entry = FileEntry::from_path(link).unwrap();
        let (theme, icons) = ctx();
        let mut buf = Vec::new();
        render(
            &[entry],
            &theme,
            &icons,
            ColorMode::Never,
            false,
            &long_default(),
            &mut buf,
        )
        .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\u{21d2}"), "expected ⇒ in: {s:?}");
        assert!(s.contains("real.txt"));
    }

    #[test]
    fn dead_symlink_marked() {
        let tmp = TempDir::new().unwrap();
        let link = tmp.path().join("broken");
        std::os::unix::fs::symlink(PathBuf::from("/no/such/target/zzzz"), &link).unwrap();
        let entry = FileEntry::from_path(link).unwrap();
        let (theme, icons) = ctx();
        let mut buf = Vec::new();
        render(
            &[entry],
            &theme,
            &icons,
            ColorMode::Never,
            false,
            &long_default(),
            &mut buf,
        )
        .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("[Dead link]"),
            "expected dead-link marker in: {s:?}"
        );
    }

    #[test]
    fn non_human_readable_uses_raw_byte_width() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("a");
        std::fs::write(&p, vec![0u8; 1500]).unwrap();
        let entry = FileEntry::from_path(p).unwrap();
        let (theme, icons) = ctx();
        let mut long = long_default();
        long.human_readable = false;
        let mut buf = Vec::new();
        render(
            &[entry],
            &theme,
            &icons,
            ColorMode::Never,
            false,
            &long,
            &mut buf,
        )
        .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1500"), "expected raw byte count: {s:?}");
        assert!(!s.contains("KiB"), "should not include KiB unit");
    }
}
