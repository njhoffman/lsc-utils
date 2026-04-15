//! Grid layouts: vertical (top-to-bottom) and horizontal (left-to-right).
//!
//! Algorithm ported from colorls's `lib/colorls/layout.rb`. We binary-search
//! for the largest column count whose summed per-column widths fit within
//! `screen_width`. Column gap is two spaces (matches colorls's `"  "`).

use std::io::{self, Write};

use super::cell::Cell;

const COLUMN_GAP: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridKind {
    Vertical,
    Horizontal,
}

pub fn render_grid(
    cells: &[Cell],
    screen_width: usize,
    kind: GridKind,
    out: &mut dyn Write,
) -> io::Result<()> {
    if cells.is_empty() {
        return Ok(());
    }
    let widths: Vec<usize> = cells.iter().map(|c| c.width + COLUMN_GAP).collect();
    let (chunk_size, col_widths) = fit_columns(&widths, screen_width.max(1), kind);

    match kind {
        GridKind::Horizontal => write_horizontal(cells, chunk_size, &col_widths, out),
        GridKind::Vertical => write_vertical(cells, chunk_size, &col_widths, out),
    }
}

fn write_horizontal(
    cells: &[Cell],
    cols: usize,
    col_widths: &[usize],
    out: &mut dyn Write,
) -> io::Result<()> {
    if cols == 0 {
        return Ok(());
    }
    for row in cells.chunks(cols) {
        let mut line = String::new();
        for (i, cell) in row.iter().enumerate() {
            line.push_str(&cell.text);
            if i + 1 < row.len() {
                let pad = col_widths[i].saturating_sub(cell.width);
                line.extend(std::iter::repeat_n(' ', pad));
            }
        }
        line.push('\n');
        out.write_all(line.as_bytes())?;
    }
    Ok(())
}

fn write_vertical(
    cells: &[Cell],
    rows: usize,
    col_widths: &[usize],
    out: &mut dyn Write,
) -> io::Result<()> {
    if rows == 0 {
        return Ok(());
    }
    let cols = col_widths.len();
    for r in 0..rows {
        let mut line = String::new();
        for (c, col_w) in col_widths.iter().enumerate().take(cols) {
            let idx = c * rows + r;
            let Some(cell) = cells.get(idx) else { continue };
            line.push_str(&cell.text);
            if c + 1 < cols && (c + 1) * rows + r < cells.len() {
                let pad = col_w.saturating_sub(cell.width);
                line.extend(std::iter::repeat_n(' ', pad));
            }
        }
        line.push('\n');
        out.write_all(line.as_bytes())?;
    }
    Ok(())
}

/// Binary search for the column configuration that fills `screen_width`
/// without overflowing. Returns `(chunk_size, per_column_max_widths)` where
/// `chunk_size` is "items per row" for horizontal and "items per column"
/// (i.e. row count) for vertical layouts.
fn fit_columns(widths: &[usize], screen_width: usize, kind: GridKind) -> (usize, Vec<usize>) {
    let n = widths.len();
    if n == 0 {
        return (0, vec![]);
    }
    let min_w = *widths.iter().min().unwrap_or(&1);
    let mut max_chunks = (screen_width / min_w.max(1)).max(1).min(n);
    let mut min_chunks = 1usize;
    let mut last = column_widths(widths, max_chunks, kind);
    loop {
        let mid = (min_chunks + max_chunks).div_ceil(2);
        let candidate = column_widths(widths, mid, kind);
        let fits = candidate.1.iter().sum::<usize>() <= screen_width;
        if min_chunks < max_chunks && !fits {
            max_chunks = mid - 1;
        } else if min_chunks < mid {
            min_chunks = mid;
            last = candidate;
        } else {
            return last;
        }
    }
}

fn column_widths(widths: &[usize], mid: usize, kind: GridKind) -> (usize, Vec<usize>) {
    match kind {
        GridKind::Horizontal => column_widths_horizontal(widths, mid),
        GridKind::Vertical => column_widths_vertical(widths, mid),
    }
}

/// Horizontal: `mid` items per row, max-of-column across rows.
fn column_widths_horizontal(widths: &[usize], mid: usize) -> (usize, Vec<usize>) {
    let mid = mid.max(1);
    let mut cols = vec![0usize; mid];
    for chunk in widths.chunks(mid) {
        for (i, w) in chunk.iter().enumerate() {
            if *w > cols[i] {
                cols[i] = *w;
            }
        }
    }
    (mid, cols)
}

/// Vertical: `mid` candidate columns -> compute rows per column, then take the
/// max width within each per-column slice.
fn column_widths_vertical(widths: &[usize], mid: usize) -> (usize, Vec<usize>) {
    let mid = mid.max(1);
    let rows = widths.len().div_ceil(mid);
    let cols: Vec<usize> = widths
        .chunks(rows)
        .map(|c| *c.iter().max().unwrap_or(&0))
        .collect();
    (rows, cols)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(names: &[&str]) -> Vec<Cell> {
        names
            .iter()
            .map(|n| Cell {
                text: (*n).to_owned(),
                width: n.chars().count(),
            })
            .collect()
    }

    fn render(c: &[Cell], width: usize, kind: GridKind) -> String {
        let mut buf = Vec::new();
        render_grid(c, width, kind, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn horizontal_three_per_row_in_30_cols() {
        let c = cells(&["aa", "bb", "cc", "dd", "ee"]);
        let out = render(&c, 30, GridKind::Horizontal);
        // Each cell width=2, +2 gap. Three per row easily fits.
        let lines: Vec<&str> = out.trim_end().split('\n').collect();
        assert!(!lines.is_empty());
        // First line should contain at least aa, bb, cc.
        assert!(lines[0].contains("aa"));
        assert!(lines[0].contains("cc"));
    }

    #[test]
    fn vertical_top_to_bottom_ordering() {
        let c = cells(&["a", "b", "c", "d", "e", "f"]);
        let out = render(&c, 8, GridKind::Vertical);
        // 8 cols / (1+2 per cell) -> 2 columns -> 3 rows. Order must be
        // column-major: row0 = a, d ; row1 = b, e ; row2 = c, f.
        let lines: Vec<&str> = out.trim_end().split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with('a') && lines[0].contains('d'));
        assert!(lines[1].starts_with('b') && lines[1].contains('e'));
        assert!(lines[2].starts_with('c') && lines[2].contains('f'));
    }

    #[test]
    fn empty_input_emits_nothing() {
        assert_eq!(render(&[], 80, GridKind::Vertical), "");
        assert_eq!(render(&[], 80, GridKind::Horizontal), "");
    }

    #[test]
    fn single_cell_one_line() {
        let c = cells(&["only"]);
        let out = render(&c, 80, GridKind::Vertical);
        assert_eq!(out, "only\n");
    }

    #[test]
    fn narrow_width_falls_back_to_one_column() {
        let c = cells(&["alpha", "beta", "gamma"]);
        // 3 cols width, smaller than any cell: still produce one item per row.
        let out = render(&c, 3, GridKind::Vertical);
        let lines: Vec<&str> = out.trim_end().split('\n').collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn horizontal_pads_short_cells_to_column_width() {
        let c = cells(&["a", "bbb", "c", "d"]);
        let out = render(&c, 20, GridKind::Horizontal);
        // First row should pad `a` so `bbb` aligns predictably.
        let first = out.lines().next().unwrap();
        let a_pos = first.find('a').unwrap();
        let b_pos = first.find('b').unwrap();
        // b should be at least 3 chars after a (cell width 1 + gap 2).
        assert!(b_pos - a_pos >= 3);
    }
}
