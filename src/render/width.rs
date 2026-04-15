//! Display-width measurement.
//!
//! `unicode-width` (UAX #11) reports the canonical East-Asian width but
//! Private Use Area glyphs (where Nerd Font ships its icons) are reported as
//! width 1 even though every Nerd Font renders them as width 2. We bump PUA
//! codepoints to width 2 so column math matches what the user sees.

use unicode_width::UnicodeWidthChar;

/// Sum of display widths of all chars in `s`. Control chars count as 0.
pub fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

pub fn char_display_width(c: char) -> usize {
    if is_nerd_font_pua(c) {
        return 2;
    }
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// True for codepoints inside the three Unicode Private Use Areas. Nerd Font
/// glyphs live almost exclusively in BMP PUA (U+E000..=U+F8FF) plus a small
/// set in Plane 15/16 supplementary PUAs.
fn is_nerd_font_pua(c: char) -> bool {
    let cp = c as u32;
    (0xE000..=0xF8FF).contains(&cp)
        || (0xF0000..=0xFFFFD).contains(&cp)
        || (0x100000..=0x10FFFD).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_widths() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn cjk_double_width() {
        assert_eq!(display_width("漢字"), 4);
    }

    #[test]
    fn pua_glyph_counted_as_two() {
        // U+E74E (js icon from files.yaml) is in BMP PUA.
        assert_eq!(char_display_width('\u{e74e}'), 2);
        // Outside PUA, BMP char stays width 1.
        assert_eq!(char_display_width('a'), 1);
    }

    #[test]
    fn control_chars_zero_width() {
        assert_eq!(display_width("\x07\x08"), 0);
    }

    #[test]
    fn supplementary_pua_double_width() {
        // U+F0000 is start of Supplementary PUA-A.
        assert_eq!(char_display_width('\u{F0000}'), 2);
    }
}
