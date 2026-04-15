//! Command-line argument parsing.
//!
//! Step-4 surface only: filtering (-a/-A/-d/-f), layout (-1/-x/-C/--format),
//! theming (--color/--dark/--light/--without-icons). Long, sort, git, tree,
//! hyperlink, and indicator flags are added in their own steps.

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};

use crate::config::{ActiveTheme, ColorMode};
use crate::options::{
    Filter, Group, HyperlinkMode, IndicatorStyle, LayoutMode, LongOptions, RunOptions, SortKey,
    SortSpec,
};
use crate::util::report::ReportKind;
use crate::util::time_fmt::TimeStyle;

#[derive(Debug, Parser)]
#[command(
    name = "lsc",
    version,
    about = "Modern, fast Rust ls-color (colorls-compatible)",
    // colorls binds `-h` to --human-readable for ls compatibility. Drop
    // clap's auto `-h`-as-help; `--help` (long form) still works.
    disable_help_flag = true,
)]
pub struct Args {
    /// Print help
    #[arg(long = "help", action = clap::ArgAction::Help)]
    pub _help: Option<bool>,

    /// Paths to list. Defaults to the current directory.
    pub paths: Vec<PathBuf>,

    /// Do not ignore entries starting with `.`
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Do not list `.` and `..`
    #[arg(short = 'A', long = "almost-all")]
    pub almost_all: bool,

    /// Show only directories
    #[arg(short = 'd', long = "dirs")]
    pub dirs: bool,

    /// Show only files
    #[arg(short = 'f', long = "files")]
    pub files: bool,

    /// List one entry per line
    #[arg(short = '1', conflicts_with_all = ["horizontal", "vertical", "format"])]
    pub one_per_line: bool,

    /// List entries by lines (horizontal)
    #[arg(short = 'x', conflicts_with_all = ["one_per_line", "vertical", "format"])]
    pub horizontal: bool,

    /// List entries by columns (vertical)
    #[arg(short = 'C', conflicts_with_all = ["one_per_line", "horizontal", "format"])]
    pub vertical: bool,

    /// Output format
    #[arg(long = "format", value_enum, conflicts_with_all = ["one_per_line", "horizontal", "vertical"])]
    pub format: Option<FormatArg>,

    /// Colorize output
    #[arg(long = "color", value_enum, num_args = 0..=1, default_missing_value = "always")]
    pub color: Option<ColorArg>,

    /// Use the dark color theme (default)
    #[arg(long = "dark", conflicts_with = "light")]
    pub dark: bool,

    /// Use the light color theme
    #[arg(long = "light", conflicts_with = "dark")]
    pub light: bool,

    /// Hide nerd-font icons
    #[arg(long = "without-icons")]
    pub without_icons: bool,

    /// Long listing format with mode/links/user/group/size/time
    #[arg(short = 'l', long = "long", conflicts_with_all = ["one_per_line", "horizontal", "vertical", "format"])]
    pub long: bool,

    /// Long listing without group (alias for --no-group)
    #[arg(short = 'o')]
    pub no_group_o: bool,

    /// Long listing without owner
    #[arg(short = 'g')]
    pub no_owner: bool,

    /// Suppress group column in long listing
    #[arg(short = 'G', long = "no-group")]
    pub no_group: bool,

    /// Hide hard-link count in long listing
    #[arg(long = "no-hardlinks")]
    pub no_hardlinks: bool,

    /// Show file sizes in raw bytes (disables human-readable units)
    #[arg(long = "non-human-readable")]
    pub non_human_readable: bool,

    /// Show inode number
    #[arg(short = 'i', long = "inode")]
    pub inode: bool,

    /// mtime format. `+FORMAT` is a strftime pattern; otherwise asctime.
    #[arg(long = "time-style", value_name = "FORMAT")]
    pub time_style: Option<String>,

    /// Show summary report (`short` or `long`; bare flag = `short`).
    #[arg(long = "report", value_name = "WORD", num_args = 0..=1, default_missing_value = "short")]
    pub report: Option<String>,

    /// Append `/` to directories (shorthand for `--indicator-style=slash`).
    #[arg(short = 'p')]
    pub indicator_p: bool,

    /// Indicator style: `none` or `slash`.
    #[arg(long = "indicator-style", value_name = "STYLE", num_args = 0..=1, default_missing_value = "slash")]
    pub indicator_style: Option<String>,

    /// Always show file sizes in human-readable form (no-op; kept for ls
    /// compatibility, since lsc is human-readable by default).
    #[arg(short = 'h', long = "human-readable", hide = true)]
    pub _human: bool,

    /// Sort by modification time (newest first)
    #[arg(short = 't', conflicts_with_all = ["sort_size", "sort_extension", "sort_none", "sort"])]
    pub sort_time: bool,

    /// Do not sort; use directory order
    #[arg(short = 'U', conflicts_with_all = ["sort_size", "sort_extension", "sort_time", "sort"])]
    pub sort_none: bool,

    /// Sort by file size (largest first)
    #[arg(short = 'S', conflicts_with_all = ["sort_time", "sort_extension", "sort_none", "sort"])]
    pub sort_size: bool,

    /// Sort by file extension
    #[arg(short = 'X', conflicts_with_all = ["sort_time", "sort_size", "sort_none", "sort"])]
    pub sort_extension: bool,

    /// Sort key: name, size, time, extension, none
    #[arg(long = "sort", value_name = "WORD",
        conflicts_with_all = ["sort_time", "sort_size", "sort_extension", "sort_none"])]
    pub sort: Option<String>,

    /// Reverse sort order
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

    /// Group directories before files
    #[arg(
        long = "sd",
        visible_alias = "sort-dirs",
        visible_alias = "group-directories-first",
        conflicts_with = "sort_files"
    )]
    pub sort_dirs: bool,

    /// Group files before directories
    #[arg(
        long = "sf",
        visible_alias = "sort-files",
        conflicts_with = "sort_dirs"
    )]
    pub sort_files: bool,

    /// Show git status for each entry
    #[arg(long = "gs", visible_alias = "git-status")]
    pub git_status: bool,

    /// Show tree view; optional max depth (e.g. --tree=2)
    #[arg(long = "tree", value_name = "DEPTH", num_args = 0..=1, default_missing_value = "0")]
    pub tree: Option<usize>,

    /// Follow symbolic links when listing
    #[arg(short = 'L')]
    pub follow_symlinks: bool,

    /// Emit OSC-8 hyperlinks: always, auto, or never (bare flag = always).
    #[arg(long = "hyperlink", value_name = "WHEN", num_args = 0..=1, default_missing_value = "always")]
    pub hyperlink: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {
    Across,
    Horizontal,
    Long,
    SingleColumn,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorArg {
    Always,
    Auto,
    Never,
}

impl Args {
    /// Parse argv via clap.
    pub fn parse_argv() -> Self {
        Self::parse()
    }

    /// Resolve clap output into the canonical `RunOptions`.
    pub fn resolve(self) -> Result<RunOptions> {
        let layout = resolve_layout(&self)?;
        let theme = if self.light {
            ActiveTheme::Light
        } else {
            ActiveTheme::Dark
        };
        let color_mode = match self.color {
            None | Some(ColorArg::Auto) => ColorMode::Auto,
            Some(ColorArg::Always) => ColorMode::Always,
            Some(ColorArg::Never) => ColorMode::Never,
        };
        let sort = resolve_sort(&self)?;
        let paths = if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths
        };
        let long = LongOptions {
            time_style: self
                .time_style
                .as_deref()
                .map(TimeStyle::parse)
                .unwrap_or_default(),
            // -g hides owner; -o, -G/--no-group hide group.
            show_owner: !self.no_owner,
            show_group: !(self.no_group || self.no_group_o),
            show_hardlinks: !self.no_hardlinks,
            show_inode: self.inode,
            human_readable: !self.non_human_readable,
        };
        let report = match self.report.as_deref() {
            None => None,
            Some("short") => Some(ReportKind::Short),
            Some("long") => Some(ReportKind::Long),
            Some(other) => {
                return Err(anyhow!(
                    "--report: expected `short` or `long`, got `{other}`"
                ))
            }
        };
        // Default style is `slash`. `-p` is a colorls-/ls-compat alias for it
        // and is a no-op when no other indicator style is requested.
        let _ = self.indicator_p;
        let indicator = match self.indicator_style.as_deref() {
            None | Some("slash") => IndicatorStyle::Slash,
            Some("none") => IndicatorStyle::None,
            Some(other) => {
                return Err(anyhow!(
                    "--indicator-style: expected `none` or `slash`, got `{other}`"
                ))
            }
        };
        Ok(RunOptions {
            paths,
            layout,
            filter: Filter {
                all: self.all,
                almost_all: self.almost_all,
                only_dirs: self.dirs,
                only_files: self.files,
            },
            sort,
            show_icons: !self.without_icons,
            theme,
            color_mode,
            long,
            report,
            indicator,
            git_status: self.git_status,
            hyperlink: match self.hyperlink.as_deref() {
                None => HyperlinkMode::Off,
                Some("always") => HyperlinkMode::Always,
                Some("auto") => HyperlinkMode::Auto,
                Some("never") => HyperlinkMode::Never,
                Some(other) => {
                    return Err(anyhow!(
                        "--hyperlink: expected always|auto|never, got `{other}`"
                    ))
                }
            },
        })
    }
}

fn resolve_sort(a: &Args) -> Result<SortSpec> {
    let key = if a.sort_time {
        SortKey::Time
    } else if a.sort_size {
        SortKey::Size
    } else if a.sort_extension {
        SortKey::Extension
    } else if a.sort_none {
        SortKey::None
    } else if let Some(s) = a.sort.as_deref() {
        match s {
            "name" => SortKey::Name,
            "size" => SortKey::Size,
            "time" => SortKey::Time,
            "extension" => SortKey::Extension,
            "none" => SortKey::None,
            other => {
                return Err(anyhow!(
                    "--sort: expected name|size|time|extension|none, got `{other}`"
                ))
            }
        }
    } else {
        SortKey::Name
    };
    let group = if a.sort_dirs {
        Group::DirsFirst
    } else if a.sort_files {
        Group::FilesFirst
    } else {
        Group::Mixed
    };
    Ok(SortSpec {
        key,
        reverse: a.reverse,
        group,
    })
}

fn resolve_layout(a: &Args) -> Result<LayoutMode> {
    if let Some(d) = a.tree {
        // `--tree` (bare) maps to depth=None (unbounded); `--tree=N` to Some(N).
        // We use 0 as the "bare flag" sentinel from default_missing_value.
        let depth = if d == 0 { None } else { Some(d) };
        return Ok(LayoutMode::Tree { depth });
    }
    if a.long {
        return Ok(LayoutMode::Long);
    }
    if a.one_per_line {
        return Ok(LayoutMode::OnePerLine);
    }
    if a.horizontal {
        return Ok(LayoutMode::Horizontal);
    }
    if a.vertical {
        return Ok(LayoutMode::Vertical);
    }
    if let Some(fmt) = a.format {
        return Ok(match fmt {
            FormatArg::Across | FormatArg::Horizontal => LayoutMode::Horizontal,
            FormatArg::Vertical => LayoutMode::Vertical,
            FormatArg::SingleColumn => LayoutMode::OnePerLine,
            FormatArg::Long => LayoutMode::Long,
        });
    }
    Ok(if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        LayoutMode::Vertical
    } else {
        LayoutMode::OnePerLine
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("lsc").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn defaults_resolve_to_auto_color_dark_theme() {
        let opts = parse(&[]).resolve().unwrap();
        assert_eq!(opts.color_mode, ColorMode::Auto);
        assert_eq!(opts.theme, ActiveTheme::Dark);
        assert!(opts.show_icons);
        assert_eq!(opts.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn explicit_paths_passed_through() {
        let opts = parse(&["foo", "bar"]).resolve().unwrap();
        assert_eq!(opts.paths, vec![PathBuf::from("foo"), PathBuf::from("bar")]);
    }

    #[test]
    fn one_per_line_flag() {
        let opts = parse(&["-1"]).resolve().unwrap();
        assert_eq!(opts.layout, LayoutMode::OnePerLine);
    }

    #[test]
    fn horizontal_flag() {
        let opts = parse(&["-x"]).resolve().unwrap();
        assert_eq!(opts.layout, LayoutMode::Horizontal);
    }

    #[test]
    fn vertical_flag() {
        let opts = parse(&["-C"]).resolve().unwrap();
        assert_eq!(opts.layout, LayoutMode::Vertical);
    }

    #[test]
    fn format_across_aliases_horizontal() {
        let opts = parse(&["--format=across"]).resolve().unwrap();
        assert_eq!(opts.layout, LayoutMode::Horizontal);
    }

    #[test]
    fn long_flag_resolves() {
        let opts = parse(&["-l"]).resolve().unwrap();
        assert_eq!(opts.layout, LayoutMode::Long);
    }

    #[test]
    fn format_long_resolves() {
        let opts = parse(&["--format=long"]).resolve().unwrap();
        assert_eq!(opts.layout, LayoutMode::Long);
    }

    #[test]
    fn long_flag_conflicts_with_one_per_line() {
        assert!(Args::try_parse_from(["lsc", "-l", "-1"]).is_err());
    }

    #[test]
    fn no_owner_no_group_flags_propagate() {
        let opts = parse(&["-l", "-g", "-G"]).resolve().unwrap();
        assert!(!opts.long.show_owner);
        assert!(!opts.long.show_group);
    }

    #[test]
    fn inode_flag() {
        let opts = parse(&["-l", "-i"]).resolve().unwrap();
        assert!(opts.long.show_inode);
    }

    #[test]
    fn non_human_readable_flag() {
        let opts = parse(&["-l", "--non-human-readable"]).resolve().unwrap();
        assert!(!opts.long.human_readable);
    }

    #[test]
    fn time_style_plus_prefix() {
        let opts = parse(&["-l", "--time-style=+%F %T"]).resolve().unwrap();
        match opts.long.time_style {
            crate::util::time_fmt::TimeStyle::Custom(s) => assert_eq!(s, "%F %T"),
            _ => panic!("expected Custom"),
        }
    }

    #[test]
    fn report_short_default_when_bare() {
        let opts = parse(&["--report"]).resolve().unwrap();
        assert_eq!(opts.report, Some(ReportKind::Short));
    }

    #[test]
    fn report_long_explicit() {
        let opts = parse(&["--report=long"]).resolve().unwrap();
        assert_eq!(opts.report, Some(ReportKind::Long));
    }

    #[test]
    fn report_invalid_value_errors() {
        assert!(parse(&["--report=garbage"]).resolve().is_err());
    }

    #[test]
    fn indicator_style_none() {
        let opts = parse(&["--indicator-style=none"]).resolve().unwrap();
        assert_eq!(opts.indicator, IndicatorStyle::None);
    }

    #[test]
    fn light_flag_swaps_theme() {
        let opts = parse(&["--light"]).resolve().unwrap();
        assert_eq!(opts.theme, ActiveTheme::Light);
    }

    #[test]
    fn color_never_resolved() {
        let opts = parse(&["--color=never"]).resolve().unwrap();
        assert_eq!(opts.color_mode, ColorMode::Never);
    }

    #[test]
    fn color_bare_defaults_to_always() {
        let opts = parse(&["--color"]).resolve().unwrap();
        assert_eq!(opts.color_mode, ColorMode::Always);
    }

    #[test]
    fn without_icons_flag() {
        let opts = parse(&["--without-icons"]).resolve().unwrap();
        assert!(!opts.show_icons);
    }

    #[test]
    fn dark_and_light_conflict() {
        let res = Args::try_parse_from(["lsc", "--dark", "--light"]);
        assert!(res.is_err());
    }

    #[test]
    fn one_and_horizontal_conflict() {
        let res = Args::try_parse_from(["lsc", "-1", "-x"]);
        assert!(res.is_err());
    }

    #[test]
    fn filter_flags_collected() {
        let opts = parse(&["-a", "-d"]).resolve().unwrap();
        assert!(opts.filter.all);
        assert!(opts.filter.only_dirs);
        assert!(!opts.filter.almost_all);
    }
}
