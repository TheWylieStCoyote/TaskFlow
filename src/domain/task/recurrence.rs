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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    #[test]
    fn test_daily_display() {
        let recurrence = Recurrence::Daily;
        assert_eq!(recurrence.to_string(), "Daily");
    }

    #[test]
    fn test_weekly_display_single_day() {
        let recurrence = Recurrence::Weekly {
            days: vec![Weekday::Mon],
        };
        assert_eq!(recurrence.to_string(), "Weekly (Mon)");
    }

    #[test]
    fn test_weekly_display_multiple_days() {
        let recurrence = Recurrence::Weekly {
            days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
        };
        assert_eq!(recurrence.to_string(), "Weekly (Mon, Wed, Fri)");
    }

    #[test]
    fn test_weekly_display_all_days() {
        let recurrence = Recurrence::Weekly {
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
        assert_eq!(
            recurrence.to_string(),
            "Weekly (Mon, Tue, Wed, Thu, Fri, Sat, Sun)"
        );
    }

    #[test]
    fn test_monthly_display() {
        let recurrence = Recurrence::Monthly { day: 15 };
        assert_eq!(recurrence.to_string(), "Monthly (day 15)");
    }

    #[test]
    fn test_yearly_display() {
        let recurrence = Recurrence::Yearly { month: 3, day: 1 };
        assert_eq!(recurrence.to_string(), "Yearly (3/1)");
    }

    #[test]
    fn test_recurrence_equality() {
        assert_eq!(Recurrence::Daily, Recurrence::Daily);
        assert_ne!(Recurrence::Daily, Recurrence::Monthly { day: 1 });

        let weekly1 = Recurrence::Weekly {
            days: vec![Weekday::Mon],
        };
        let weekly2 = Recurrence::Weekly {
            days: vec![Weekday::Mon],
        };
        let weekly3 = Recurrence::Weekly {
            days: vec![Weekday::Tue],
        };
        assert_eq!(weekly1, weekly2);
        assert_ne!(weekly1, weekly3);
    }

    #[test]
    fn test_recurrence_serialization() {
        let daily = Recurrence::Daily;
        let json = serde_json::to_string(&daily).expect("serialize");
        let restored: Recurrence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(daily, restored);

        let weekly = Recurrence::Weekly {
            days: vec![Weekday::Mon, Weekday::Fri],
        };
        let json = serde_json::to_string(&weekly).expect("serialize");
        let restored: Recurrence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(weekly, restored);

        let monthly = Recurrence::Monthly { day: 28 };
        let json = serde_json::to_string(&monthly).expect("serialize");
        let restored: Recurrence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(monthly, restored);

        let yearly = Recurrence::Yearly { month: 12, day: 25 };
        let json = serde_json::to_string(&yearly).expect("serialize");
        let restored: Recurrence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(yearly, restored);
    }
}
