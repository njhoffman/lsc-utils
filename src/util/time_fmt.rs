//! `--time-style` formatting for mtime.
//!
//! - Default (`TimeStyle::Asctime`): `Wed Apr 15 02:34:56 2026` (24 chars,
//!   matches Ruby's `Time#asctime`).
//! - `TimeStyle::Custom(fmt)` (set via `--time-style=+FORMAT`): `fmt` is a
//!   strtime pattern passed straight to jiff.

use std::time::SystemTime;

use anyhow::{anyhow, Result};
use jiff::{tz::TimeZone, Timestamp};

#[derive(Debug, Clone, Default)]
pub enum TimeStyle {
    #[default]
    Asctime,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeBucket {
    /// < 1 hour ago
    HourOld,
    /// < 24 hours ago
    DayOld,
    /// older
    Old,
}

impl TimeStyle {
    /// Parse a `--time-style=...` argument. colorls treats values starting with
    /// `+` as strftime patterns; everything else maps to asctime.
    pub fn parse(arg: &str) -> Self {
        if let Some(fmt) = arg.strip_prefix('+') {
            TimeStyle::Custom(fmt.to_owned())
        } else {
            TimeStyle::Asctime
        }
    }
}

pub fn format_mtime(mtime: SystemTime, style: &TimeStyle) -> Result<String> {
    let ts = system_time_to_jiff(mtime)?;
    let zoned = ts.to_zoned(TimeZone::system());
    let fmt = match style {
        TimeStyle::Asctime => "%a %b %e %H:%M:%S %Y",
        TimeStyle::Custom(s) => s.as_str(),
    };
    Ok(zoned.strftime(fmt).to_string())
}

pub fn age_bucket(mtime: SystemTime, now: SystemTime) -> AgeBucket {
    let delta = now
        .duration_since(mtime)
        .unwrap_or(std::time::Duration::ZERO);
    if delta.as_secs() < 60 * 60 {
        AgeBucket::HourOld
    } else if delta.as_secs() < 24 * 60 * 60 {
        AgeBucket::DayOld
    } else {
        AgeBucket::Old
    }
}

fn system_time_to_jiff(st: SystemTime) -> Result<Timestamp> {
    let dur = st
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| anyhow!("mtime before unix epoch: {e}"))?;
    Timestamp::new(dur.as_secs() as i64, dur.subsec_nanos() as i32)
        .map_err(|e| anyhow!("invalid timestamp: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn parse_plus_prefix_custom() {
        match TimeStyle::parse("+%F %T") {
            TimeStyle::Custom(s) => assert_eq!(s, "%F %T"),
            _ => panic!("expected Custom"),
        }
    }

    #[test]
    fn parse_no_prefix_asctime() {
        assert!(matches!(TimeStyle::parse("anything"), TimeStyle::Asctime));
        assert!(matches!(TimeStyle::parse(""), TimeStyle::Asctime));
    }

    #[test]
    fn asctime_shape_for_unix_epoch() {
        // Format result depends on local TZ; just check structural shape.
        let s = format_mtime(SystemTime::UNIX_EPOCH, &TimeStyle::Asctime).unwrap();
        assert_eq!(s.len(), 24, "expected 24 char asctime, got {s:?}");
        // `Day Mon dd HH:MM:SS YYYY` -> 4 spaces.
        assert_eq!(s.matches(' ').count(), 4);
    }

    #[test]
    fn custom_format_passes_through_to_jiff() {
        let s = format_mtime(
            SystemTime::UNIX_EPOCH + Duration::from_secs(0),
            &TimeStyle::Custom("%Y".into()),
        )
        .unwrap();
        // Year of unix epoch in any TZ near UTC is 1969 or 1970.
        assert!(s == "1969" || s == "1970", "got: {s}");
    }

    #[test]
    fn age_buckets() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(86_400 * 365);
        assert_eq!(age_bucket(now, now), AgeBucket::HourOld);
        assert_eq!(
            age_bucket(now - Duration::from_secs(30 * 60), now),
            AgeBucket::HourOld
        );
        assert_eq!(
            age_bucket(now - Duration::from_secs(2 * 60 * 60), now),
            AgeBucket::DayOld
        );
        assert_eq!(
            age_bucket(now - Duration::from_secs(48 * 60 * 60), now),
            AgeBucket::Old
        );
    }
}
