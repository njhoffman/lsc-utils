//! Tree-view renderer.
//!
//! Mirrors colorls's `tree_traverse`/`tree_branch_preprint`:
//! - indent = 2 chars per nesting level.
//! - connectors: ` ├──` for non-last, ` └──` for the last item AND for
//!   directories (the "directories use └──" quirk is in colorls; we keep
//!   it for parity).
//! - prefix per ancestor depth: ` │ ` (3 chars) repeated `prespace/indent`
//!   times, followed by the connector, then `─` * indent.
//!
//! Cycle guard: track visited (dev, inode) tuples to avoid infinite loops
//! through symlink cycles or hard-linked directories. Hard depth ceiling
//! at 256 levels to bound stack depth on pathological inputs.

use std::collections::HashSet;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use anyhow::Result;

use crate::config::{ColorMode, Theme};
use crate::fs::{scan_directory, sort};
use crate::options::{Filter, SortSpec};
use crate::render::cell::CellBuilder;

const INDENT: usize = 2;
const HARD_DEPTH_CEILING: usize = 256;

#[allow(clippy::too_many_arguments)]
pub fn render(
    root: &Path,
    depth_limit: Option<usize>,
    filter: &Filter,
    sort_spec: SortSpec,
    builder: &CellBuilder<'_>,
    theme: &Theme,
    color_mode: ColorMode,
    out: &mut dyn Write,
) -> Result<()> {
    let mut visited: HashSet<(u64, u64)> = HashSet::new();
    traverse(
        root,
        0,
        1,
        depth_limit,
        filter,
        sort_spec,
        builder,
        theme,
        color_mode,
        &mut visited,
        out,
    )
}

#[allow(clippy::too_many_arguments)]
fn traverse(
    path: &Path,
    prespace: usize,
    depth: usize,
    depth_limit: Option<usize>,
    filter: &Filter,
    sort_spec: SortSpec,
    builder: &CellBuilder<'_>,
    theme: &Theme,
    color_mode: ColorMode,
    visited: &mut HashSet<(u64, u64)>,
    out: &mut dyn Write,
) -> Result<()> {
    if depth > HARD_DEPTH_CEILING {
        return Ok(());
    }
    let mut entries = scan_directory(path, filter)?;
    sort::sort(&mut entries, sort_spec);
    let last_idx = entries.len().saturating_sub(1);
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == last_idx;
        // Mirror colorls quirk: directories also get └── regardless of position.
        let connector = if is_last || entry.is_dir() {
            " \u{2514}\u{2500}\u{2500}"
        } else {
            " \u{251c}\u{2500}\u{2500}"
        };
        let prefix_text = preprint(prespace, connector);
        let prefix_painted = theme.paint("tree", &prefix_text, color_mode);
        let cell = builder.build(entry);
        out.write_all(prefix_painted.as_bytes())?;
        out.write_all(b" ")?;
        out.write_all(cell.text.as_bytes())?;
        out.write_all(b"\n")?;

        if !entry.is_dir() {
            continue;
        }
        if !keep_going(depth, depth_limit) {
            continue;
        }
        // Cycle guard: skip if we've descended into this inode before.
        let key = (entry.meta.dev(), entry.meta.ino());
        if !visited.insert(key) {
            continue;
        }
        traverse(
            &entry.path,
            prespace + INDENT,
            depth + 1,
            depth_limit,
            filter,
            sort_spec,
            builder,
            theme,
            color_mode,
            visited,
            out,
        )?;
    }
    Ok(())
}

fn keep_going(depth: usize, depth_limit: Option<usize>) -> bool {
    match depth_limit {
        None => true,
        Some(n) => depth < n,
    }
}

fn preprint(prespace: usize, connector: &str) -> String {
    if prespace == 0 {
        return connector.to_owned();
    }
    let bars = " \u{2502} ".repeat(prespace / INDENT);
    let mut s = bars;
    s.push_str(connector);
    s.push_str(&"\u{2500}".repeat(INDENT));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ActiveTheme, Config, Icons};
    use tempfile::TempDir;

    fn ctx() -> (Theme, Icons) {
        let cfg = Config::load(None).unwrap();
        (
            Theme::from_config(&cfg, ActiveTheme::Dark).unwrap(),
            Icons::from_config(&cfg).unwrap(),
        )
    }

    fn render_to_string(root: &Path, depth: Option<usize>, theme: &Theme, icons: &Icons) -> String {
        let builder = CellBuilder {
            theme,
            icons,
            color_mode: ColorMode::Never,
            show_icons: false,
            indicator: crate::options::IndicatorStyle::None,
            hyperlink: crate::options::HyperlinkMode::Off,
        };
        let mut buf = Vec::new();
        render(
            root,
            depth,
            &Filter::default(),
            SortSpec::default(),
            &builder,
            theme,
            ColorMode::Never,
            &mut buf,
        )
        .unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn empty_dir_emits_nothing() {
        let tmp = TempDir::new().unwrap();
        let (theme, icons) = ctx();
        let s = render_to_string(tmp.path(), None, &theme, &icons);
        assert!(s.is_empty(), "expected empty, got: {s:?}");
    }

    #[test]
    fn flat_dir_uses_branch_connectors() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a"), "").unwrap();
        std::fs::write(tmp.path().join("b"), "").unwrap();
        std::fs::write(tmp.path().join("c"), "").unwrap();
        let (theme, icons) = ctx();
        let s = render_to_string(tmp.path(), None, &theme, &icons);
        let lines: Vec<&str> = s.lines().collect();
        // Three files, sorted alphabetically. First two non-last get ├──,
        // last gets └──.
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("\u{251c}\u{2500}\u{2500}"));
        assert!(lines[1].contains("\u{251c}\u{2500}\u{2500}"));
        assert!(lines[2].contains("\u{2514}\u{2500}\u{2500}"));
        assert!(lines[0].contains(" a"));
        assert!(lines[2].contains(" c"));
    }

    #[test]
    fn nested_dir_descends_with_pipe_prefix() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("d")).unwrap();
        std::fs::write(tmp.path().join("d/inner"), "").unwrap();
        let (theme, icons) = ctx();
        let s = render_to_string(tmp.path(), None, &theme, &icons);
        // First line: " └── d" (directories use └──)
        // Second line: " │  └── inner"
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(" d"), "line0={:?}", lines[0]);
        assert!(
            lines[1].contains("\u{2502}"),
            "expected │, line1={:?}",
            lines[1]
        );
        assert!(lines[1].contains(" inner"), "line1={:?}", lines[1]);
    }

    #[test]
    fn depth_limit_truncates() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        std::fs::write(tmp.path().join("a/b/c/leaf"), "").unwrap();
        let (theme, icons) = ctx();
        let s = render_to_string(tmp.path(), Some(2), &theme, &icons);
        // depth=1 visits `a`; depth=2 visits `b`; depth=3 (would visit c) is blocked.
        // So we see a and b but not c or leaf.
        assert!(s.contains(" a"));
        assert!(s.contains(" b"));
        assert!(!s.contains(" c"), "depth limit failed: {s:?}");
        assert!(!s.contains(" leaf"));
    }

    #[test]
    fn depth_zero_treated_as_unbounded_via_caller() {
        // The CLI converts bare `--tree` to depth=None; depth=Some(0) here is
        // explicit "no descents" — we still print top-level entries.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("d")).unwrap();
        std::fs::write(tmp.path().join("d/inner"), "").unwrap();
        let (theme, icons) = ctx();
        let s = render_to_string(tmp.path(), Some(0), &theme, &icons);
        // depth=Some(0) -> depth(1) < 0 is false -> no recursion.
        assert!(s.contains(" d"));
        assert!(!s.contains(" inner"));
    }
}
