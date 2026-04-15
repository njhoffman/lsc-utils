//! OSC-8 terminal hyperlink wrapping.
//!
//! Format: `\x1b]8;;URI\x1b\\TEXT\x1b]8;;\x1b\\`
//! Supported by modern terminal emulators (kitty, iTerm2, WezTerm, recent
//! Gnome Terminal, etc.). Disabled when stdout isn't a TTY or `TERM=dumb`,
//! mirroring `ls --hyperlink=auto`.

use std::io::IsTerminal;
use std::path::Path;

use crate::options::HyperlinkMode;

pub fn wrap(text: &str, path: &Path, mode: HyperlinkMode) -> String {
    if !enabled(mode) {
        return text.to_owned();
    }
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let uri = path_to_file_uri(&abs);
    format!("\x1b]8;;{uri}\x1b\\{text}\x1b]8;;\x1b\\")
}

fn enabled(mode: HyperlinkMode) -> bool {
    match mode {
        HyperlinkMode::Off | HyperlinkMode::Never => false,
        HyperlinkMode::Always => true,
        HyperlinkMode::Auto => {
            if std::env::var_os("TERM").is_some_and(|v| v == "dumb") {
                return false;
            }
            std::io::stdout().is_terminal()
        }
    }
}

/// Best-effort file URI: percent-encodes characters outside a small safe set.
/// We don't need RFC 3986 perfection; consumers (terminal emulators) are
/// permissive here.
fn path_to_file_uri(p: &Path) -> String {
    let s = p.to_string_lossy();
    let mut out = String::with_capacity(s.len() + 8);
    out.push_str("file://");
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn off_returns_text_unchanged() {
        let s = wrap("hello", Path::new("/tmp/x"), HyperlinkMode::Off);
        assert_eq!(s, "hello");
    }

    #[test]
    fn never_returns_text_unchanged() {
        let s = wrap("hello", Path::new("/tmp/x"), HyperlinkMode::Never);
        assert_eq!(s, "hello");
    }

    #[test]
    fn always_wraps_in_osc8() {
        let s = wrap("hello", Path::new("/tmp/x"), HyperlinkMode::Always);
        assert!(s.starts_with("\x1b]8;;file:///"));
        assert!(s.ends_with("hello\x1b]8;;\x1b\\"));
    }

    #[test]
    fn percent_encodes_spaces_and_specials() {
        let uri = path_to_file_uri(&PathBuf::from("/tmp/has space/x@y"));
        assert!(uri.contains("%20"));
        assert!(uri.contains("%40"));
    }

    #[test]
    fn ascii_alnum_path_unencoded() {
        let uri = path_to_file_uri(&PathBuf::from("/usr/local/bin"));
        assert_eq!(uri, "file:///usr/local/bin");
    }
}
