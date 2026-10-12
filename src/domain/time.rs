//! Time formatting and parsing for the player clock.

/// Formats milliseconds as `HH:MM:SS`, the style the design uses for both the
/// elapsed and total labels. Negative values clamp to zero.
pub fn format_hms(ms: i64) -> String {
    let total = (ms.max(0) / 1000) as u64;
    let (h, m, s) = (total / 3600, (total / 60) % 60, total % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

/// Compact form for lists: `M:SS` under an hour, `H:MM:SS` otherwise.
pub fn format_short(ms: i64) -> String {
    let total = (ms.max(0) / 1000) as u64;
    let (h, m, s) = (total / 3600, (total / 60) % 60, total % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// Signed seconds with three decimals, e.g. `-4.000 s`, as used by delay fields.
pub fn format_delay(us: i64) -> String {
    format!("{:.3} s", us as f64 / 1_000_000.0)
}

/// Parses `SS`, `MM:SS` or `HH:MM:SS` (fractional seconds allowed) into milliseconds.
pub fn parse_clock(text: &str) -> Option<i64> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|p| p.trim().is_empty()) {
        return None;
    }
    let mut ms = 0f64;
    for (i, p) in parts.iter().enumerate() {
        let v: f64 = p.trim().parse().ok()?;
        if v < 0.0 || (i + 1 < parts.len() && v.fract() != 0.0) {
            return None;
        }
        ms = ms * 60.0 + v;
    }
    Some((ms * 1000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hms_matches_design() {
        assert_eq!(format_hms(130_000), "00:02:10");
        assert_eq!(format_hms(1_540_000), "00:25:40");
        assert_eq!(format_hms(-5), "00:00:00");
    }

    #[test]
    fn short_form() {
        assert_eq!(format_short(65_000), "1:05");
        assert_eq!(format_short(3_725_000), "1:02:05");
    }

    #[test]
    fn clock_round_trip() {
        assert_eq!(parse_clock("2:10"), Some(130_000));
        assert_eq!(parse_clock("01:00:01.5"), Some(3_601_500));
        assert_eq!(parse_clock("90"), Some(90_000));
        assert_eq!(parse_clock("a:b"), None);
        assert_eq!(parse_clock(""), None);
    }

    #[test]
    fn delay_format() {
        assert_eq!(format_delay(-4_000_000), "-4.000 s");
    }
}
