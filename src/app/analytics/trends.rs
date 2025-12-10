//! Completion trend and velocity calculations.
//!
//! This module provides methods for computing:
//! - Completion trends over time (tasks created vs completed)
//! - Velocity metrics (weekly and monthly task completion rates)

use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;

use crate::domain::analytics::{CompletionTrend, TimeSeriesPoint, VelocityMetrics};

use super::AnalyticsEngine;

impl AnalyticsEngine<'_> {
    /// Compute completion trends over a date range.
    ///
    /// Tracks the number of tasks created and completed each day,
    /// along with the cumulative completion rate over time.
    #[must_use]
    pub fn compute_completion_trend(&self, start: NaiveDate, end: NaiveDate) -> CompletionTrend {
        let mut completions_by_day: HashMap<NaiveDate, u32> = HashMap::new();
        let mut creations_by_day: HashMap<NaiveDate, u32> = HashMap::new();

        for task in self.model.tasks.values() {
            // Count creations
            let created_date = task.created_at.date_naive();
            if created_date >= start && created_date <= end {
                *creations_by_day.entry(created_date).or_insert(0) += 1;
            }

            // Count completions
            if let Some(completed_at) = task.completed_at {
                let completed_date = completed_at.date_naive();
                if completed_date >= start && completed_date <= end {
                    *completions_by_day.entry(completed_date).or_insert(0) += 1;
                }
            }
        }

        // Convert to sorted time series
        let mut completions: Vec<TimeSeriesPoint> = completions_by_day
            .into_iter()
            .map(|(date, count)| TimeSeriesPoint::new(date, f64::from(count)))
            .collect();
        completions.sort_by_key(|p| p.date);

        let mut creations: Vec<TimeSeriesPoint> = creations_by_day
            .into_iter()
            .map(|(date, count)| TimeSeriesPoint::new(date, f64::from(count)))
            .collect();
        creations.sort_by_key(|p| p.date);

        // Calculate completion rate over time
        let mut completion_rate = Vec::new();
        let mut current_date = start;
        let mut total_created = 0u32;
        let mut total_completed = 0u32;

        while current_date <= end {
            // Add creations for this day
            total_created += creations
                .iter()
                .find(|p| p.date == current_date)
                .map_or(0, |p| p.value as u32);

            // Add completions for this day
            total_completed += completions
                .iter()
                .find(|p| p.date == current_date)
                .map_or(0, |p| p.value as u32);

            // Calculate rate
            let rate = if total_created > 0 {
                f64::from(total_completed) / f64::from(total_created)
            } else {
                0.0
            };
            completion_rate.push(TimeSeriesPoint::new(current_date, rate));

            current_date = current_date.succ_opt().unwrap_or(current_date);
        }

        CompletionTrend {
            completions_by_day: completions,
            creations_by_day: creations,
            completion_rate_over_time: completion_rate,
        }
    }

    /// Compute velocity metrics.
    ///
    /// Calculates weekly and monthly task completion counts,
    /// average weekly velocity, and trend direction.
    #[must_use]
    pub fn compute_velocity(&self, start: NaiveDate, end: NaiveDate) -> VelocityMetrics {
        // Group completions by week (start of week = Monday)
        let mut weekly: HashMap<NaiveDate, u32> = HashMap::new();
        let mut monthly: HashMap<NaiveDate, u32> = HashMap::new();

        for task in self.model.tasks.values() {
            if let Some(completed_at) = task.completed_at {
                let completed_date = completed_at.date_naive();
                if completed_date >= start && completed_date <= end {
                    // Get start of week (Monday)
                    let week_start = completed_date
                        - chrono::Duration::days(i64::from(
                            completed_date.weekday().num_days_from_monday(),
                        ));
                    *weekly.entry(week_start).or_insert(0) += 1;

                    // Get start of month
                    let month_start =
                        NaiveDate::from_ymd_opt(completed_date.year(), completed_date.month(), 1)
                            .expect("day 1 of any month always exists");
                    *monthly.entry(month_start).or_insert(0) += 1;
                }
            }
        }

        let mut weekly_vec: Vec<(NaiveDate, u32)> = weekly.into_iter().collect();
        weekly_vec.sort_by_key(|(date, _)| *date);

        let mut monthly_vec: Vec<(NaiveDate, u32)> = monthly.into_iter().collect();
        monthly_vec.sort_by_key(|(date, _)| *date);

        // Calculate average weekly velocity
        let avg_weekly = if weekly_vec.is_empty() {
            0.0
        } else {
            let sum: u32 = weekly_vec.iter().map(|(_, v)| v).sum();
            f64::from(sum) / weekly_vec.len() as f64
        };

        // Calculate trend (simple linear regression slope)
        let trend = if weekly_vec.len() < 2 {
            0.0
        } else {
            let n = weekly_vec.len() as f64;
            let sum_x: f64 = (0..weekly_vec.len()).map(|i| i as f64).sum();
            let sum_y: f64 = weekly_vec.iter().map(|(_, v)| f64::from(*v)).sum();
            let sum_xy: f64 = weekly_vec
                .iter()
                .enumerate()
                .map(|(i, (_, v))| i as f64 * f64::from(*v))
                .sum();
            let sum_xx: f64 = (0..weekly_vec.len()).map(|i| (i * i) as f64).sum();

            let denominator = n * sum_xx - sum_x * sum_x;
            if denominator.abs() < f64::EPSILON {
                0.0
            } else {
                (n * sum_xy - sum_x * sum_y) / denominator
            }
        };

        VelocityMetrics {
            weekly_velocity: weekly_vec,
            monthly_velocity: monthly_vec,
            avg_weekly,
            trend,
        }
    }
}
