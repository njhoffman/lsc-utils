//! Canonical run-time options resolved from CLI flags.
//!
//! `cli::Args` is the raw clap surface; `RunOptions` is what the rest of the
//! crate consumes. This separation keeps clap-isms (Option<String> for tri-
//! state flags, mutually-exclusive groups, etc.) at the boundary.

use std::path::PathBuf;

use crate::config::{ActiveTheme, ColorMode};
use crate::util::report::ReportKind;
use crate::util::time_fmt::TimeStyle;

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub paths: Vec<PathBuf>,
    pub layout: LayoutMode,
    pub filter: Filter,
    pub sort: SortSpec,
    pub show_icons: bool,
    pub theme: ActiveTheme,
    pub color_mode: ColorMode,
    pub long: LongOptions,
    pub report: Option<ReportKind>,
    pub indicator: IndicatorStyle,
    pub git_status: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Time,
    Extension,
    /// `-U`: do not sort; preserve directory order.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Group {
    #[default]
    Mixed,
    DirsFirst,
    FilesFirst,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SortSpec {
    pub key: SortKey,
    pub reverse: bool,
    pub group: Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Multi-column, fills top-to-bottom by column. TTY default.
    Vertical,
    /// Multi-column, fills row-by-row.
    Horizontal,
    /// One entry per line. Default when stdout is not a TTY.
    OnePerLine,
    /// Long listing (`-l` / `--format=long`).
    Long,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Filter {
    /// `-a` / `--all`: show . and ..
    pub all: bool,
    /// `-A` / `--almost-all`: show dotfiles, hide . and ..
    pub almost_all: bool,
    /// `-d` / `--dirs`: only directories
    pub only_dirs: bool,
    /// `-f` / `--files`: only files
    pub only_files: bool,
}

#[derive(Debug, Clone)]
pub struct LongOptions {
    pub time_style: TimeStyle,
    /// `-g` hides owner; `-G`/`--no-group` and `-o` hide group.
    pub show_owner: bool,
    pub show_group: bool,
    pub show_hardlinks: bool,
    pub show_inode: bool,
    pub human_readable: bool,
}

impl Default for LongOptions {
    fn default() -> Self {
        Self {
            time_style: TimeStyle::default(),
            show_owner: true,
            show_group: true,
            show_hardlinks: true,
            show_inode: false,
            human_readable: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndicatorStyle {
    /// No trailing indicator.
    None,
    /// `/` after directories.
    #[default]
    Slash,
}
