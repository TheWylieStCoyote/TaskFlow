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
