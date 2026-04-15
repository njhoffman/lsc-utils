//! Color theme: maps semantic keys (e.g. "dir", "recognized_file") to ANSI
//! styles. Values come from `dark_colors.yaml` / `light_colors.yaml`. Colorls
//! accepts CSS named colors (`dodgerblue`) or hex (`#fafafa`); both are parsed
//! to RGB and emitted as 24-bit ANSI SGR.

use std::collections::HashMap;
use std::io::IsTerminal;

use anyhow::{anyhow, Context, Result};
use csscolorparser::Color as CssColor;

use super::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTheme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Always,
    Auto,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Rgb,
}

#[derive(Debug, Clone)]
pub struct Theme {
    active: ActiveTheme,
    dark: HashMap<String, Style>,
    light: HashMap<String, Style>,
}

impl Theme {
    pub fn from_config(cfg: &Config, active: ActiveTheme) -> Result<Self> {
        Ok(Self {
            active,
            dark: parse_palette(&cfg.dark_colors, "dark_colors.yaml")?,
            light: parse_palette(&cfg.light_colors, "light_colors.yaml")?,
        })
    }

    pub fn active(&self) -> ActiveTheme {
        self.active
    }

    /// Returns the style for `key`, or a neutral white fallback. Missing keys
    /// indicate a corrupted theme; we trace at debug level rather than panic.
    pub fn style(&self, key: &str) -> Style {
        let palette = match self.active {
            ActiveTheme::Dark => &self.dark,
            ActiveTheme::Light => &self.light,
        };
        palette.get(key).copied().unwrap_or_else(|| {
            tracing::debug!(missing_key = key, "theme key missing; using fallback");
            Style {
                fg: Rgb(255, 255, 255),
            }
        })
    }

    /// Render `text` wrapped in SGR for `key` if `mode` permits.
    pub fn paint(&self, key: &str, text: &str, mode: ColorMode) -> String {
        if !mode.enabled() {
            return text.to_owned();
        }
        let s = self.style(key);
        format!("\x1b[38;2;{};{};{}m{}\x1b[0m", s.fg.0, s.fg.1, s.fg.2, text)
    }
}

impl ColorMode {
    /// Resolve --color=auto against env + tty; explicit always/never bypass.
    pub fn enabled(self) -> bool {
        match self {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => Self::auto_enabled(),
        }
    }

    fn auto_enabled() -> bool {
        // NO_COLOR (https://no-color.org) wins when set to any non-empty value.
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return false;
        }
        // CLICOLOR_FORCE forces on regardless of TTY (https://bixense.com/clicolors/).
        if std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v != "0") {
            return true;
        }
        // Without a TTY, default off.
        if !std::io::stdout().is_terminal() {
            return false;
        }
        // CLICOLOR=0 disables even on a TTY.
        if std::env::var_os("CLICOLOR").is_some_and(|v| v == "0") {
            return false;
        }
        // TERM=dumb disables.
        if std::env::var_os("TERM").is_some_and(|v| v == "dumb") {
            return false;
        }
        true
    }
}

fn parse_palette(map: &super::YamlMap, source: &str) -> Result<HashMap<String, Style>> {
    let mut out = HashMap::with_capacity(map.len());
    for (k, v) in map {
        let rgb = parse_color(v)
            .with_context(|| format!("{source}: invalid color value for `{k}`: `{v}`"))?;
        out.insert(k.clone(), Style { fg: rgb });
    }
    Ok(out)
}

/// Colorls's YAMLs use a few X11 color names that aren't in the CSS Level 4
/// named-color set (which `csscolorparser` follows). Map them to their CSS
/// equivalents before parsing so user-edited YAMLs from colorls drop in.
fn alias_color_name(value: &str) -> &str {
    match value {
        "navyblue" => "navy", // X11 NavyBlue == Navy (#000080)
        other => other,
    }
}

fn parse_color(value: &str) -> Result<Rgb> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("empty color value"));
    }
    let normalized = alias_color_name(trimmed);
    let parsed: CssColor = normalized
        .parse()
        .map_err(|e| anyhow!("css parse failed: {e}"))?;
    let [r, g, b, _a] = parsed.to_rgba8();
    Ok(Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme {
        let cfg = Config::load(None).unwrap();
        Theme::from_config(&cfg, ActiveTheme::Dark).unwrap()
    }

    #[test]
    fn parse_named_css_color() {
        assert_eq!(parse_color("dodgerblue").unwrap(), Rgb(30, 144, 255));
        assert_eq!(parse_color("gold").unwrap(), Rgb(255, 215, 0));
    }

    #[test]
    fn parse_hex_color() {
        assert_eq!(parse_color("#fafafa").unwrap(), Rgb(0xfa, 0xfa, 0xfa));
        assert_eq!(parse_color("#000").unwrap(), Rgb(0, 0, 0));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_color("not-a-color").is_err());
        assert!(parse_color("").is_err());
    }

    #[test]
    fn parse_navyblue_x11_alias() {
        // colorls's light_colors.yaml uses `navyblue`; CSS only knows `navy`.
        assert_eq!(parse_color("navyblue").unwrap(), Rgb(0, 0, 128));
    }

    #[test]
    fn light_palette_loads_with_all_aliases_resolved() {
        let cfg = Config::load(None).unwrap();
        let t = Theme::from_config(&cfg, ActiveTheme::Light).expect("light palette parses");
        // `dir` is `navyblue` in light theme; aliased to navy.
        assert_eq!(t.style("dir").fg, Rgb(0, 0, 128));
    }

    #[test]
    fn theme_loads_dark_with_canonical_styles() {
        let t = theme();
        assert_eq!(t.style("dir").fg, Rgb(30, 144, 255)); // dodgerblue
        assert_eq!(t.style("recognized_file").fg, Rgb(255, 255, 0)); // yellow
    }

    #[test]
    fn missing_key_returns_fallback() {
        let t = theme();
        assert_eq!(t.style("totally_not_a_key").fg, Rgb(255, 255, 255));
    }

    #[test]
    fn paint_emits_sgr_when_enabled() {
        let t = theme();
        let painted = t.paint("dir", "x", ColorMode::Always);
        assert_eq!(painted, "\x1b[38;2;30;144;255mx\x1b[0m");
    }

    #[test]
    fn paint_passthrough_when_never() {
        let t = theme();
        assert_eq!(t.paint("dir", "x", ColorMode::Never), "x");
    }

    #[test]
    fn no_color_env_disables_auto() {
        let saved = std::env::var_os("NO_COLOR");
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(!ColorMode::Auto.enabled());
        unsafe {
            match saved {
                Some(v) => std::env::set_var("NO_COLOR", v),
                None => std::env::remove_var("NO_COLOR"),
            }
        }
    }
}
