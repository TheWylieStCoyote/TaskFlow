//! Date parsing for quick add.
//!
//! Supports various date formats including:
//! - Keywords: `today`, `tomorrow`, `yesterday`
//! - Weekdays: `mon`, `monday`, `tue`, etc.
//! - Extended weekdays: `next monday`, `this friday`
//! - Periods: `next week`, `next month`, `next year`
//! - Relative durations: `in 3 days`, `in 2 weeks`, `in 1 month`
//! - End of period: `eow`, `eom`, `eoy`
//! - Ordinal days: `1st`, `15th`, `22nd`, `last day`
//! - ISO format: `YYYY-MM-DD`
//! - Month/Day: `MM/DD`, `MM-DD`

use std::sync::LazyLock;

use chrono::{Datelike, NaiveDate, Utc, Weekday};
use regex::Regex;

// Pre-compiled regex patterns for date parsing
static RELATIVE_DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^in\s+(\d+)\s*(d|day|days|w|week|weeks|m|month|months)$")
        .expect("valid regex pattern")
});
static ORDINAL_DAY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,2})(st|nd|rd|th)$").expect("valid regex pattern"));
// Compact relative shorthand: +3d, +2w, +1m, +1y
static COMPACT_RELATIVE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\+(\d+)(d|w|m|y)$").expect("valid regex pattern"));

/// Parse a date string with various formats.
///
/// Supported formats:
/// - `today`, `tod` - Today's date
/// - `tomorrow`, `tom` - Tomorrow's date
/// - `mon`, `tue`, `wed`, etc. - Next occurrence of that weekday
/// - `monday`, `tuesday`, etc. - Next occurrence of that weekday
/// - `next monday`, `this friday` - Extended weekday expressions
/// - `next week`, `next month` - Start of next period
/// - `in 3 days`, `in 2 weeks`, `in 1 month` - Relative durations
/// - `eow`, `eom`, `end of week` - End of period
/// - `1st`, `15th`, `22nd` - Ordinal day of month
/// - `last day` - Last day of current month
/// - `YYYY-MM-DD` - ISO format
/// - `MM/DD` - Month/Day (current year)
/// - `MM-DD` - Month-Day (current year)
#[must_use]
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let today = Utc::now().date_naive();
    parse_date_with_reference(s, today)
}

/// Parse a date string with a reference date (for testing).
#[must_use]
pub fn parse_date_with_reference(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s_trimmed = s.trim();
    let s_lower = s_trimmed.to_lowercase();

    // 1. Keywords (today, tomorrow)
    match s_lower.as_str() {
        "today" | "tod" => return Some(today),
        "tomorrow" | "tom" => return Some(today + chrono::Duration::days(1)),
        "yesterday" => return Some(today - chrono::Duration::days(1)),
        _ => {}
    }

    // 2. Extended weekday (next monday, this friday)
    if let Some(date) = parse_extended_weekday(s_trimmed, today) {
        return Some(date);
    }

    // 3. Next period (next week, next month)
    if let Some(date) = parse_next_period(s_trimmed, today) {
        return Some(date);
    }

    // 4a. Compact relative shorthand (+3d, +2w, +1m, +1y)
    if let Some(date) = parse_compact_relative(s_trimmed, today) {
        return Some(date);
    }

    // 4b. Relative duration (in 3 days, in 2 weeks)
    if let Some(date) = parse_relative_duration(s_trimmed, today) {
        return Some(date);
    }

    // 5. End of period (eow, eom)
    if let Some(date) = parse_end_of_period(s_trimmed, today) {
        return Some(date);
    }

    // 6. Ordinal day (15th, 1st)
    if let Some(date) = parse_ordinal_day(s_trimmed, today) {
        return Some(date);
    }

    // 7. Plain weekday (monday, tue)
    if let Some(weekday) = parse_weekday(&s_lower) {
        return Some(next_weekday(today, weekday));
    }

    // 8. ISO format (YYYY-MM-DD)
    if let Ok(date) = NaiveDate::parse_from_str(s_trimmed, "%Y-%m-%d") {
        return Some(date);
    }

    // 9. Month/Day formats (MM/DD, MM-DD)
    if let Some(date) = parse_month_day(s_trimmed, '/', today.year()) {
        return Some(date);
    }

    if let Some(date) = parse_month_day(s_trimmed, '-', today.year()) {
        return Some(date);
    }

    None
}

/// Parse a weekday name.
fn parse_weekday(s: &str) -> Option<Weekday> {
    match s {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Get the next occurrence of a weekday from a given date.
pub(crate) fn next_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let current = from.weekday();
    let days_until =
        (i64::from(target.num_days_from_monday()) - i64::from(current.num_days_from_monday()) + 7)
            % 7;

    // If it's the same day, return next week
    let days = if days_until == 0 { 7 } else { days_until };

    from + chrono::Duration::days(days)
}

/// Parse MM/DD or MM-DD format.
fn parse_month_day(s: &str, sep: char, year: i32) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split(sep).collect();
    if parts.len() != 2 {
        return None;
    }

    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;

    NaiveDate::from_ymd_opt(year, month, day)
}

/// Parse relative duration expressions like "in 3 days", "in 2 weeks", "in 1 month".
/// Parse compact relative shorthand: `+3d`, `+2w`, `+1m`, `+1y`.
fn parse_compact_relative(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s_lower = s.to_lowercase();
    let caps = COMPACT_RELATIVE_RE.captures(&s_lower)?;
    let count: i64 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2)?.as_str();
    match unit {
        "d" => Some(today + chrono::Duration::days(count)),
        "w" => Some(today + chrono::Duration::weeks(count)),
        "m" => {
            let new_month = i64::from(today.month()) + count;
            let years_to_add = (new_month - 1) / 12;
            let final_month = ((new_month - 1) % 12 + 1) as u32;
            let final_year = today.year() + years_to_add as i32;
            let max_day = days_in_month(final_year, final_month);
            let final_day = today.day().min(max_day);
            NaiveDate::from_ymd_opt(final_year, final_month, final_day)
        }
        "y" => NaiveDate::from_ymd_opt(today.year() + count as i32, today.month(), today.day())
            .or_else(|| {
                // Handle Feb 29 in non-leap years
                NaiveDate::from_ymd_opt(today.year() + count as i32, today.month() + 1, 1)
                    .map(|d| d - chrono::Duration::days(1))
            }),
        _ => None,
    }
}

fn parse_relative_duration(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s_lower = s.to_lowercase();
    let caps = RELATIVE_DURATION_RE.captures(&s_lower)?;

    let count: i64 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2)?.as_str();

    match unit {
        "d" | "day" | "days" => Some(today + chrono::Duration::days(count)),
        "w" | "week" | "weeks" => Some(today + chrono::Duration::weeks(count)),
        "m" | "month" | "months" => {
            // Add months by advancing the month number
            let new_month = i64::from(today.month()) + count;
            let years_to_add = (new_month - 1) / 12;
            let final_month = ((new_month - 1) % 12 + 1) as u32;
            let final_year = today.year() + years_to_add as i32;

            // Handle day overflow (e.g., Jan 31 + 1 month -> Feb 28)
            let max_day = days_in_month(final_year, final_month);
            let final_day = today.day().min(max_day);

            NaiveDate::from_ymd_opt(final_year, final_month, final_day)
        }
        _ => None,
    }
}

/// Get the number of days in a month.
pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30, // Fallback
    }
}

/// Check if a year is a leap year.
pub(crate) fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Parse end of period expressions like "eow", "eom", "end of week", "end of month".
fn parse_end_of_period(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s_lower = s.to_lowercase();

    match s_lower.as_str() {
        "eow" | "end of week" | "endofweek" => Some(end_of_week(today)),
        "eom" | "end of month" | "endofmonth" => Some(end_of_month(today)),
        "eoy" | "end of year" | "endofyear" => NaiveDate::from_ymd_opt(today.year(), 12, 31),
        _ => None,
    }
}

/// Get the end of the week (Sunday) from a given date.
fn end_of_week(from: NaiveDate) -> NaiveDate {
    // Sunday has num_days_from_monday = 6
    // We want days_until_sunday to be 0 for Sunday
    let current_day = from.weekday().num_days_from_monday();
    let days_until_sunday = if current_day == 6 {
        0 // Already Sunday
    } else {
        i64::from(6 - current_day) // Days until Sunday
    };
    from + chrono::Duration::days(days_until_sunday)
}

/// Get the end of the month (last day) from a given date.
fn end_of_month(from: NaiveDate) -> NaiveDate {
    let last_day = days_in_month(from.year(), from.month());
    NaiveDate::from_ymd_opt(from.year(), from.month(), last_day).unwrap_or(from)
}

/// Parse ordinal day expressions like "1st", "15th", "22nd", "3rd", "last day".
fn parse_ordinal_day(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s_lower = s.to_lowercase();

    // Handle "last day" specially
    if s_lower == "last day" || s_lower == "lastday" {
        return Some(end_of_month(today));
    }

    // Parse ordinal numbers like "1st", "2nd", "3rd", "4th", "15th", "22nd"
    let caps = ORDINAL_DAY_RE.captures(&s_lower)?;

    let day: u32 = caps.get(1)?.as_str().parse().ok()?;

    // Validate day is within month bounds
    if !(1..=31).contains(&day) {
        return None;
    }

    // Try current month first
    let max_day = days_in_month(today.year(), today.month());
    if day > max_day {
        return None;
    }

    // If the day has passed this month, use next month
    if day < today.day() {
        let next_month = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, day)
        } else {
            let next_max = days_in_month(today.year(), today.month() + 1);
            if day <= next_max {
                NaiveDate::from_ymd_opt(today.year(), today.month() + 1, day)
            } else {
                None
            }
        };
        return next_month;
    }

    NaiveDate::from_ymd_opt(today.year(), today.month(), day)
}

/// Parse extended weekday expressions like "next monday", "this friday".
fn parse_extended_weekday(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s_lower = s.to_lowercase();

    // Try "next <weekday>"
    if let Some(rest) = s_lower.strip_prefix("next ") {
        if let Some(weekday) = parse_weekday(rest.trim()) {
            // "next monday" means the monday of next week (7+ days from now)
            // First find days until that weekday
            let target_day = i64::from(weekday.num_days_from_monday());
            let current_day = i64::from(today.weekday().num_days_from_monday());
            let days_until = (target_day - current_day + 7) % 7;
            // Always add 7 to get next week's occurrence
            let days = days_until + 7;
            return Some(today + chrono::Duration::days(days));
        }
    }

    // Try "this <weekday>"
    if let Some(rest) = s_lower.strip_prefix("this ") {
        if let Some(weekday) = parse_weekday(rest.trim()) {
            // "this friday" means this week's friday (past or future)
            let days_diff = i64::from(weekday.num_days_from_monday())
                - i64::from(today.weekday().num_days_from_monday());
            return Some(today + chrono::Duration::days(days_diff));
        }
    }

    None
}

/// Parse next period expressions like "next week", "next month".
fn parse_next_period(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s_lower = s.to_lowercase();

    match s_lower.as_str() {
        "next week" | "nextweek" => {
            // Next week = Monday of next week
            let days_until_monday = (7 - i64::from(today.weekday().num_days_from_monday())) % 7;
            let days = if days_until_monday == 0 {
                7
            } else {
                days_until_monday
            };
            Some(today + chrono::Duration::days(days))
        }
        "next month" | "nextmonth" => {
            // Next month = 1st of next month
            if today.month() == 12 {
                NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
            }
        }
        "next year" | "nextyear" => {
            // Next year = Jan 1 of next year
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
        }
        _ => None,
    }
}
