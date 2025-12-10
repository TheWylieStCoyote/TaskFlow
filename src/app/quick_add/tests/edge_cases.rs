//! Edge cases and validation tests.

use chrono::NaiveDate;

use super::super::date::{days_in_month, is_leap_year, parse_date_with_reference};
use super::super::parse_quick_add;

#[test]
fn test_parse_multiple_priorities_uses_first() {
    // When multiple priorities are given, regex captures the first match
    let parsed = parse_quick_add("Task !high !low !urgent");
    // The regex only captures the first priority
    assert_eq!(parsed.priority, Some(crate::domain::Priority::High));
}

#[test]
fn test_parse_multiple_projects_uses_first() {
    // When multiple projects are given, regex captures the first match
    let parsed = parse_quick_add("Task @work @home @office");
    // The regex only captures the first project
    assert_eq!(parsed.project_name, Some("work".to_string()));
}

#[test]
fn test_parse_multiple_due_dates_uses_first() {
    // When multiple due dates are given, regex captures the first match
    let parsed = parse_quick_add("Task due:today due:tomorrow");
    assert_eq!(parsed.due_date, Some(chrono::Utc::now().date_naive()));
}

#[test]
fn test_parse_invalid_iso_date_returns_none() {
    let parsed = parse_quick_add("Task due:2025-13-45");
    // Invalid month/day should return None
    assert!(parsed.due_date.is_none());
}

#[test]
fn test_parse_invalid_month_day_returns_none() {
    let parsed = parse_quick_add("Task due:13/45");
    // Invalid month/day should return None
    assert!(parsed.due_date.is_none());
}

#[test]
fn test_parse_invalid_weekday_returns_none() {
    let parsed = parse_quick_add("Task due:notaday");
    // Invalid weekday should return None
    assert!(parsed.due_date.is_none());
}

#[test]
fn test_parse_tag_with_numbers() {
    let parsed = parse_quick_add("Task #v2 #bug123 #3d");
    assert_eq!(parsed.tags, vec!["v2", "bug123", "3d"]);
}

#[test]
fn test_parse_tag_stops_at_special_chars() {
    // Tags only match word characters (\w+)
    let parsed = parse_quick_add("Task #hello-world");
    // Should only capture "hello", not "hello-world"
    assert_eq!(parsed.tags, vec!["hello"]);
}

#[test]
fn test_parse_whitespace_only_input() {
    let parsed = parse_quick_add("   ");
    assert_eq!(parsed.title, "");
    assert!(parsed.tags.is_empty());
    assert!(parsed.priority.is_none());
}

#[test]
fn test_parse_consecutive_metadata_tokens() {
    let parsed = parse_quick_add("#tag1#tag2 !high!low");
    // The regex should handle consecutive tokens - let's see what actually happens
    // #tag1#tag2 will match as one tag "tag1" (stops at #)
    // Actually \w+ won't match #, so it will get tag1 and tag2 separately
    assert!(parsed.tags.contains(&"tag1".to_string()));
    assert!(parsed.tags.contains(&"tag2".to_string()));
}

#[test]
fn test_parse_date_invalid_input() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(parse_date_with_reference("invalid", reference), None);
    assert_eq!(parse_date_with_reference("blah blah", reference), None);
    assert_eq!(parse_date_with_reference("in days", reference), None);
    assert_eq!(
        parse_date_with_reference("in 0 days", reference),
        Some(reference)
    ); // 0 days = today
}

#[test]
fn test_parse_date_whitespace_handling() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("  today  ", reference),
        Some(reference)
    );
    assert_eq!(
        parse_date_with_reference(" in 3 days ", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 18).unwrap())
    );
}

#[test]
fn test_leap_year_detection() {
    assert!(is_leap_year(2024)); // Divisible by 4
    assert!(!is_leap_year(2025)); // Not divisible by 4
    assert!(!is_leap_year(2100)); // Divisible by 100 but not 400
    assert!(is_leap_year(2000)); // Divisible by 400
}

#[test]
fn test_days_in_month() {
    assert_eq!(days_in_month(2025, 1), 31); // January
    assert_eq!(days_in_month(2025, 2), 28); // February (non-leap)
    assert_eq!(days_in_month(2024, 2), 29); // February (leap)
    assert_eq!(days_in_month(2025, 4), 30); // April
    assert_eq!(days_in_month(2025, 6), 30); // June
    assert_eq!(days_in_month(2025, 7), 31); // July
    assert_eq!(days_in_month(2025, 12), 31); // December
}
