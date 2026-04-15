//! POSIX permission-bits formatting.
//!
//! Produces a 9-character `rwxrwxrwx` string (no leading file-type char to
//! match colorls). Setuid/setgid/sticky are folded into the corresponding
//! exec bit per `ls(1)`: `s`/`S` and `t`/`T`.

use std::os::unix::fs::PermissionsExt;

const MASK_SUID: u32 = 0o4000;
const MASK_SGID: u32 = 0o2000;
const MASK_STICKY: u32 = 0o1000;

pub fn format_mode(meta: &std::fs::Metadata) -> String {
    let m = meta.permissions().mode();
    let mut s = String::with_capacity(9);
    s.push_str(&triplet(m >> 6, m & MASK_SUID != 0, 's'));
    s.push_str(&triplet(m >> 3, m & MASK_SGID != 0, 's'));
    s.push_str(&triplet(m, m & MASK_STICKY != 0, 't'));
    s
}

fn triplet(rwx: u32, special: bool, ch: char) -> String {
    let r = if rwx & 4 == 0 { '-' } else { 'r' };
    let w = if rwx & 2 == 0 { '-' } else { 'w' };
    let x = if special {
        if rwx & 1 == 0 {
            ch.to_ascii_uppercase()
        } else {
            ch
        }
    } else if rwx & 1 == 0 {
        '-'
    } else {
        'x'
    };
    let mut t = String::with_capacity(3);
    t.push(r);
    t.push(w);
    t.push(x);
    t
}

/// Map a single mode-string char to a Theme color key.
pub fn color_key_for_char(c: char) -> &'static str {
    match c {
        'r' => "read",
        'w' => "write",
        '-' => "no_access",
        'x' | 's' | 'S' | 't' | 'T' => "exec",
        _ => "normal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_mode(mode: u32) -> (TempDir, std::fs::Metadata) {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("f");
        std::fs::write(&p, "").unwrap();
        let mut perms = std::fs::metadata(&p).unwrap().permissions();
        perms.set_mode(mode);
        std::fs::set_permissions(&p, perms).unwrap();
        let meta = std::fs::metadata(&p).unwrap();
        (tmp, meta)
    }

    #[test]
    fn rwxr_xr_x() {
        let (_t, m) = with_mode(0o755);
        assert_eq!(format_mode(&m), "rwxr-xr-x");
    }

    #[test]
    fn no_perms() {
        let (_t, m) = with_mode(0o000);
        assert_eq!(format_mode(&m), "---------");
    }

    #[test]
    fn setuid_with_x_lowercase_s() {
        let (_t, m) = with_mode(0o4755);
        assert_eq!(format_mode(&m), "rwsr-xr-x");
    }

    #[test]
    fn setuid_without_x_uppercase_s() {
        let (_t, m) = with_mode(0o4644);
        assert_eq!(format_mode(&m), "rwSr--r--");
    }

    #[test]
    fn sticky_with_x_lowercase_t() {
        let (_t, m) = with_mode(0o1777);
        assert_eq!(format_mode(&m), "rwxrwxrwt");
    }

    #[test]
    fn color_key_mapping() {
        assert_eq!(color_key_for_char('r'), "read");
        assert_eq!(color_key_for_char('w'), "write");
        assert_eq!(color_key_for_char('-'), "no_access");
        assert_eq!(color_key_for_char('x'), "exec");
        assert_eq!(color_key_for_char('s'), "exec");
        assert_eq!(color_key_for_char('T'), "exec");
    }
}
