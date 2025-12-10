//! Smart/relative date parsing tests.

use chrono::NaiveDate;

use super::super::date::parse_date_with_reference;

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
