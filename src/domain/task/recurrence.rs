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

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_monthly_edge_of_month() {
        // Test edge cases for monthly day values
        let day1 = Recurrence::Monthly { day: 1 };
        assert_eq!(day1.to_string(), "Monthly (day 1)");

        let day31 = Recurrence::Monthly { day: 31 };
        assert_eq!(day31.to_string(), "Monthly (day 31)");
    }

    #[test]
    fn test_monthly_february_edge_case() {
        // Day 29, 30, 31 for February - domain layer doesn't validate
        let day29 = Recurrence::Monthly { day: 29 };
        let day30 = Recurrence::Monthly { day: 30 };
        let day31 = Recurrence::Monthly { day: 31 };

        assert_eq!(day29.to_string(), "Monthly (day 29)");
        assert_eq!(day30.to_string(), "Monthly (day 30)");
        assert_eq!(day31.to_string(), "Monthly (day 31)");
    }

    #[test]
    fn test_yearly_leap_year_date() {
        // Feb 29 for yearly recurrence
        let leap_day = Recurrence::Yearly { month: 2, day: 29 };
        assert_eq!(leap_day.to_string(), "Yearly (2/29)");
    }

    #[test]
    fn test_yearly_invalid_dates() {
        // Domain layer doesn't validate - accepts any values
        let invalid1 = Recurrence::Yearly { month: 2, day: 30 }; // Feb 30 doesn't exist
        let invalid2 = Recurrence::Yearly { month: 13, day: 1 }; // Month 13 doesn't exist
        let invalid3 = Recurrence::Yearly { month: 4, day: 31 }; // April 31 doesn't exist

        assert_eq!(invalid1.to_string(), "Yearly (2/30)");
        assert_eq!(invalid2.to_string(), "Yearly (13/1)");
        assert_eq!(invalid3.to_string(), "Yearly (4/31)");
    }

    #[test]
    fn test_yearly_boundary_dates() {
        // First and last day of year
        let new_year = Recurrence::Yearly { month: 1, day: 1 };
        assert_eq!(new_year.to_string(), "Yearly (1/1)");

        let new_years_eve = Recurrence::Yearly { month: 12, day: 31 };
        assert_eq!(new_years_eve.to_string(), "Yearly (12/31)");
    }

    // ========================================================================
    // Weekly Recurrence Edge Cases
    // ========================================================================

    #[test]
    fn test_weekly_empty_days() {
        // Weekly with no days - technically valid in domain layer
        let weekly = Recurrence::Weekly { days: vec![] };
        assert_eq!(weekly.to_string(), "Weekly ()");
    }

    #[test]
    fn test_weekly_weekdays_only() {
        let weekdays = Recurrence::Weekly {
            days: vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
            ],
        };
        assert_eq!(weekdays.to_string(), "Weekly (Mon, Tue, Wed, Thu, Fri)");
    }

    #[test]
    fn test_weekly_weekends_only() {
        let weekends = Recurrence::Weekly {
            days: vec![Weekday::Sat, Weekday::Sun],
        };
        assert_eq!(weekends.to_string(), "Weekly (Sat, Sun)");
    }

    #[test]
    fn test_weekly_duplicate_days() {
        // Domain layer allows duplicates
        let duplicates = Recurrence::Weekly {
            days: vec![Weekday::Mon, Weekday::Mon, Weekday::Tue],
        };
        assert_eq!(duplicates.to_string(), "Weekly (Mon, Mon, Tue)");
    }

    #[test]
    fn test_weekly_unordered_days() {
        let unordered = Recurrence::Weekly {
            days: vec![Weekday::Fri, Weekday::Mon, Weekday::Wed],
        };
        // Displays in the order given, doesn't sort
        assert_eq!(unordered.to_string(), "Weekly (Fri, Mon, Wed)");
    }

    // ========================================================================
    // Cloning and Equality
    // ========================================================================

    #[test]
    fn test_recurrence_clone() {
        let original = Recurrence::Weekly {
            days: vec![Weekday::Mon, Weekday::Wed],
        };
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_recurrence_clone_independence() {
        let original = Recurrence::Weekly {
            days: vec![Weekday::Mon],
        };
        let cloned = original.clone();

        // Create a different instance to verify independence
        let modified = Recurrence::Weekly {
            days: vec![Weekday::Tue],
        };

        assert_eq!(original, cloned);
        assert_ne!(original, modified);
    }

    #[test]
    fn test_recurrence_different_types_not_equal() {
        let daily = Recurrence::Daily;
        let weekly = Recurrence::Weekly {
            days: vec![Weekday::Mon],
        };
        let monthly = Recurrence::Monthly { day: 1 };
        let yearly = Recurrence::Yearly { month: 1, day: 1 };

        assert_ne!(daily, weekly);
        assert_ne!(daily, monthly);
        assert_ne!(daily, yearly);
        assert_ne!(weekly, monthly);
        assert_ne!(weekly, yearly);
        assert_ne!(monthly, yearly);
    }

    // ========================================================================
    // Serialization Edge Cases
    // ========================================================================

    #[test]
    fn test_recurrence_serialization_format() {
        let daily = Recurrence::Daily;
        let json = serde_json::to_string(&daily).unwrap();
        assert!(json.contains("\"type\":\"daily\""));
    }

    #[test]
    fn test_weekly_serialization_with_days() {
        let weekly = Recurrence::Weekly {
            days: vec![Weekday::Mon, Weekday::Fri],
        };
        let json = serde_json::to_string(&weekly).unwrap();
        assert!(json.contains("\"type\":\"weekly\""));
        assert!(json.contains("\"days\""));
    }

    #[test]
    fn test_weekly_serialization_empty_days() {
        let weekly = Recurrence::Weekly { days: vec![] };
        let json = serde_json::to_string(&weekly).unwrap();
        let restored: Recurrence = serde_json::from_str(&json).unwrap();
        assert_eq!(weekly, restored);
    }

    #[test]
    fn test_monthly_serialization_boundary_values() {
        let month_start = Recurrence::Monthly { day: 1 };
        let month_end = Recurrence::Monthly { day: 31 };

        let json1 = serde_json::to_string(&month_start).unwrap();
        let json2 = serde_json::to_string(&month_end).unwrap();

        let restored1: Recurrence = serde_json::from_str(&json1).unwrap();
        let restored2: Recurrence = serde_json::from_str(&json2).unwrap();

        assert_eq!(month_start, restored1);
        assert_eq!(month_end, restored2);
    }

    #[test]
    fn test_yearly_serialization_all_months() {
        // Test a few different months
        for month in [1, 2, 6, 11, 12] {
            let yearly = Recurrence::Yearly { month, day: 15 };
            let json = serde_json::to_string(&yearly).unwrap();
            let restored: Recurrence = serde_json::from_str(&json).unwrap();
            assert_eq!(yearly, restored);
        }
    }

    // ========================================================================
    // Display Tests
    // ========================================================================

    #[test]
    fn test_display_does_not_panic() {
        // Ensure display doesn't panic on edge cases
        let patterns = vec![
            Recurrence::Daily,
            Recurrence::Weekly { days: vec![] },
            Recurrence::Weekly {
                days: vec![Weekday::Mon],
            },
            Recurrence::Weekly {
                days: vec![
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                    Weekday::Sat,
                    Weekday::Sun,
                ],
            },
            Recurrence::Monthly { day: 1 },
            Recurrence::Monthly { day: 31 },
            Recurrence::Yearly { month: 1, day: 1 },
            Recurrence::Yearly { month: 12, day: 31 },
        ];

        for pattern in patterns {
            let _ = pattern.to_string(); // Should not panic
        }
    }

    #[test]
    fn test_debug_format() {
        let daily = Recurrence::Daily;
        let debug_str = format!("{daily:?}");
        assert!(debug_str.contains("Daily"));

        let weekly = Recurrence::Weekly {
            days: vec![Weekday::Mon],
        };
        let debug_str = format!("{weekly:?}");
        assert!(debug_str.contains("Weekly"));
        assert!(debug_str.contains("Mon"));
    }
}
