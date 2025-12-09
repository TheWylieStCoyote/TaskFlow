//! Habit tracking types.
//!
//! This module contains types for tracking daily habits with streaks
//! and completion analytics.
//!
//! # Examples
//!
//! ## Creating a Daily Habit
//!
//! ```
//! use taskflow::domain::{Habit, HabitFrequency};
//!
//! let habit = Habit::new("Exercise")
//!     .with_description("30 minutes of cardio");
//!
//! assert!(habit.is_due_today());
//! assert_eq!(habit.current_streak(), 0);
//! ```
//!
//! ## Weekly Habits
//!
//! ```
//! use taskflow::domain::{Habit, HabitFrequency};
//! use chrono::Weekday;
//!
//! let habit = Habit::new("Team standup")
//!     .with_frequency(HabitFrequency::Weekly {
//!         days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]
//!     });
//! ```
//!
//! ## Tracking Check-ins
//!
//! ```
//! use taskflow::domain::Habit;
//! use chrono::Utc;
//!
//! let mut habit = Habit::new("Read");
//! habit.check_in_today(true, None);
//!
//! let today = Utc::now().date_naive();
//! assert!(habit.is_completed_on(today));
//! ```

use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for habits.
///
/// Each habit has a UUID-based identifier that remains stable across
/// serialization and storage operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HabitId(pub Uuid);

impl HabitId {
    /// Creates a new unique habit identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for HabitId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HabitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// When a habit should repeat.
///
/// Habits can repeat daily, on specific days of the week, or every N days.
///
/// # Examples
///
/// ```
/// use taskflow::domain::HabitFrequency;
/// use chrono::Weekday;
///
/// // Every day
/// let daily = HabitFrequency::Daily;
///
/// // Monday, Wednesday, Friday
/// let mwf = HabitFrequency::Weekly {
///     days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]
/// };
///
/// // Every 3 days
/// let every_3 = HabitFrequency::EveryNDays { n: 3 };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HabitFrequency {
    /// Repeats every day.
    #[default]
    Daily,
    /// Repeats on specific days of the week.
    Weekly {
        /// Days of the week when the habit is due.
        days: Vec<Weekday>,
    },
    /// Repeats every N days from the habit's start date.
    EveryNDays {
        /// Number of days between repetitions.
        n: u32,
    },
}

impl HabitFrequency {
    /// Check if the habit is due on a given date.
    ///
    /// For `EveryNDays`, uses the habit's start date to calculate
    /// which days the habit falls on.
    #[must_use]
    pub fn is_due_on(&self, date: NaiveDate, habit_start: NaiveDate) -> bool {
        match self {
            Self::Daily => true,
            Self::Weekly { days } => days.contains(&date.weekday()),
            Self::EveryNDays { n } => {
                let days_since_start = (date - habit_start).num_days();
                days_since_start >= 0 && days_since_start % i64::from(*n) == 0
            }
        }
    }
}

impl std::fmt::Display for HabitFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Daily => write!(f, "Daily"),
            Self::Weekly { days } => {
                let day_names: Vec<&str> = days
                    .iter()
                    .map(|d| match d {
                        Weekday::Mon => "Mon",
                        Weekday::Tue => "Tue",
                        Weekday::Wed => "Wed",
                        Weekday::Thu => "Thu",
                        Weekday::Fri => "Fri",
                        Weekday::Sat => "Sat",
                        Weekday::Sun => "Sun",
                    })
                    .collect();
                write!(f, "Weekly ({})", day_names.join(", "))
            }
            Self::EveryNDays { n } => write!(f, "Every {n} days"),
        }
    }
}

/// A single check-in entry for a habit.
///
/// Records whether the habit was completed on a specific date,
/// along with an optional note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitCheckIn {
    /// The date of the check-in.
    pub date: NaiveDate,
    /// Whether the habit was completed.
    pub completed: bool,
    /// Optional note about this check-in.
    pub note: Option<String>,
    /// When the check-in was recorded.
    pub checked_at: DateTime<Utc>,
}

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
            Some(completed_days as f64 / due_days as f64 * 100.0)
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

/// Trend direction for habit performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HabitTrend {
    /// Performance is improving (recent > historical)
    Improving,
    /// Performance is declining (recent < historical)
    Declining,
    /// Performance is stable (within 10%)
    Stable,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(rates[0], 100.0); // Monday: 2/2 = 100%
        assert_eq!(rates[1], 0.0); // Tuesday: 0/1 = 0%
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
        let display = format!("{}", id);
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
}
