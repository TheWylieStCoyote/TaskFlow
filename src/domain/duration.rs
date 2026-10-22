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
}
