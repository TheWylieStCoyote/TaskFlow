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

mod core;
mod types;

pub use core::Habit;
pub use types::{HabitCheckIn, HabitFrequency, HabitId, HabitTrend};

#[cfg(test)]
mod tests;
