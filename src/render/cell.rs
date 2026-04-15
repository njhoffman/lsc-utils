//! Build pre-rendered cells (icon + name + ANSI styling) from FileEntry.

use crate::config::{ColorMode, IconKind, Icons, Theme};
use crate::fs::{EntryKind, FileEntry};
use crate::render::width::display_width;

/// A renderable cell with its measured display width pre-computed.
#[derive(Debug, Clone)]
pub struct Cell {
    /// ANSI-colored output ready to write to the terminal.
    pub text: String,
    /// Visible width on a Unicode-aware terminal (used for column math).
    pub width: usize,
}

#[derive(Debug)]
pub struct CellBuilder<'a> {
    pub theme: &'a Theme,
    pub icons: &'a Icons,
    pub color_mode: ColorMode,
    pub show_icons: bool,
}

impl<'a> CellBuilder<'a> {
    pub fn build(&self, entry: &FileEntry) -> Cell {
        let name = entry.name_lossy().into_owned();
        let (icon_glyph, color_key) = self.resolve(entry);

        // Text width starts with the visible name. Add 2 (icon glyph display
        // width) + 2 (separator spaces) when icons are on, mirroring colorls's
        // `"#{logo}  #{name}"` formatting.
        let mut visible_width = display_width(&name);
        let mut text = String::new();
        if self.show_icons {
            let icon = icon_glyph.to_string();
            text.push_str(&self.theme.paint(&color_key, &icon, self.color_mode));
            text.push_str("  ");
            visible_width += 2 + 2;
        }
        text.push_str(&self.theme.paint(&color_key, &name, self.color_mode));

        Cell {
            text,
            width: visible_width,
        }
    }

    fn resolve(&self, entry: &FileEntry) -> (char, String) {
        match entry.kind {
            EntryKind::Directory => {
                let r = self.icons.for_directory(&entry.name_lossy());
                // colorls: directory color is always `dir` regardless of which
                // folder key matched.
                (r.glyph, "dir".to_owned())
            }
            EntryKind::Symlink => {
                let r = self.icons.for_file(&entry.name_lossy());
                let key = if entry.link_target.is_none() {
                    "dead_link"
                } else {
                    "link"
                };
                (r.glyph, key.to_owned())
            }
            EntryKind::Fifo | EntryKind::Socket => {
                let r = self.icons.for_file(&entry.name_lossy());
                (r.glyph, "socket".to_owned())
            }
            EntryKind::BlockDevice => {
                let r = self.icons.for_file(&entry.name_lossy());
                (r.glyph, "blockdev".to_owned())
            }
            EntryKind::CharDevice => {
                let r = self.icons.for_file(&entry.name_lossy());
                (r.glyph, "chardev".to_owned())
            }
            EntryKind::File => {
                let r = self.icons.for_file(&entry.name_lossy());
                let color_key = if entry.is_executable() {
                    "executable_file"
                } else if r.kind == IconKind::File {
                    "recognized_file"
                } else {
                    "unrecognized_file"
                };
                (r.glyph, color_key.to_owned())
            }
        }
    }
}

/// Convenience: build cells for many entries.
pub fn build_cells(entries: &[FileEntry], builder: &CellBuilder<'_>) -> Vec<Cell> {
    entries.iter().map(|e| builder.build(e)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ActiveTheme, Config};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make<'a>(
        theme: &'a Theme,
        icons: &'a Icons,
        mode: ColorMode,
        show_icons: bool,
    ) -> CellBuilder<'a> {
        CellBuilder {
            theme,
            icons,
            color_mode: mode,
            show_icons,
        }
    }

    fn fixture(name: &str, dir: bool) -> (TempDir, FileEntry) {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join(name);
        if dir {
            std::fs::create_dir(&p).unwrap();
        } else {
            std::fs::write(&p, "").unwrap();
        }
        let entry = FileEntry::from_path(p).unwrap();
        (tmp, entry)
    }

    fn ctx() -> (Theme, Icons) {
        let cfg = Config::load(None).unwrap();
        (
            Theme::from_config(&cfg, ActiveTheme::Dark).unwrap(),
            Icons::from_config(&cfg).unwrap(),
        )
    }

    #[test]
    fn file_cell_has_icon_when_enabled() {
        let (theme, icons) = ctx();
        let (_t, entry) = fixture("script.py", false);
        let b = make(&theme, &icons, ColorMode::Never, true);
        let cell = b.build(&entry);
        assert!(cell.text.contains("script.py"));
        // Width = 2 (glyph) + 2 (gap) + 9 (script.py)
        assert_eq!(cell.width, 13);
    }

    #[test]
    fn file_cell_without_icon() {
        let (theme, icons) = ctx();
        let (_t, entry) = fixture("script.py", false);
        let b = make(&theme, &icons, ColorMode::Never, false);
        let cell = b.build(&entry);
        assert_eq!(cell.text, "script.py");
        assert_eq!(cell.width, 9);
    }

    #[test]
    fn directory_uses_dir_color() {
        let (theme, icons) = ctx();
        let (_t, entry) = fixture("subdir", true);
        let b = make(&theme, &icons, ColorMode::Always, false);
        let cell = b.build(&entry);
        // SGR with dodgerblue (30,144,255).
        assert!(cell.text.contains("\x1b[38;2;30;144;255m"));
    }

    #[test]
    fn unknown_extension_uses_unrecognized_file_color() {
        let (theme, icons) = ctx();
        let (_t, entry) = fixture("data.qwertyuiop", false);
        let b = make(&theme, &icons, ColorMode::Always, false);
        let cell = b.build(&entry);
        // gold (255,215,0)
        assert!(cell.text.contains("\x1b[38;2;255;215;0m"));
    }

    #[test]
    fn live_symlink_uses_link_color() {
        // readlink succeeds on any symlink whose link content is readable, even
        // if the target doesn't exist — so this test exercises the `link` (not
        // `dead_link`) branch. True dead-link rendering depends on
        // target-following behavior added in step 5.
        let tmp = TempDir::new().unwrap();
        let link = tmp.path().join("broken");
        std::os::unix::fs::symlink(PathBuf::from("/no/such/target"), &link).unwrap();
        let entry = FileEntry::from_path(link).unwrap();
        let (theme, icons) = ctx();
        let b = make(&theme, &icons, ColorMode::Always, false);
        let cell = b.build(&entry);
        // cyan (0,255,255) is `link`
        assert!(cell.text.contains("\x1b[38;2;0;255;255m"));
    }
}
