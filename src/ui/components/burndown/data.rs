//! Burndown data structures and queries.

use chrono::{Duration, Local, NaiveDate};

use crate::domain::ProjectId;

use super::Burndown;

/// Data structure for burndown chart
pub struct BurndownData {
    pub total: usize,
    pub completed: usize,
    pub remaining: usize,
    pub daily_completions: Vec<(NaiveDate, usize)>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
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

        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.status.is_complete()).count();
        let remaining = total - completed;

        // Get completion history for the last 14 days
        let today = Local::now().date_naive();
        let mut daily_completions: Vec<(NaiveDate, usize)> = Vec::new();

        for days_ago in (0..14).rev() {
            let date = today - Duration::days(days_ago);
            let completed_by_date = tasks
                .iter()
                .filter(|t| {
                    t.status.is_complete() && t.completed_at.is_some_and(|c| c.date_naive() <= date)
                })
                .count();
            daily_completions.push((date, total - completed_by_date));
        }

        // Find earliest task and latest due date for scope
        let start_date = tasks
            .iter()
            .map(|t| t.created_at.date_naive())
            .min()
            .unwrap_or(today - Duration::days(14));

        let end_date = tasks
            .iter()
            .filter_map(|t| t.due_date)
            .max()
            .unwrap_or(today + Duration::days(14));

        BurndownData {
            total,
            completed,
            remaining,
            daily_completions,
            start_date,
            end_date,
        }
    }
}
