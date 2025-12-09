//! Task recurrence patterns.

use serde::{Deserialize, Serialize};

/// Recurrence pattern for repeating tasks.
///
/// When a recurring task is completed, a new instance is automatically
/// created based on the recurrence pattern.
///
/// # Examples
///
/// ```
/// use taskflow::domain::Recurrence;
/// use chrono::Weekday;
///
/// // Daily tasks (e.g., standup)
/// let daily = Recurrence::Daily;
///
/// // Weekly on specific days (e.g., team sync on Mon/Wed/Fri)
/// let weekly = Recurrence::Weekly {
///     days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]
/// };
///
/// // Monthly on a specific day (e.g., monthly report on the 15th)
/// let monthly = Recurrence::Monthly { day: 15 };
///
/// // Yearly (e.g., annual review on March 1st)
/// let yearly = Recurrence::Yearly { month: 3, day: 1 };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Recurrence {
    /// Repeats every day
    Daily,
    /// Repeats on specific days of the week
    Weekly {
        /// Days of the week when the task recurs
        days: Vec<chrono::Weekday>,
    },
    /// Repeats on a specific day each month
    Monthly {
        /// Day of the month (1-31)
        day: u32,
    },
    /// Repeats on a specific date each year
    Yearly {
        /// Month (1-12)
        month: u32,
        /// Day of the month (1-31)
        day: u32,
    },
}

impl std::fmt::Display for Recurrence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Daily => write!(f, "Daily"),
            Self::Weekly { days } => {
                let day_names: Vec<&str> = days
                    .iter()
                    .map(|d| match d {
                        chrono::Weekday::Mon => "Mon",
                        chrono::Weekday::Tue => "Tue",
                        chrono::Weekday::Wed => "Wed",
                        chrono::Weekday::Thu => "Thu",
                        chrono::Weekday::Fri => "Fri",
                        chrono::Weekday::Sat => "Sat",
                        chrono::Weekday::Sun => "Sun",
                    })
                    .collect();
                write!(f, "Weekly ({})", day_names.join(", "))
            }
            Self::Monthly { day } => write!(f, "Monthly (day {day})"),
            Self::Yearly { month, day } => write!(f, "Yearly ({month}/{day})"),
        }
    }
}
