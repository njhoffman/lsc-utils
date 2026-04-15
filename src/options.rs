//! Canonical run-time options resolved from CLI flags.
//!
//! `cli::Args` is the raw clap surface; `RunOptions` is what the rest of the
//! crate consumes. This separation keeps clap-isms (Option<String> for tri-
//! state flags, mutually-exclusive groups, etc.) at the boundary.

use std::path::PathBuf;

use crate::config::{ActiveTheme, ColorMode};

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub paths: Vec<PathBuf>,
    pub layout: LayoutMode,
    pub filter: Filter,
    pub show_icons: bool,
    pub theme: ActiveTheme,
    pub color_mode: ColorMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Multi-column, fills top-to-bottom by column. TTY default.
    Vertical,
    /// Multi-column, fills row-by-row.
    Horizontal,
    /// One entry per line. Default when stdout is not a TTY.
    OnePerLine,
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
