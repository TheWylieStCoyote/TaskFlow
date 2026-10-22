//! Human-readable duration formatting shared by every surface that shows
//! tracked time (task list, dashboard, reports, CLI, exports).
//!
//! Keeping one formatter means a minute count renders the same everywhere:
//! `0m`, `45m`, `2h`, `1h 30m`.

/// Format a minute count as `Xh Ym`, dropping a zero component (`2h`, `45m`).
#[must_use]
pub fn format_minutes(minutes: u32) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// Format a minute count given as a signed integer; negative values clamp to `0m`.
#[must_use]
pub fn format_minutes_i64(minutes: i64) -> String {
    format_minutes(u32::try_from(minutes.max(0)).unwrap_or(u32::MAX))
}

/// Format a second count as a live clock `HH:MM:SS` (hours are not capped).
#[must_use]
pub fn format_hms(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

/// Format minutes as decimal hours with one decimal place (`1.5h`), for charts.
#[must_use]
pub fn format_hours_decimal(minutes: u32) -> String {
    format!("{:.1}h", f64::from(minutes) / 60.0)
}

/// Parse a human duration (`90`, `90m`, `1h`, `1.5h`, `1h30m`, `1h 30m`) into minutes.
///
/// A bare number is read as minutes. Returns `None` for empty or malformed
/// input. Case-insensitive; surrounding whitespace is ignored.
#[must_use]
pub fn parse_minutes(input: &str) -> Option<u32> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return None;
    }

    // Plain number: minutes
    if let Ok(mins) = input.parse::<u32>() {
        return Some(mins);
    }

    // Decimal or whole hours: "1.5h", "2h"
    if let Some(hours_str) = input.strip_suffix('h') {
        if let Ok(hours) = hours_str.trim().parse::<f64>() {
            if hours.is_finite() && hours >= 0.0 {
                return Some((hours * 60.0).round() as u32);
            }
        }
    }

    // Minutes only: "90m"
    if let Some(mins_str) = input.strip_suffix('m') {
        if !mins_str.contains('h') {
            if let Ok(mins) = mins_str.trim().parse::<u32>() {
                return Some(mins);
            }
        }
    }

    // Hours and minutes: "1h30m", "1h 30m", "1h30"
    let input = input.replace(' ', "");
    let (hours_str, rest) = input.split_once('h')?;
    let hours = hours_str.parse::<u32>().ok()?;
    let mins = if rest.is_empty() {
        0
    } else {
        rest.strip_suffix('m').unwrap_or(rest).parse::<u32>().ok()?
    };
    Some(hours * 60 + mins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_drop_zero_components() {
        assert_eq!(format_minutes(0), "0m");
        assert_eq!(format_minutes(45), "45m");
        assert_eq!(format_minutes(60), "1h");
        assert_eq!(format_minutes(90), "1h 30m");
        assert_eq!(format_minutes(1441), "24h 1m");
    }

    #[test]
    fn signed_minutes_clamp_at_zero() {
        assert_eq!(format_minutes_i64(-5), "0m");
        assert_eq!(format_minutes_i64(125), "2h 5m");
    }

    #[test]
    fn hms_is_zero_padded() {
        assert_eq!(format_hms(0), "00:00:00");
        assert_eq!(format_hms(59), "00:00:59");
        assert_eq!(format_hms(3661), "01:01:01");
        assert_eq!(format_hms(100 * 3600), "100:00:00");
    }

    #[test]
    fn decimal_hours() {
        assert_eq!(format_hours_decimal(90), "1.5h");
        assert_eq!(format_hours_decimal(0), "0.0h");
    }

    #[test]
    fn parse_minutes_accepts_every_spelling() {
        assert_eq!(parse_minutes("30"), Some(30));
        assert_eq!(parse_minutes("90m"), Some(90));
        assert_eq!(parse_minutes("2h"), Some(120));
        assert_eq!(parse_minutes("1.5h"), Some(90));
        assert_eq!(parse_minutes("2.25h"), Some(135));
        assert_eq!(parse_minutes("1h30m"), Some(90));
        assert_eq!(parse_minutes("1h 30m"), Some(90));
        assert_eq!(parse_minutes("1h30"), Some(90));
        assert_eq!(parse_minutes(" 2H 15M "), Some(135));
        assert_eq!(parse_minutes("0"), Some(0));
    }

    #[test]
    fn parse_minutes_rejects_garbage() {
        assert_eq!(parse_minutes(""), None);
        assert_eq!(parse_minutes("   "), None);
        assert_eq!(parse_minutes("abc"), None);
        assert_eq!(parse_minutes("h"), None);
        assert_eq!(parse_minutes("m"), None);
        assert_eq!(parse_minutes("-1h"), None);
        assert_eq!(parse_minutes("1hxm"), None);
        assert_eq!(parse_minutes("1h30x"), None);
    }

    #[test]
    fn parse_minutes_roundtrips_format_minutes() {
        for mins in [0, 5, 45, 60, 90, 135, 600] {
            assert_eq!(parse_minutes(&format_minutes(mins)), Some(mins));
        }
    }
}
