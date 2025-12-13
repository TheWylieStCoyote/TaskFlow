//! Time blocking quick add parsing tests.

use chrono::NaiveTime;

use super::super::{parse_quick_add, parse_single_time, parse_time_range};

// ============================================================================
// Single Time Parsing Tests
// ============================================================================

#[test]
fn test_parse_single_time_24h_format() {
    assert_eq!(
        parse_single_time("14:30"),
        Some(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("09:00"),
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("0:00"),
        Some(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("23:59"),
        Some(NaiveTime::from_hms_opt(23, 59, 0).unwrap())
    );
}

#[test]
fn test_parse_single_time_12h_am() {
    assert_eq!(
        parse_single_time("9am"),
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("9:30am"),
        Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("12am"),
        Some(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    );
}

#[test]
fn test_parse_single_time_12h_pm() {
    assert_eq!(
        parse_single_time("2pm"),
        Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("2:30pm"),
        Some(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("12pm"),
        Some(NaiveTime::from_hms_opt(12, 0, 0).unwrap())
    );
}

#[test]
fn test_parse_single_time_hour_only() {
    assert_eq!(
        parse_single_time("9"),
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("14"),
        Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
    );
}

#[test]
fn test_parse_single_time_invalid() {
    assert_eq!(parse_single_time(""), None);
    assert_eq!(parse_single_time("25:00"), None); // Hour out of range
    assert_eq!(parse_single_time("12:60"), None); // Minute out of range
    assert_eq!(parse_single_time("13pm"), None); // Invalid 12h format
    assert_eq!(parse_single_time("0am"), None); // 0 invalid in 12h
    assert_eq!(parse_single_time("abc"), None);
}

#[test]
fn test_parse_single_time_whitespace() {
    assert_eq!(
        parse_single_time("  9:00  "),
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(
        parse_single_time(" 2pm "),
        Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
    );
}

#[test]
fn test_parse_single_time_case_insensitive() {
    assert_eq!(
        parse_single_time("9AM"),
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(
        parse_single_time("2PM"),
        Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
    );
}

// ============================================================================
// Time Range Parsing Tests
// ============================================================================

#[test]
fn test_parse_time_range_24h() {
    let expected = (
        NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
    );
    assert_eq!(parse_time_range("9:00-11:00"), Some(expected));
}

#[test]
fn test_parse_time_range_12h() {
    let expected = (
        NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
    );
    assert_eq!(parse_time_range("9am-11am"), Some(expected));
}

#[test]
fn test_parse_time_range_mixed() {
    let expected = (
        NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
    );
    assert_eq!(parse_time_range("9:00-14:30"), Some(expected));
}

#[test]
fn test_parse_time_range_afternoon() {
    let expected = (
        NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
        NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
    );
    assert_eq!(parse_time_range("2pm-5pm"), Some(expected));
}

#[test]
fn test_parse_time_range_invalid() {
    assert_eq!(parse_time_range("9:00"), None); // No range
    assert_eq!(parse_time_range("9:00-"), None); // Missing end
    assert_eq!(parse_time_range("-11:00"), None); // Missing start
    assert_eq!(parse_time_range("9:00-11:00-13:00"), None); // Too many parts
    assert_eq!(parse_time_range("abc-def"), None); // Invalid times
}

// ============================================================================
// Quick Add Time Syntax Tests
// ============================================================================

#[test]
fn test_quick_add_time_range() {
    let parsed = parse_quick_add("Meeting time:9:00-11:00");
    assert_eq!(parsed.title, "Meeting");
    assert_eq!(
        parsed.scheduled_start_time,
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(
        parsed.scheduled_end_time,
        Some(NaiveTime::from_hms_opt(11, 0, 0).unwrap())
    );
}

#[test]
fn test_quick_add_time_12h() {
    let parsed = parse_quick_add("Call time:2pm-3pm");
    assert_eq!(parsed.title, "Call");
    assert_eq!(
        parsed.scheduled_start_time,
        Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
    );
    assert_eq!(
        parsed.scheduled_end_time,
        Some(NaiveTime::from_hms_opt(15, 0, 0).unwrap())
    );
}

#[test]
fn test_quick_add_time_single() {
    let parsed = parse_quick_add("Standup time:9am");
    assert_eq!(parsed.title, "Standup");
    assert_eq!(
        parsed.scheduled_start_time,
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(parsed.scheduled_end_time, None);
}

#[test]
fn test_quick_add_time_with_other_metadata() {
    let parsed = parse_quick_add("Meeting #work !high time:9:00-11:00 @project");
    assert_eq!(parsed.title, "Meeting");
    assert_eq!(parsed.tags, vec!["work"]);
    assert_eq!(parsed.priority, Some(crate::domain::Priority::High));
    assert_eq!(parsed.project_name, Some("project".to_string()));
    assert_eq!(
        parsed.scheduled_start_time,
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
    assert_eq!(
        parsed.scheduled_end_time,
        Some(NaiveTime::from_hms_opt(11, 0, 0).unwrap())
    );
}

#[test]
fn test_quick_add_time_with_scheduled_date() {
    let parsed = parse_quick_add("Review sched:tomorrow time:14:00-15:30");
    assert_eq!(parsed.title, "Review");
    assert!(parsed.scheduled_date.is_some());
    assert_eq!(
        parsed.scheduled_start_time,
        Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
    );
    assert_eq!(
        parsed.scheduled_end_time,
        Some(NaiveTime::from_hms_opt(15, 30, 0).unwrap())
    );
}

#[test]
fn test_quick_add_invalid_time_ignored() {
    let parsed = parse_quick_add("Task time:invalid");
    assert_eq!(parsed.title, "Task");
    assert_eq!(parsed.scheduled_start_time, None);
    assert_eq!(parsed.scheduled_end_time, None);
}

#[test]
fn test_quick_add_time_only() {
    let parsed = parse_quick_add("time:9:00-10:00");
    assert_eq!(parsed.title, "");
    assert_eq!(
        parsed.scheduled_start_time,
        Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    );
}
