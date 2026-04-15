//! Human-readable file sizes (matches the Ruby `filesize` gem's `pretty`
//! output: KiB/MiB/GiB binary suffixes with two decimals).

const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeBucket {
    Small,
    Medium,
    Large,
}

pub fn bucket(bytes: u64) -> SizeBucket {
    if bytes >= 512 * 1024 * 1024 {
        SizeBucket::Large
    } else if bytes >= 128 * 1024 * 1024 {
        SizeBucket::Medium
    } else {
        SizeBucket::Small
    }
}

/// "1.23 KiB" style. Returns (number, unit) for column alignment.
pub fn pretty(bytes: u64) -> (String, &'static str) {
    if bytes < 1024 {
        return (bytes.to_string(), "B");
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    (format!("{value:.2}"), UNITS[unit])
}

/// Raw bytes as a number string (used with `--non-human-readable`).
pub fn raw(bytes: u64) -> String {
    bytes.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_under_1024() {
        let (n, u) = pretty(1023);
        assert_eq!((n.as_str(), u), ("1023", "B"));
    }

    #[test]
    fn kib_with_two_decimals() {
        let (n, u) = pretty(1536);
        assert_eq!(u, "KiB");
        assert_eq!(n, "1.50");
    }

    #[test]
    fn mib_threshold() {
        let (_, u) = pretty(2 * 1024 * 1024);
        assert_eq!(u, "MiB");
    }

    #[test]
    fn buckets() {
        assert_eq!(bucket(1024), SizeBucket::Small);
        assert_eq!(bucket(128 * 1024 * 1024), SizeBucket::Medium);
        assert_eq!(bucket(512 * 1024 * 1024), SizeBucket::Large);
        assert_eq!(bucket(1024 * 1024 * 1024), SizeBucket::Large);
    }

    #[test]
    fn raw_bytes() {
        assert_eq!(raw(0), "0");
        assert_eq!(raw(123456789), "123456789");
    }
}
