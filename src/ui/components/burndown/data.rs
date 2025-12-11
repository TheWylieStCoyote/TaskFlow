//! Burndown data structures and queries.

use chrono::{Duration, Local, NaiveDate};

use crate::app::BurndownMode;
use crate::domain::ProjectId;

use super::Burndown;

/// Daily data point for burndown chart
#[derive(Debug, Clone)]
pub struct DailyPoint {
    /// The date
    pub date: NaiveDate,
    /// Remaining value (tasks or minutes)
    pub remaining: f64,
    /// Tasks/hours added on this day (scope creep)
    pub added: f64,
}

/// Data structure for burndown chart
pub struct BurndownData {
    /// Total tasks or estimated minutes at start
    pub total: f64,
    /// Completed tasks or minutes tracked
    pub completed: f64,
    /// Remaining tasks or estimated minutes
    pub remaining: f64,
    /// Daily data points with scope creep tracking
    pub daily_points: Vec<DailyPoint>,
    /// Start date of the burndown period
    pub start_date: NaiveDate,
    /// End date (latest due date or today + 14 days)
    pub end_date: NaiveDate,
    /// Total scope added during the period (tasks or minutes)
    pub scope_added: f64,
    /// Display mode used for this data
    pub mode: BurndownMode,
    /// Number of days in the window
    pub window_days: i64,
}

impl Burndown<'_> {
    /// Get burndown data for a project or all tasks
    pub(crate) fn get_burndown_data(&self, project_id: Option<ProjectId>) -> BurndownData {
        let tasks: Vec<_> = self
            .model
            .tasks
            .values()
            .filter(|t| project_id.is_none() || t.project_id == project_id)
            .collect();

        let mode = self.model.burndown_state.mode;
        let window_days = self.model.burndown_state.time_window.days();
        let today = Local::now().date_naive();
        let period_start = today - Duration::days(window_days - 1);

        // Calculate totals based on mode
        let (total, completed, remaining) = match mode {
            BurndownMode::TaskCount => {
                let total = tasks.len() as f64;
                let completed = tasks.iter().filter(|t| t.status.is_complete()).count() as f64;
                (total, completed, total - completed)
            }
            BurndownMode::TimeHours => {
                let total_minutes: u32 =
                    tasks.iter().map(|t| t.estimated_minutes.unwrap_or(0)).sum();
                let completed_minutes: u32 = tasks
                    .iter()
                    .filter(|t| t.status.is_complete())
                    .map(|t| t.estimated_minutes.unwrap_or(0))
                    .sum();
                let total = f64::from(total_minutes) / 60.0;
                let completed = f64::from(completed_minutes) / 60.0;
                (total, completed, total - completed)
            }
        };

        // Build daily data points with scope creep tracking
        let mut daily_points: Vec<DailyPoint> = Vec::new();
        let mut scope_added = 0.0;

        for days_ago in (0..window_days).rev() {
            let date = today - Duration::days(days_ago);

            // Calculate remaining at end of this day
            let remaining_on_date = match mode {
                BurndownMode::TaskCount => {
                    let completed_by_date = tasks
                        .iter()
                        .filter(|t| {
                            t.status.is_complete()
                                && t.completed_at.is_some_and(|c| c.date_naive() <= date)
                        })
                        .count();
                    (tasks.len() - completed_by_date) as f64
                }
                BurndownMode::TimeHours => {
                    let completed_minutes: u32 = tasks
                        .iter()
                        .filter(|t| {
                            t.status.is_complete()
                                && t.completed_at.is_some_and(|c| c.date_naive() <= date)
                        })
                        .map(|t| t.estimated_minutes.unwrap_or(0))
                        .sum();
                    let total_minutes: u32 =
                        tasks.iter().map(|t| t.estimated_minutes.unwrap_or(0)).sum();
                    f64::from(total_minutes.saturating_sub(completed_minutes)) / 60.0
                }
            };

            // Calculate scope added on this day (tasks created during the period)
            let added_on_date = match mode {
                BurndownMode::TaskCount => tasks
                    .iter()
                    .filter(|t| t.created_at.date_naive() == date && date >= period_start)
                    .count() as f64,
                BurndownMode::TimeHours => {
                    let added_minutes: u32 = tasks
                        .iter()
                        .filter(|t| t.created_at.date_naive() == date && date >= period_start)
                        .map(|t| t.estimated_minutes.unwrap_or(0))
                        .sum();
                    f64::from(added_minutes) / 60.0
                }
            };

            scope_added += added_on_date;

            daily_points.push(DailyPoint {
                date,
                remaining: remaining_on_date,
                added: added_on_date,
            });
        }

        // Find earliest task and latest due date for scope
        let start_date = tasks
            .iter()
            .map(|t| t.created_at.date_naive())
            .min()
            .unwrap_or(period_start);

        let end_date = tasks
            .iter()
            .filter_map(|t| t.due_date)
            .max()
            .unwrap_or(today + Duration::days(14));

        BurndownData {
            total,
            completed,
            remaining,
            daily_points,
            start_date,
            end_date,
            scope_added,
            mode,
            window_days,
        }
    }
}
