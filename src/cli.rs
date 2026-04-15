//! Command-line argument parsing.
//!
//! Step-4 surface only: filtering (-a/-A/-d/-f), layout (-1/-x/-C/--format),
//! theming (--color/--dark/--light/--without-icons). Long, sort, git, tree,
//! hyperlink, and indicator flags are added in their own steps.

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};

use crate::config::{ActiveTheme, ColorMode};
use crate::options::{Filter, LayoutMode, RunOptions};

#[derive(Debug, Parser)]
#[command(
    name = "lsc",
    version,
    about = "Modern, fast Rust ls-color (colorls-compatible)",
    disable_help_flag = false,
    disable_version_flag = false
)]
pub struct Args {
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
        let paths = if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths
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
            show_icons: !self.without_icons,
            theme,
            color_mode,
        })
    }
}

fn resolve_layout(a: &Args) -> Result<LayoutMode> {
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
            // Long lands in step 5; reject explicitly until then so the user
            // gets a clear error instead of a silent fallback.
            FormatArg::Long => {
                return Err(anyhow!(
                    "--format=long / -l: long mode not yet implemented (step 5)"
                ))
            }
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
    fn format_long_errors_until_step_5() {
        let err = parse(&["--format=long"]).resolve().unwrap_err();
        assert!(err.to_string().contains("step 5"));
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
