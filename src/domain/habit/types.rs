//! Habit-related types and identifiers.

use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};
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
