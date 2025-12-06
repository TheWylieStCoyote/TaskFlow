//! Quick add parsing for task creation.
//!
//! Parses task titles for embedded metadata using a simple syntax:
//!
//! - `#tag` - Add a tag
//! - `!priority` - Set priority (urgent/high/med/medium/low)
//! - `due:date` - Set due date
//! - `sched:date` - Set scheduled date
//! - `@project` - Assign to project
//!
//! # Example
//!
//! ```
//! use taskflow::app::quick_add::parse_quick_add;
//!
//! let parsed = parse_quick_add("Fix bug #backend !high due:tomorrow");
//! assert_eq!(parsed.title, "Fix bug");
//! assert_eq!(parsed.tags, vec!["backend"]);
//! assert!(parsed.priority.is_some());
//! assert!(parsed.due_date.is_some());
//! ```

use chrono::{Datelike, NaiveDate, Utc, Weekday};
use regex::Regex;

use crate::domain::Priority;

/// Result of parsing a quick add string
#[derive(Debug, Clone, Default)]
pub struct ParsedTask {
    /// The cleaned task title (metadata tokens removed)
    pub title: String,
    /// Tags extracted from #tag syntax
    pub tags: Vec<String>,
    /// Priority extracted from !priority syntax
    pub priority: Option<Priority>,
    /// Due date extracted from due:date syntax
    pub due_date: Option<NaiveDate>,
    /// Scheduled date extracted from sched:date syntax
    pub scheduled_date: Option<NaiveDate>,
    /// Project name extracted from @project syntax
    pub project_name: Option<String>,
}

/// Parse a quick add string to extract metadata
///
/// # Arguments
///
/// * `input` - The raw task input string
///
/// # Returns
///
/// A `ParsedTask` containing the cleaned title and extracted metadata
///
/// # Example
///
/// ```
/// use taskflow::app::quick_add::parse_quick_add;
///
/// let parsed = parse_quick_add("Buy groceries #shopping !med due:saturday @home");
/// assert_eq!(parsed.title, "Buy groceries");
/// assert_eq!(parsed.tags, vec!["shopping"]);
/// assert_eq!(parsed.project_name, Some("home".to_string()));
/// ```
pub fn parse_quick_add(input: &str) -> ParsedTask {
    let mut result = ParsedTask::default();
    let mut remaining = input.to_string();

    // Extract tags (#tag)
    let tag_re = Regex::new(r"#(\w+)").unwrap();
    for cap in tag_re.captures_iter(input) {
        result.tags.push(cap[1].to_string());
    }
    remaining = tag_re.replace_all(&remaining, "").to_string();

    // Extract priority (!priority)
    let priority_re = Regex::new(r"!(\w+)").unwrap();
    if let Some(cap) = priority_re.captures(input) {
        result.priority = parse_priority(&cap[1]);
    }
    remaining = priority_re.replace_all(&remaining, "").to_string();

    // Extract due date (due:date)
    let due_re = Regex::new(r"due:(\S+)").unwrap();
    if let Some(cap) = due_re.captures(input) {
        result.due_date = parse_date(&cap[1]);
    }
    remaining = due_re.replace_all(&remaining, "").to_string();

    // Extract scheduled date (sched:date)
    let sched_re = Regex::new(r"sched:(\S+)").unwrap();
    if let Some(cap) = sched_re.captures(input) {
        result.scheduled_date = parse_date(&cap[1]);
    }
    remaining = sched_re.replace_all(&remaining, "").to_string();

    // Extract project (@project)
    let project_re = Regex::new(r"@(\w+)").unwrap();
    if let Some(cap) = project_re.captures(input) {
        result.project_name = Some(cap[1].to_string());
    }
    remaining = project_re.replace_all(&remaining, "").to_string();

    // Clean up title: collapse multiple spaces and trim
    result.title = remaining.split_whitespace().collect::<Vec<_>>().join(" ");

    result
}

/// Parse a priority string
fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "urgent" | "u" | "!!!!" => Some(Priority::Urgent),
        "high" | "h" | "!!!" => Some(Priority::High),
        "med" | "medium" | "m" | "!!" => Some(Priority::Medium),
        "low" | "l" | "!" => Some(Priority::Low),
        "none" | "n" | "0" => Some(Priority::None),
        _ => None,
    }
}

/// Parse a date string with various formats
///
/// Supported formats:
/// - `today`, `tod` - Today's date
/// - `tomorrow`, `tom` - Tomorrow's date
/// - `mon`, `tue`, `wed`, etc. - Next occurrence of that weekday
/// - `monday`, `tuesday`, etc. - Next occurrence of that weekday
/// - `YYYY-MM-DD` - ISO format
/// - `MM/DD` - Month/Day (current year)
/// - `MM-DD` - Month-Day (current year)
fn parse_date(s: &str) -> Option<NaiveDate> {
    let today = Utc::now().date_naive();
    let s_lower = s.to_lowercase();

    match s_lower.as_str() {
        "today" | "tod" => Some(today),
        "tomorrow" | "tom" => Some(today + chrono::Duration::days(1)),
        _ => {
            // Try weekday names
            if let Some(weekday) = parse_weekday(&s_lower) {
                return Some(next_weekday(today, weekday));
            }

            // Try ISO format (YYYY-MM-DD)
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Some(date);
            }

            // Try MM/DD format
            if let Some(date) = parse_month_day(s, '/', today.year()) {
                return Some(date);
            }

            // Try MM-DD format
            if let Some(date) = parse_month_day(s, '-', today.year()) {
                return Some(date);
            }

            None
        }
    }
}

/// Parse a weekday name
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

/// Get the next occurrence of a weekday from a given date
fn next_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let current = from.weekday();
    let days_until = (target.num_days_from_monday() as i64
        - current.num_days_from_monday() as i64
        + 7)
        % 7;

    // If it's the same day, return next week
    let days = if days_until == 0 { 7 } else { days_until };

    from + chrono::Duration::days(days)
}

/// Parse MM/DD or MM-DD format
fn parse_month_day(s: &str, sep: char, year: i32) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split(sep).collect();
    if parts.len() != 2 {
        return None;
    }

    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;

    NaiveDate::from_ymd_opt(year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quick_add_simple_title() {
        let parsed = parse_quick_add("Buy groceries");
        assert_eq!(parsed.title, "Buy groceries");
        assert!(parsed.tags.is_empty());
        assert!(parsed.priority.is_none());
        assert!(parsed.due_date.is_none());
        assert!(parsed.project_name.is_none());
    }

    #[test]
    fn test_parse_quick_add_with_tag() {
        let parsed = parse_quick_add("Fix bug #backend");
        assert_eq!(parsed.title, "Fix bug");
        assert_eq!(parsed.tags, vec!["backend"]);
    }

    #[test]
    fn test_parse_quick_add_multiple_tags() {
        let parsed = parse_quick_add("Fix bug #backend #urgent #v2");
        assert_eq!(parsed.title, "Fix bug");
        assert_eq!(parsed.tags, vec!["backend", "urgent", "v2"]);
    }

    #[test]
    fn test_parse_quick_add_priority_high() {
        let parsed = parse_quick_add("Important task !high");
        assert_eq!(parsed.title, "Important task");
        assert_eq!(parsed.priority, Some(Priority::High));
    }

    #[test]
    fn test_parse_quick_add_priority_urgent() {
        let parsed = parse_quick_add("Critical issue !urgent");
        assert_eq!(parsed.priority, Some(Priority::Urgent));
    }

    #[test]
    fn test_parse_quick_add_priority_medium() {
        let parsed = parse_quick_add("Normal task !med");
        assert_eq!(parsed.priority, Some(Priority::Medium));
    }

    #[test]
    fn test_parse_quick_add_priority_low() {
        let parsed = parse_quick_add("Low priority task !low");
        assert_eq!(parsed.priority, Some(Priority::Low));
    }

    #[test]
    fn test_parse_quick_add_due_today() {
        let parsed = parse_quick_add("Task due:today");
        assert_eq!(parsed.title, "Task");
        assert_eq!(parsed.due_date, Some(Utc::now().date_naive()));
    }

    #[test]
    fn test_parse_quick_add_due_tomorrow() {
        let parsed = parse_quick_add("Task due:tomorrow");
        let expected = Utc::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(parsed.due_date, Some(expected));
    }

    #[test]
    fn test_parse_quick_add_due_iso_format() {
        let parsed = parse_quick_add("Task due:2025-12-25");
        assert_eq!(
            parsed.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_quick_add_due_month_day() {
        let parsed = parse_quick_add("Task due:12/25");
        let year = Utc::now().date_naive().year();
        assert_eq!(
            parsed.due_date,
            Some(NaiveDate::from_ymd_opt(year, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_quick_add_scheduled() {
        let parsed = parse_quick_add("Task sched:tomorrow");
        let expected = Utc::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(parsed.scheduled_date, Some(expected));
    }

    #[test]
    fn test_parse_quick_add_project() {
        let parsed = parse_quick_add("Task @work");
        assert_eq!(parsed.title, "Task");
        assert_eq!(parsed.project_name, Some("work".to_string()));
    }

    #[test]
    fn test_parse_quick_add_complex() {
        let parsed = parse_quick_add("Fix login bug #backend #auth !high due:friday @work");
        assert_eq!(parsed.title, "Fix login bug");
        assert_eq!(parsed.tags, vec!["backend", "auth"]);
        assert_eq!(parsed.priority, Some(Priority::High));
        assert!(parsed.due_date.is_some());
        assert_eq!(parsed.project_name, Some("work".to_string()));
    }

    #[test]
    fn test_parse_quick_add_empty() {
        let parsed = parse_quick_add("");
        assert_eq!(parsed.title, "");
        assert!(parsed.tags.is_empty());
    }

    #[test]
    fn test_parse_quick_add_only_metadata() {
        let parsed = parse_quick_add("#tag !high");
        assert_eq!(parsed.title, "");
        assert_eq!(parsed.tags, vec!["tag"]);
        assert_eq!(parsed.priority, Some(Priority::High));
    }

    #[test]
    fn test_parse_weekday_monday() {
        let today = Utc::now().date_naive();
        let next_monday = next_weekday(today, Weekday::Mon);
        assert_eq!(next_monday.weekday(), Weekday::Mon);
        assert!(next_monday > today || next_monday == today + chrono::Duration::days(7));
    }

    #[test]
    fn test_parse_quick_add_weekday() {
        let parsed = parse_quick_add("Meeting due:monday");
        assert!(parsed.due_date.is_some());
        if let Some(date) = parsed.due_date {
            assert_eq!(date.weekday(), Weekday::Mon);
        }
    }

    #[test]
    fn test_parse_priority_aliases() {
        assert_eq!(parse_priority("u"), Some(Priority::Urgent));
        assert_eq!(parse_priority("h"), Some(Priority::High));
        assert_eq!(parse_priority("m"), Some(Priority::Medium));
        assert_eq!(parse_priority("l"), Some(Priority::Low));
        assert_eq!(parse_priority("n"), Some(Priority::None));
    }

    #[test]
    fn test_parse_quick_add_preserves_title_words() {
        let parsed = parse_quick_add("This is a long task title with many words");
        assert_eq!(parsed.title, "This is a long task title with many words");
    }

    #[test]
    fn test_parse_quick_add_metadata_in_middle() {
        let parsed = parse_quick_add("Fix #bug in the code !high today");
        assert_eq!(parsed.title, "Fix in the code today");
        assert_eq!(parsed.tags, vec!["bug"]);
        assert_eq!(parsed.priority, Some(Priority::High));
    }
}
