//! Habit tracking messages.

use crate::domain::HabitId;
use chrono::NaiveDate;

/// Habit tracking messages.
///
/// These messages handle creating, modifying, and checking in habits.
#[derive(Debug, Clone)]
pub enum HabitMessage {
    /// Create a new habit with the given name
    Create(String),
    /// Check in for today
    CheckInToday {
        /// The habit to check in
        habit_id: HabitId,
        /// Whether the habit was completed
        completed: bool,
    },
    /// Check in for a specific date
    CheckIn {
        /// The habit to check in
        habit_id: HabitId,
        /// The date to check in for
        date: NaiveDate,
        /// Whether the habit was completed
        completed: bool,
    },
    /// Toggle today's completion status
    ToggleToday(HabitId),
    /// Archive a habit
    Archive(HabitId),
    /// Unarchive a habit
    Unarchive(HabitId),
    /// Delete a habit
    Delete(HabitId),
    /// Update habit name
    UpdateName {
        /// The habit to update
        habit_id: HabitId,
        /// The new name
        name: String,
    },
}
