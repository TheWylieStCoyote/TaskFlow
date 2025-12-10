//! Date format parsing tests.

use chrono::{Datelike, NaiveDate, Utc, Weekday};

use super::super::date::{next_weekday, parse_date_with_reference};
use super::super::parse_quick_add;

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
fn test_parse_date_month_day_dash_format() {
    let parsed = parse_quick_add("Task due:12-25");
    let year = Utc::now().date_naive().year();
    assert_eq!(
        parsed.due_date,
        Some(NaiveDate::from_ymd_opt(year, 12, 25).unwrap())
    );
}

#[test]
fn test_parse_date_abbreviations() {
    let parsed = parse_quick_add("Task due:tod");
    assert_eq!(parsed.due_date, Some(Utc::now().date_naive()));

    let parsed2 = parse_quick_add("Task due:tom");
    let tomorrow = Utc::now().date_naive() + chrono::Duration::days(1);
    assert_eq!(parsed2.due_date, Some(tomorrow));
}

#[test]
fn test_parse_date_with_reference_today_tomorrow() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(); // Sunday

    assert_eq!(
        parse_date_with_reference("today", reference),
        Some(reference)
    );
    assert_eq!(parse_date_with_reference("tod", reference), Some(reference));
    assert_eq!(
        parse_date_with_reference("tomorrow", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("tom", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap())
    );
}

#[test]
fn test_parse_date_with_reference_yesterday() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("yesterday", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 14).unwrap())
    );
}

#[test]
fn test_parse_date_iso_format() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("2025-12-25", reference),
        Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
    );
}

#[test]
fn test_parse_date_month_day_slash() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("12/25", reference),
        Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
    );
}

#[test]
fn test_plain_weekday_parsing() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(); // Sunday

    // Plain "monday" should give next Monday
    let result = parse_date_with_reference("monday", reference);
    assert!(result.is_some());
    assert_eq!(result.unwrap().weekday(), Weekday::Mon);

    // Test abbreviated form
    let result = parse_date_with_reference("mon", reference);
    assert!(result.is_some());
    assert_eq!(result.unwrap().weekday(), Weekday::Mon);
}

#[test]
fn test_quick_add_with_smart_dates() {
    // Test that quick add works with new smart date formats
    let parsed = parse_quick_add("Meeting due:tomorrow");
    let tomorrow = Utc::now().date_naive() + chrono::Duration::days(1);
    assert_eq!(parsed.due_date, Some(tomorrow));

    // Test with "next week" - contains space so use due:nextweek
    let parsed = parse_quick_add("Report sched:nextweek");
    assert!(parsed.scheduled_date.is_some());
}
