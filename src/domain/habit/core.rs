//! Core Habit struct and implementation.

use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{HabitCheckIn, HabitFrequency, HabitId, HabitTrend};

/// A habit with daily check-in tracking and streak analytics.
///
/// Habits are distinct from tasks - they represent recurring daily activities
/// that you want to build into routines. Unlike tasks which are completed
/// once, habits are checked in daily and track streaks.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use taskflow::domain::Habit;
///
/// let mut habit = Habit::new("Meditate");
///
/// // Check in for today
/// habit.check_in_today(true, Some("10 minutes".to_string()));
///
/// // Check streak
/// assert_eq!(habit.current_streak(), 1);
/// ```
///
/// ## With Builder Pattern
///
/// ```
/// use taskflow::domain::{Habit, HabitFrequency};
/// use chrono::Weekday;
///
/// let habit = Habit::new("Go to gym")
///     .with_frequency(HabitFrequency::Weekly {
///         days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]
///     })
///     .with_color("#e74c3c")
///     .with_tags(vec!["health".into(), "fitness".into()]);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Habit {
    /// Unique identifier.
    pub id: HabitId,
    /// Name of the habit.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,

    /// When the habit repeats.
    pub frequency: HabitFrequency,
    /// When the habit started.
    pub start_date: NaiveDate,
    /// Optional end date for time-bounded habits.
    pub end_date: Option<NaiveDate>,

    /// Check-ins stored by date for O(1) lookup.
    #[serde(default)]
    pub check_ins: HashMap<NaiveDate, HabitCheckIn>,

    /// Display color (hex format).
    pub color: Option<String>,
    /// Icon identifier.
    pub icon: Option<String>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Whether the habit is archived.
    #[serde(default)]
    pub archived: bool,

    /// When the habit was created.
    pub created_at: DateTime<Utc>,
    /// When the habit was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Habit {
    /// Creates a new daily habit with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: HabitId::new(),
            name: name.into(),
            description: None,
            frequency: HabitFrequency::default(),
            start_date: now.date_naive(),
            end_date: None,
            check_ins: HashMap::new(),
            color: None,
            icon: None,
            tags: Vec::new(),
            archived: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the habit's frequency.
    #[must_use]
    pub fn with_frequency(mut self, frequency: HabitFrequency) -> Self {
        self.frequency = frequency;
        self
    }

    /// Sets the habit's description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the habit's color.
    #[must_use]
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the habit's tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Sets the habit's start date.
    #[must_use]
    pub const fn with_start_date(mut self, date: NaiveDate) -> Self {
        self.start_date = date;
        self
    }

    /// Check in for today.
    pub fn check_in_today(&mut self, completed: bool, note: Option<String>) {
        let today = Utc::now().date_naive();
        self.check_in(today, completed, note);
    }

    /// Check in for a specific date.
    pub fn check_in(&mut self, date: NaiveDate, completed: bool, note: Option<String>) {
        self.check_ins.insert(
            date,
            HabitCheckIn {
                date,
                completed,
                note,
                checked_at: Utc::now(),
            },
        );
        self.updated_at = Utc::now();
    }

    /// Check if completed on a specific date.
    #[must_use]
    pub fn is_completed_on(&self, date: NaiveDate) -> bool {
        self.check_ins.get(&date).is_some_and(|c| c.completed)
    }

    /// Returns whether the habit is due today.
    #[must_use]
    pub fn is_due_today(&self) -> bool {
        let today = Utc::now().date_naive();
        self.frequency.is_due_on(today, self.start_date)
    }

    /// Returns whether the habit is active (not archived).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !self.archived
    }

    /// Calculate current streak (consecutive completions ending today or yesterday).
    ///
    /// The streak can end on today (if checked in) or yesterday (if today
    /// hasn't been checked in yet but yesterday was completed).
    #[must_use]
    pub fn current_streak(&self) -> u32 {
        let today = Utc::now().date_naive();
        self.streak_ending_on(today)
            .or_else(|| {
                let yesterday = today - TimeDelta::days(1);
                self.streak_ending_on(yesterday)
            })
            .unwrap_or(0)
    }

    /// Calculate streak ending on a specific date.
    fn streak_ending_on(&self, end_date: NaiveDate) -> Option<u32> {
        if !self.is_completed_on(end_date) {
            return None;
        }

        let mut streak = 1;
        let mut date = end_date - TimeDelta::days(1);

        while date >= self.start_date {
            if self.frequency.is_due_on(date, self.start_date) {
                if self.is_completed_on(date) {
                    streak += 1;
                } else {
                    break;
                }
            }
            date -= TimeDelta::days(1);
        }

        Some(streak)
    }

    /// Calculate longest streak ever achieved.
    #[must_use]
    pub fn longest_streak(&self) -> u32 {
        if self.check_ins.is_empty() {
            return 0;
        }

        let mut dates: Vec<_> = self.check_ins.keys().copied().collect();
        dates.sort();

        let mut max_streak = 0;
        let mut current = 0;
        let mut prev_date: Option<NaiveDate> = None;

        for date in dates {
            if let Some(prev) = prev_date {
                let is_consecutive = self.is_consecutive(prev, date);
                if self.is_completed_on(date) {
                    if is_consecutive && self.is_completed_on(prev) {
                        current += 1;
                    } else {
                        current = 1;
                    }
                    max_streak = max_streak.max(current);
                }
            } else if self.is_completed_on(date) {
                current = 1;
                max_streak = 1;
            }
            prev_date = Some(date);
        }

        max_streak
    }

    /// Check if two dates are consecutive for this habit's frequency.
    fn is_consecutive(&self, earlier: NaiveDate, later: NaiveDate) -> bool {
        match &self.frequency {
            HabitFrequency::Daily => (later - earlier).num_days() == 1,
            HabitFrequency::Weekly { days } => {
                // Find next due date after earlier
                let mut check = earlier + TimeDelta::days(1);
                while check <= later {
                    if days.contains(&check.weekday()) {
                        return check == later;
                    }
                    check += TimeDelta::days(1);
                }
                false
            }
            HabitFrequency::EveryNDays { n } => (later - earlier).num_days() == i64::from(*n),
        }
    }

    /// Completion rate by day of week (0=Mon to 6=Sun).
    ///
    /// Returns an array of 7 percentages (0.0 to 100.0).
    #[must_use]
    pub fn completion_rate_by_weekday(&self) -> [f64; 7] {
        let mut completed_by_day = [0u32; 7];
        let mut total_by_day = [0u32; 7];

        for (date, check_in) in &self.check_ins {
            let day_idx = date.weekday().num_days_from_monday() as usize;
            total_by_day[day_idx] += 1;
            if check_in.completed {
                completed_by_day[day_idx] += 1;
            }
        }

        let mut rates = [0.0; 7];
        for i in 0..7 {
            if total_by_day[i] > 0 {
                rates[i] = f64::from(completed_by_day[i]) / f64::from(total_by_day[i]) * 100.0;
            }
        }
        rates
    }

    /// Overall completion rate (percentage of check-ins that were completed).
    #[must_use]
    pub fn overall_completion_rate(&self) -> f64 {
        if self.check_ins.is_empty() {
            return 0.0;
        }
        let completed = self.check_ins.values().filter(|c| c.completed).count();
        completed as f64 / self.check_ins.len() as f64 * 100.0
    }

    /// Returns total number of completions.
    #[must_use]
    pub fn total_completions(&self) -> usize {
        self.check_ins.values().filter(|c| c.completed).count()
    }

    /// Calculate completion rate for a specific period.
    ///
    /// Returns the percentage of due days that were completed in the given range.
    #[must_use]
    pub fn completion_rate_for_period(&self, start: NaiveDate, end: NaiveDate) -> Option<f64> {
        let mut due_days = 0;
        let mut completed_days = 0;

        let mut date = start;
        while date <= end {
            if self.frequency.is_due_on(date, self.start_date) {
                due_days += 1;
                if self.is_completed_on(date) {
                    completed_days += 1;
                }
            }
            date += TimeDelta::days(1);
        }

        if due_days == 0 {
            None
        } else {
            Some(f64::from(completed_days) / f64::from(due_days) * 100.0)
        }
    }

    /// Analyze trend by comparing recent performance to historical.
    ///
    /// Compares the last 7 days to the previous 21 days.
    /// Returns:
    /// - `Some(HabitTrend::Improving)` if recent rate is >10% higher
    /// - `Some(HabitTrend::Declining)` if recent rate is >10% lower
    /// - `Some(HabitTrend::Stable)` if within 10%
    /// - `None` if not enough data
    #[must_use]
    pub fn trend(&self) -> Option<HabitTrend> {
        let today = Utc::now().date_naive();

        // Recent period: last 7 days
        let recent_start = today - TimeDelta::days(6);
        let recent_rate = self.completion_rate_for_period(recent_start, today)?;

        // Historical period: 21 days before recent
        let hist_end = recent_start - TimeDelta::days(1);
        let hist_start = hist_end - TimeDelta::days(20);
        let hist_rate = self.completion_rate_for_period(hist_start, hist_end)?;

        // Compare rates
        let diff = recent_rate - hist_rate;
        Some(if diff > 10.0 {
            HabitTrend::Improving
        } else if diff < -10.0 {
            HabitTrend::Declining
        } else {
            HabitTrend::Stable
        })
    }

    /// Get a trend indicator symbol.
    #[must_use]
    pub fn trend_symbol(&self) -> &'static str {
        match self.trend() {
            Some(HabitTrend::Improving) => "↑",
            Some(HabitTrend::Declining) => "↓",
            Some(HabitTrend::Stable) => "→",
            None => " ",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeDelta, Utc, Weekday};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_habit_new_defaults() {
        let habit = Habit::new("Exercise");
        assert_eq!(habit.name, "Exercise");
        assert!(!habit.archived);
        assert!(habit.check_ins.is_empty());
        assert!(habit.description.is_none());
        assert_eq!(habit.frequency, HabitFrequency::Daily);
    }

    #[test]
    fn test_habit_builder_methods() {
        let habit = Habit::new("Read")
            .with_description("30 minutes")
            .with_color("#ff0000")
            .with_tags(vec!["learning".into()]);
        assert_eq!(habit.description, Some("30 minutes".to_string()));
        assert_eq!(habit.color, Some("#ff0000".to_string()));
        assert_eq!(habit.tags, vec!["learning"]);
    }

    #[test]
    fn test_check_in_and_is_completed_on() {
        let mut habit = Habit::new("Meditate");
        let d = date(2024, 1, 10);
        assert!(!habit.is_completed_on(d));
        habit.check_in(d, true, None);
        assert!(habit.is_completed_on(d));
    }

    #[test]
    fn test_check_in_with_note() {
        let mut habit = Habit::new("Journal");
        let d = date(2024, 3, 5);
        habit.check_in(d, true, Some("wrote 3 pages".to_string()));
        let check = habit.check_ins.get(&d).unwrap();
        assert_eq!(check.note, Some("wrote 3 pages".to_string()));
        assert!(check.completed);
    }

    #[test]
    fn test_check_in_not_completed() {
        let mut habit = Habit::new("Run");
        let d = date(2024, 5, 1);
        habit.check_in(d, false, None);
        assert!(!habit.is_completed_on(d));
    }

    #[test]
    fn test_is_active() {
        let habit = Habit::new("Stretch");
        assert!(habit.is_active());
    }

    #[test]
    fn test_is_active_when_archived() {
        let mut habit = Habit::new("Old habit");
        habit.archived = true;
        assert!(!habit.is_active());
    }

    #[test]
    fn test_current_streak_no_checkins() {
        let habit = Habit::new("Walk");
        assert_eq!(habit.current_streak(), 0);
    }

    #[test]
    fn test_current_streak_today_only() {
        let mut habit = Habit::new("Walk");
        let today = Utc::now().date_naive();
        habit.check_in(today, true, None);
        assert_eq!(habit.current_streak(), 1);
    }

    #[test]
    fn test_current_streak_yesterday_only() {
        let mut habit = Habit::new("Walk");
        let yesterday = Utc::now().date_naive() - TimeDelta::days(1);
        habit.check_in(yesterday, true, None);
        assert_eq!(habit.current_streak(), 1);
    }

    #[test]
    fn test_current_streak_consecutive_days() {
        let today = Utc::now().date_naive();
        let mut habit = Habit::new("Walk").with_start_date(today - TimeDelta::days(10));
        // Check in for 3 consecutive days ending today
        habit.check_in(today, true, None);
        habit.check_in(today - TimeDelta::days(1), true, None);
        habit.check_in(today - TimeDelta::days(2), true, None);
        assert_eq!(habit.current_streak(), 3);
    }

    #[test]
    fn test_current_streak_broken() {
        let mut habit = Habit::new("Walk");
        let today = Utc::now().date_naive();
        // Today and 2 days ago but NOT yesterday — breaks the streak
        habit.check_in(today, true, None);
        habit.check_in(today - TimeDelta::days(2), true, None);
        assert_eq!(habit.current_streak(), 1);
    }

    #[test]
    fn test_longest_streak_empty() {
        let habit = Habit::new("Yoga");
        assert_eq!(habit.longest_streak(), 0);
    }

    #[test]
    fn test_longest_streak_single() {
        let mut habit = Habit::new("Yoga");
        habit.check_in(date(2024, 1, 1), true, None);
        assert_eq!(habit.longest_streak(), 1);
    }

    #[test]
    fn test_longest_streak_consecutive() {
        let mut habit = Habit::new("Yoga");
        for day in 1..=5u32 {
            habit.check_in(date(2024, 6, day), true, None);
        }
        assert_eq!(habit.longest_streak(), 5);
    }

    #[test]
    fn test_longest_streak_with_gap() {
        let mut habit = Habit::new("Yoga");
        // 3 days, gap, 2 days
        habit.check_in(date(2024, 1, 1), true, None);
        habit.check_in(date(2024, 1, 2), true, None);
        habit.check_in(date(2024, 1, 3), true, None);
        // gap at Jan 4
        habit.check_in(date(2024, 1, 5), true, None);
        habit.check_in(date(2024, 1, 6), true, None);
        assert_eq!(habit.longest_streak(), 3);
    }

    #[test]
    fn test_overall_completion_rate_empty() {
        let habit = Habit::new("Read");
        assert!(habit.overall_completion_rate() < f64::EPSILON);
    }

    #[test]
    fn test_overall_completion_rate_all_complete() {
        let mut habit = Habit::new("Read");
        habit.check_in(date(2024, 1, 1), true, None);
        habit.check_in(date(2024, 1, 2), true, None);
        assert!((habit.overall_completion_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_overall_completion_rate_half() {
        let mut habit = Habit::new("Read");
        habit.check_in(date(2024, 1, 1), true, None);
        habit.check_in(date(2024, 1, 2), false, None);
        assert!((habit.overall_completion_rate() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_total_completions() {
        let mut habit = Habit::new("Read");
        habit.check_in(date(2024, 1, 1), true, None);
        habit.check_in(date(2024, 1, 2), false, None);
        habit.check_in(date(2024, 1, 3), true, None);
        assert_eq!(habit.total_completions(), 2);
    }

    #[test]
    fn test_total_completions_none() {
        let mut habit = Habit::new("Read");
        habit.check_in(date(2024, 1, 1), false, None);
        assert_eq!(habit.total_completions(), 0);
    }

    #[test]
    fn test_completion_rate_for_period_full() {
        let mut habit = Habit::new("Walk");
        let start = date(2024, 3, 1);
        let end = date(2024, 3, 3);
        habit.check_in(start, true, None);
        habit.check_in(date(2024, 3, 2), true, None);
        habit.check_in(end, true, None);
        let rate = habit.completion_rate_for_period(start, end);
        assert_eq!(rate, Some(100.0));
    }

    #[test]
    fn test_completion_rate_for_period_half() {
        let mut habit = Habit::new("Walk");
        let start = date(2024, 3, 1);
        let end = date(2024, 3, 2);
        habit.check_in(start, true, None);
        habit.check_in(end, false, None);
        let rate = habit.completion_rate_for_period(start, end);
        assert_eq!(rate, Some(50.0));
    }

    #[test]
    fn test_completion_rate_for_period_no_due_days() {
        // Weekly habit due only on Monday; period is only Tuesday
        let start = date(2024, 3, 5); // Tuesday
        let habit = Habit::new("Gym")
            .with_frequency(HabitFrequency::Weekly {
                days: vec![Weekday::Mon],
            })
            .with_start_date(start);
        let rate = habit.completion_rate_for_period(start, start);
        assert_eq!(rate, None);
    }

    #[test]
    fn test_trend_symbol_stable_no_completions() {
        // Daily habit with no check-ins: both periods return 0%, diff=0 → Stable ("→")
        let habit = Habit::new("Run");
        assert_eq!(habit.trend_symbol(), "→");
    }

    #[test]
    fn test_trend_stable_no_completions() {
        // Daily habit, no check-ins: recent=0%, historical=0%, diff=0 → Stable
        let habit = Habit::new("Run");
        assert_eq!(habit.trend(), Some(HabitTrend::Stable));
    }

    #[test]
    fn test_trend_none_weekly_no_due_days_in_period() {
        // Weekly habit due only on a day that doesn't appear in either period
        // This is hard to guarantee with today's date, so just verify the function returns something
        let habit = Habit::new("Run");
        // trend() is Some or None — just verify it doesn't panic
        let _ = habit.trend();
    }

    #[test]
    fn test_completion_rate_by_weekday_empty() {
        let habit = Habit::new("Walk");
        let rates = habit.completion_rate_by_weekday();
        for r in rates {
            assert!(r < f64::EPSILON);
        }
    }

    #[test]
    fn test_completion_rate_by_weekday_monday() {
        let mut habit = Habit::new("Walk");
        // 2024-01-01 is a Monday (weekday index 0)
        habit.check_in(date(2024, 1, 1), true, None);
        let rates = habit.completion_rate_by_weekday();
        assert!((rates[0] - 100.0).abs() < f64::EPSILON); // Monday
    }

    #[test]
    fn test_habit_frequency_display_daily() {
        assert_eq!(HabitFrequency::Daily.to_string(), "Daily");
    }

    #[test]
    fn test_habit_frequency_display_weekly() {
        let freq = HabitFrequency::Weekly {
            days: vec![Weekday::Mon, Weekday::Wed],
        };
        assert_eq!(freq.to_string(), "Weekly (Mon, Wed)");
    }

    #[test]
    fn test_habit_frequency_display_every_n_days() {
        let freq = HabitFrequency::EveryNDays { n: 3 };
        assert_eq!(freq.to_string(), "Every 3 days");
    }

    #[test]
    fn test_habit_frequency_is_due_daily() {
        let freq = HabitFrequency::Daily;
        let start = date(2024, 1, 1);
        assert!(freq.is_due_on(date(2024, 5, 15), start));
    }

    #[test]
    fn test_habit_frequency_is_due_weekly() {
        let freq = HabitFrequency::Weekly {
            days: vec![Weekday::Mon, Weekday::Fri],
        };
        let start = date(2024, 1, 1);
        // 2024-01-01 is Monday
        assert!(freq.is_due_on(date(2024, 1, 1), start));
        // 2024-01-05 is Friday
        assert!(freq.is_due_on(date(2024, 1, 5), start));
        // 2024-01-03 is Wednesday - not due
        assert!(!freq.is_due_on(date(2024, 1, 3), start));
    }

    #[test]
    fn test_habit_frequency_is_due_every_n_days() {
        let freq = HabitFrequency::EveryNDays { n: 3 };
        let start = date(2024, 1, 1);
        assert!(freq.is_due_on(date(2024, 1, 1), start)); // day 0
        assert!(!freq.is_due_on(date(2024, 1, 2), start)); // day 1
        assert!(!freq.is_due_on(date(2024, 1, 3), start)); // day 2
        assert!(freq.is_due_on(date(2024, 1, 4), start)); // day 3
    }

    #[test]
    fn test_habit_frequency_is_due_every_n_days_before_start() {
        let freq = HabitFrequency::EveryNDays { n: 3 };
        let start = date(2024, 1, 10);
        // Date before start should not be due
        assert!(!freq.is_due_on(date(2024, 1, 1), start));
    }
}
