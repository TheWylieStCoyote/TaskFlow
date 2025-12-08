//! Tests for quick add parsing.

use chrono::{Datelike, NaiveDate, Utc, Weekday};

use crate::domain::Priority;

use super::date::{days_in_month, is_leap_year, next_weekday, parse_date_with_reference};
use super::parse_quick_add;

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
    use super::parse_priority;
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

// Edge case tests

#[test]
fn test_parse_multiple_priorities_uses_first() {
    // When multiple priorities are given, regex captures the first match
    let parsed = parse_quick_add("Task !high !low !urgent");
    // The regex only captures the first priority
    assert_eq!(parsed.priority, Some(Priority::High));
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
    assert_eq!(parsed.due_date, Some(Utc::now().date_naive()));
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
fn test_parse_date_month_day_dash_format() {
    let parsed = parse_quick_add("Task due:12-25");
    let year = Utc::now().date_naive().year();
    assert_eq!(
        parsed.due_date,
        Some(NaiveDate::from_ymd_opt(year, 12, 25).unwrap())
    );
}

#[test]
fn test_parse_priority_case_insensitive() {
    let parsed1 = parse_quick_add("Task !HIGH");
    let parsed2 = parse_quick_add("Task !High");
    let parsed3 = parse_quick_add("Task !high");
    assert_eq!(parsed1.priority, Some(Priority::High));
    assert_eq!(parsed2.priority, Some(Priority::High));
    assert_eq!(parsed3.priority, Some(Priority::High));
}

#[test]
fn test_parse_unrecognized_priority() {
    let parsed = parse_quick_add("Task !invalid");
    // Unrecognized priority string should result in None
    assert!(parsed.priority.is_none());
}

#[test]
fn test_parse_date_abbreviations() {
    let parsed = parse_quick_add("Task due:tod");
    assert_eq!(parsed.due_date, Some(Utc::now().date_naive()));

    let parsed2 = parse_quick_add("Task due:tom");
    let tomorrow = Utc::now().date_naive() + chrono::Duration::days(1);
    assert_eq!(parsed2.due_date, Some(tomorrow));
}

// Smart date parsing tests

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
fn test_parse_relative_duration_days() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("in 3 days", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 18).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("in 1 day", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("in 10 d", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 25).unwrap())
    );
}

#[test]
fn test_parse_relative_duration_weeks() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("in 2 weeks", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 29).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("in 1 week", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 22).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("in 1 w", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 22).unwrap())
    );
}

#[test]
fn test_parse_relative_duration_months() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("in 1 month", reference),
        Some(NaiveDate::from_ymd_opt(2025, 7, 15).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("in 3 months", reference),
        Some(NaiveDate::from_ymd_opt(2025, 9, 15).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("in 1 m", reference),
        Some(NaiveDate::from_ymd_opt(2025, 7, 15).unwrap())
    );
}

#[test]
fn test_parse_relative_duration_months_year_wrap() {
    let reference = NaiveDate::from_ymd_opt(2025, 11, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("in 3 months", reference),
        Some(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap())
    );
}

#[test]
fn test_parse_relative_duration_months_day_overflow() {
    // Jan 31 + 1 month should be Feb 28 (non-leap year)
    let reference = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

    assert_eq!(
        parse_date_with_reference("in 1 month", reference),
        Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap())
    );
}

#[test]
fn test_parse_end_of_week() {
    // June 15, 2025 is a Sunday
    let sunday = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    // June 16, 2025 is a Monday
    let monday = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();
    // June 18, 2025 is a Wednesday
    let wednesday = NaiveDate::from_ymd_opt(2025, 6, 18).unwrap();

    // From Sunday, end of week is same day (Sunday)
    assert_eq!(parse_date_with_reference("eow", sunday), Some(sunday));

    // From Monday, end of week is Sunday (June 22)
    assert_eq!(
        parse_date_with_reference("eow", monday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 22).unwrap())
    );

    // From Wednesday, end of week is Sunday (June 22)
    assert_eq!(
        parse_date_with_reference("end of week", wednesday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 22).unwrap())
    );
}

#[test]
fn test_parse_end_of_month() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    // June has 30 days
    assert_eq!(
        parse_date_with_reference("eom", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("end of month", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap())
    );
}

#[test]
fn test_parse_end_of_month_31_days() {
    let reference = NaiveDate::from_ymd_opt(2025, 7, 10).unwrap();

    // July has 31 days
    assert_eq!(
        parse_date_with_reference("eom", reference),
        Some(NaiveDate::from_ymd_opt(2025, 7, 31).unwrap())
    );
}

#[test]
fn test_parse_end_of_month_february() {
    let reference = NaiveDate::from_ymd_opt(2025, 2, 10).unwrap();

    // February 2025 has 28 days (not a leap year)
    assert_eq!(
        parse_date_with_reference("eom", reference),
        Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap())
    );
}

#[test]
fn test_parse_end_of_month_february_leap_year() {
    let reference = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();

    // February 2024 has 29 days (leap year)
    assert_eq!(
        parse_date_with_reference("eom", reference),
        Some(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap())
    );
}

#[test]
fn test_parse_end_of_year() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("eoy", reference),
        Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("end of year", reference),
        Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
    );
}

#[test]
fn test_parse_ordinal_day_current_month() {
    // On June 15, "20th" should be June 20
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("20th", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("25th", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 25).unwrap())
    );
}

#[test]
fn test_parse_ordinal_day_next_month() {
    // On June 15, "10th" should be July 10 (already passed this month)
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("10th", reference),
        Some(NaiveDate::from_ymd_opt(2025, 7, 10).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("1st", reference),
        Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap())
    );
}

#[test]
fn test_parse_ordinal_day_same_day() {
    // On June 15, "15th" should be June 15 (today)
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("15th", reference),
        Some(reference)
    );
}

#[test]
fn test_parse_ordinal_day_various_suffixes() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();

    assert_eq!(parse_date_with_reference("1st", reference), Some(reference));
    assert_eq!(
        parse_date_with_reference("2nd", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 2).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("3rd", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 3).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("4th", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 4).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("22nd", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 22).unwrap())
    );
}

#[test]
fn test_parse_ordinal_last_day() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("last day", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("lastday", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap())
    );
}

#[test]
fn test_parse_extended_weekday_next() {
    // June 15, 2025 is a Sunday
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    // "next monday" from Sunday June 15 should be June 23 (Monday 8 days away)
    assert_eq!(
        parse_date_with_reference("next monday", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 23).unwrap())
    );

    // "next friday" from Sunday should be June 27 (Friday 12 days away)
    assert_eq!(
        parse_date_with_reference("next friday", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 27).unwrap())
    );

    // "next sunday" should be June 22 (next Sunday, 7 days away)
    assert_eq!(
        parse_date_with_reference("next sunday", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 22).unwrap())
    );
}

#[test]
fn test_parse_extended_weekday_this() {
    // June 16, 2025 is a Monday
    let monday = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();

    // "this friday" from Monday should be June 20 (this week's Friday)
    assert_eq!(
        parse_date_with_reference("this friday", monday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap())
    );

    // "this monday" should be today (June 16)
    assert_eq!(
        parse_date_with_reference("this monday", monday),
        Some(monday)
    );

    // "this sunday" should be June 22
    assert_eq!(
        parse_date_with_reference("this sunday", monday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 22).unwrap())
    );
}

#[test]
fn test_parse_extended_weekday_this_past() {
    // June 18, 2025 is a Wednesday
    let wednesday = NaiveDate::from_ymd_opt(2025, 6, 18).unwrap();

    // "this monday" from Wednesday should be June 16 (past day this week)
    assert_eq!(
        parse_date_with_reference("this monday", wednesday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap())
    );
}

#[test]
fn test_parse_next_week() {
    // June 15, 2025 is a Sunday
    let sunday = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    // June 18, 2025 is a Wednesday
    let wednesday = NaiveDate::from_ymd_opt(2025, 6, 18).unwrap();

    // "next week" from Sunday should be June 16 (Monday)
    assert_eq!(
        parse_date_with_reference("next week", sunday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap())
    );

    // "next week" from Wednesday should be June 23 (Monday of next week)
    assert_eq!(
        parse_date_with_reference("next week", wednesday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 23).unwrap())
    );

    // Also test "nextweek" without space
    assert_eq!(
        parse_date_with_reference("nextweek", wednesday),
        Some(NaiveDate::from_ymd_opt(2025, 6, 23).unwrap())
    );
}

#[test]
fn test_parse_next_month() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    // "next month" should be July 1
    assert_eq!(
        parse_date_with_reference("next month", reference),
        Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("nextmonth", reference),
        Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap())
    );
}

#[test]
fn test_parse_next_month_december() {
    let reference = NaiveDate::from_ymd_opt(2025, 12, 15).unwrap();

    // "next month" in December should be Jan 1 of next year
    assert_eq!(
        parse_date_with_reference("next month", reference),
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    );
}

#[test]
fn test_parse_next_year() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("next year", reference),
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("nextyear", reference),
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    );
}

#[test]
fn test_parse_date_case_insensitive() {
    let reference = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert_eq!(
        parse_date_with_reference("TODAY", reference),
        Some(reference)
    );
    assert_eq!(
        parse_date_with_reference("Tomorrow", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("NEXT WEEK", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("In 3 Days", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 18).unwrap())
    );
    assert_eq!(
        parse_date_with_reference("EOM", reference),
        Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap())
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
