//! Productivity insights calculations.
//!
//! This module computes productivity insights including:
//! - Current and longest completion streaks
//! - Best day of week for task completion
//! - Peak productive hour
//! - Average tasks per day
//! - Total time tracked

use chrono::{Datelike, NaiveDate, Timelike};
use std::collections::HashSet;

use crate::domain::analytics::ProductivityInsights;
use crate::domain::TaskStatus;

use super::{AnalyticsEngine, WEEKDAYS};

impl AnalyticsEngine<'_> {
    /// Compute productivity insights.
    ///
    /// Analyzes task completion patterns to identify:
    /// - Current and longest streaks of consecutive productive days
    /// - Best day of the week for completing tasks
    /// - Peak productive hour of the day
    /// - Average tasks completed per active day
    /// - Total time tracked across all tasks
    #[must_use]
    pub fn compute_insights(&self) -> ProductivityInsights {
        let mut insights = ProductivityInsights::default();

        // Count completions by day of week and hour
        let mut by_day: [u32; 7] = [0; 7];
        let mut by_hour: [u32; 24] = [0; 24];
        let mut completion_dates: Vec<NaiveDate> = Vec::new();

        for task in self.model.tasks.values() {
            if task.status == TaskStatus::Done {
                insights.total_completed += 1;

                if let Some(completed_at) = task.completed_at {
                    let date = completed_at.date_naive();
                    completion_dates.push(date);

                    let dow = date.weekday().num_days_from_monday() as usize;
                    by_day[dow] += 1;

                    let hour = completed_at.time().hour() as usize;
                    by_hour[hour] += 1;
                }
            }

            insights.total_time_tracked += task.actual_minutes;
        }

        // Add time from time entries
        for entry in self.model.time_entries.values() {
            insights.total_time_tracked += entry.calculated_duration_minutes();
        }

        // Find best day
        let max_day_idx = by_day
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .filter(|(_, &v)| v > 0)
            .map(|(i, _)| i);

        insights.best_day = max_day_idx.map(|i| WEEKDAYS[i]);

        // Find peak hour
        insights.peak_hour = by_hour
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .filter(|(_, &v)| v > 0)
            .map(|(i, _)| i as u32);

        // Calculate streaks
        if !completion_dates.is_empty() {
            completion_dates.sort();
            let unique_dates: Vec<NaiveDate> = completion_dates
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .into_iter()
                .collect();
            let mut sorted_unique: Vec<NaiveDate> = unique_dates;
            sorted_unique.sort();

            let today = chrono::Local::now().date_naive();
            let mut current_streak = 0u32;
            let mut longest_streak = 0u32;
            let mut streak = 0u32;
            let mut prev_date: Option<NaiveDate> = None;

            for date in &sorted_unique {
                if let Some(prev) = prev_date {
                    if *date - prev == chrono::Duration::days(1) {
                        streak += 1;
                    } else {
                        longest_streak = longest_streak.max(streak);
                        streak = 1;
                    }
                } else {
                    streak = 1;
                }
                prev_date = Some(*date);
            }
            longest_streak = longest_streak.max(streak);

            // Check if current streak is still active
            if let Some(last_date) = sorted_unique.last() {
                let days_since = (today - *last_date).num_days();
                if days_since <= 1 {
                    // Still active (today or yesterday)
                    current_streak = streak;
                }
            }

            insights.current_streak = current_streak;
            insights.longest_streak = longest_streak;

            // Average tasks per day (on active days)
            let active_days = sorted_unique.len();
            if active_days > 0 {
                insights.avg_tasks_per_day =
                    f64::from(insights.total_completed) / active_days as f64;
            }
        }

        insights
    }
}
