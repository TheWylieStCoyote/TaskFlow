//! Tests for habit tracking.

use super::*;
use chrono::{TimeDelta, Utc, Weekday};

#[test]
fn test_new_habit() {
    let habit = Habit::new("Exercise");
    assert_eq!(habit.name, "Exercise");
    assert!(matches!(habit.frequency, HabitFrequency::Daily));
    assert!(!habit.archived);
    assert!(habit.check_ins.is_empty());
}

#[test]
fn test_habit_builder() {
    let habit = Habit::new("Read")
        .with_description("Read for 30 minutes")
        .with_color("#3498db")
        .with_tags(vec!["learning".into()]);

    assert_eq!(habit.description, Some("Read for 30 minutes".to_string()));
    assert_eq!(habit.color, Some("#3498db".to_string()));
    assert_eq!(habit.tags, vec!["learning"]);
}

#[test]
fn test_check_in() {
    let mut habit = Habit::new("Test");
    let today = Utc::now().date_naive();

    habit.check_in_today(true, Some("Done!".to_string()));

    assert!(habit.is_completed_on(today));
    let check_in = habit.check_ins.get(&today).unwrap();
    assert_eq!(check_in.note, Some("Done!".to_string()));
}

#[test]
fn test_current_streak() {
    let today = Utc::now().date_naive();
    let mut habit = Habit::new("Test").with_start_date(today - TimeDelta::days(10));

    // No check-ins = 0 streak
    assert_eq!(habit.current_streak(), 0);

    // One check-in today = 1 streak
    habit.check_in(today, true, None);
    assert_eq!(habit.current_streak(), 1);

    // Add yesterday = 2 streak
    habit.check_in(today - TimeDelta::days(1), true, None);
    assert_eq!(habit.current_streak(), 2);

    // Add 2 days ago = 3 streak
    habit.check_in(today - TimeDelta::days(2), true, None);
    assert_eq!(habit.current_streak(), 3);
}

#[test]
fn test_streak_broken_by_miss() {
    let today = Utc::now().date_naive();
    let mut habit = Habit::new("Test").with_start_date(today - TimeDelta::days(10));

    // Check in today and 2 days ago but not yesterday
    habit.check_in(today, true, None);
    habit.check_in(today - TimeDelta::days(2), true, None);
    // Yesterday is missing = broken streak

    assert_eq!(habit.current_streak(), 1);
}

#[test]
fn test_longest_streak() {
    let today = Utc::now().date_naive();
    let mut habit = Habit::new("Test").with_start_date(today - TimeDelta::days(20));

    // Build a 5-day streak in the past
    for i in 10..15 {
        habit.check_in(today - TimeDelta::days(i), true, None);
    }

    // Current 2-day streak
    habit.check_in(today, true, None);
    habit.check_in(today - TimeDelta::days(1), true, None);

    assert_eq!(habit.current_streak(), 2);
    assert_eq!(habit.longest_streak(), 5);
}

#[test]
fn test_frequency_daily() {
    let freq = HabitFrequency::Daily;
    let today = Utc::now().date_naive();
    let start = today - TimeDelta::days(10);

    assert!(freq.is_due_on(today, start));
    assert!(freq.is_due_on(today - TimeDelta::days(1), start));
}

#[test]
fn test_frequency_weekly() {
    use chrono::NaiveDate;

    let freq = HabitFrequency::Weekly {
        days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
    };
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    // Jan 1, 2024 was a Monday
    assert!(freq.is_due_on(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), start));
    // Jan 3 was Wednesday
    assert!(freq.is_due_on(NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(), start));
    // Jan 2 was Tuesday - not due
    assert!(!freq.is_due_on(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(), start));
}

#[test]
fn test_frequency_every_n_days() {
    use chrono::NaiveDate;

    let freq = HabitFrequency::EveryNDays { n: 3 };
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    // Due on start date
    assert!(freq.is_due_on(start, start));
    // Due 3 days later
    assert!(freq.is_due_on(start + TimeDelta::days(3), start));
    // Due 6 days later
    assert!(freq.is_due_on(start + TimeDelta::days(6), start));
    // Not due 1 day later
    assert!(!freq.is_due_on(start + TimeDelta::days(1), start));
}

#[test]
fn test_completion_rate_by_weekday() {
    use chrono::NaiveDate;

    let mut habit = Habit::new("Test");

    // Add completions on Monday
    habit.check_in(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), // Monday
        true,
        None,
    );
    habit.check_in(
        NaiveDate::from_ymd_opt(2024, 1, 8).unwrap(), // Monday
        true,
        None,
    );
    // Add miss on Tuesday
    habit.check_in(
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(), // Tuesday
        false,
        None,
    );

    let rates = habit.completion_rate_by_weekday();
    assert!((rates[0] - 100.0).abs() < 0.01); // Monday: 2/2 = 100%
    assert!(rates[1].abs() < 0.01); // Tuesday: 0/1 = 0%
}

#[test]
fn test_overall_completion_rate() {
    let mut habit = Habit::new("Test");
    let today = Utc::now().date_naive();

    // 3 completions, 1 miss = 75%
    habit.check_in(today, true, None);
    habit.check_in(today - TimeDelta::days(1), true, None);
    habit.check_in(today - TimeDelta::days(2), true, None);
    habit.check_in(today - TimeDelta::days(3), false, None);

    assert!((habit.overall_completion_rate() - 75.0).abs() < 0.01);
}

#[test]
fn test_habit_id_display() {
    let id = HabitId::new();
    let display = format!("{id}");
    assert!(!display.is_empty());
}

#[test]
fn test_frequency_display() {
    assert_eq!(format!("{}", HabitFrequency::Daily), "Daily");
    assert_eq!(
        format!(
            "{}",
            HabitFrequency::Weekly {
                days: vec![Weekday::Mon, Weekday::Fri]
            }
        ),
        "Weekly (Mon, Fri)"
    );
    assert_eq!(
        format!("{}", HabitFrequency::EveryNDays { n: 3 }),
        "Every 3 days"
    );
}

// ============================================================================
// Recurrence Edge Cases - Year Boundaries
// ============================================================================

#[test]
fn test_every_n_days_year_boundary() {
    use chrono::NaiveDate;

    let freq = HabitFrequency::EveryNDays { n: 7 };
    let start = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();

    // Dec 25 is start, should be due
    assert!(freq.is_due_on(start, start));

    // Jan 1, 2025 is 7 days later, should be due
    let jan_1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    assert!(freq.is_due_on(jan_1, start));

    // Dec 30 is 5 days later, should NOT be due
    let dec_30 = NaiveDate::from_ymd_opt(2024, 12, 30).unwrap();
    assert!(!freq.is_due_on(dec_30, start));
}

#[test]
fn test_weekly_habit_year_boundary() {
    use chrono::NaiveDate;

    let freq = HabitFrequency::Weekly {
        days: vec![Weekday::Mon],
    };
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    // Dec 30, 2024 is a Monday
    let dec_30 = NaiveDate::from_ymd_opt(2024, 12, 30).unwrap();
    assert!(freq.is_due_on(dec_30, start));

    // Jan 6, 2025 is a Monday
    let jan_6 = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
    assert!(freq.is_due_on(jan_6, start));

    // Dec 31, 2024 is a Tuesday - not due
    let dec_31 = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    assert!(!freq.is_due_on(dec_31, start));
}

#[test]
fn test_streak_across_year_boundary() {
    use chrono::NaiveDate;

    let start = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
    let mut habit = Habit::new("Test").with_start_date(start);

    // Build a streak across the year boundary
    // Dec 28, 29, 30, 31, Jan 1, 2, 3
    for day in 28..=31 {
        habit.check_in(NaiveDate::from_ymd_opt(2024, 12, day).unwrap(), true, None);
    }
    for day in 1..=3 {
        habit.check_in(NaiveDate::from_ymd_opt(2025, 1, day).unwrap(), true, None);
    }

    // The longest streak should be 7 (Dec 28 - Jan 3)
    assert_eq!(habit.longest_streak(), 7);
}

#[test]
fn test_every_n_days_leap_year_boundary() {
    use chrono::NaiveDate;

    let freq = HabitFrequency::EveryNDays { n: 1 };
    let start = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();

    // Feb 28 is start - due
    assert!(freq.is_due_on(start, start));

    // Feb 29 (leap day) is 1 day later - due
    let feb_29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    assert!(freq.is_due_on(feb_29, start));

    // Mar 1 is 2 days later - due
    let mar_1 = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
    assert!(freq.is_due_on(mar_1, start));
}

#[test]
fn test_weekly_all_days() {
    use chrono::NaiveDate;

    // All days of the week - should always be due
    let freq = HabitFrequency::Weekly {
        days: vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ],
    };
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    // Check a full week
    for day in 1..=7 {
        let date = NaiveDate::from_ymd_opt(2024, 1, day).unwrap();
        assert!(freq.is_due_on(date, start), "Day {} should be due", day);
    }
}

#[test]
fn test_completion_rate_across_year_boundary() {
    use chrono::NaiveDate;

    let mut habit = Habit::new("Test");

    // Complete in December 2024
    habit.check_in(NaiveDate::from_ymd_opt(2024, 12, 30).unwrap(), true, None);
    habit.check_in(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), true, None);

    // Complete in January 2025
    habit.check_in(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), true, None);
    habit.check_in(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(), false, None); // miss

    // Overall: 3 completed, 1 missed = 75%
    assert!((habit.overall_completion_rate() - 75.0).abs() < 0.01);
}
