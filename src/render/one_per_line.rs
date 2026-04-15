//! One entry per line. Default for non-TTY stdout and `-1`.

use std::io::{self, Write};

use super::cell::Cell;

pub fn render(cells: &[Cell], out: &mut dyn Write) -> io::Result<()> {
    for cell in cells {
        out.write_all(cell.text.as_bytes())?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_one_per_line() {
        let cells = vec![
            Cell {
                text: "a".into(),
                width: 1,
            },
            Cell {
                text: "b".into(),
                width: 1,
            },
        ];
        let mut buf = Vec::new();
        render(&cells, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "a\nb\n");
    }

    #[test]
    fn empty_input_emits_nothing() {
        let mut buf = Vec::new();
        render(&[], &mut buf).unwrap();
        assert!(buf.is_empty());
    }
}
