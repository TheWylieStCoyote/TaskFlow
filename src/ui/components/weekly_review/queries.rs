//! Query methods for weekly review data.

use chrono::NaiveDate;

use crate::domain::{Project, ProjectId, Task};

use super::WeeklyReview;

impl WeeklyReview<'_> {
    pub(crate) fn today() -> NaiveDate {
        chrono::Utc::now().date_naive()
    }

    pub(crate) fn week_ago() -> NaiveDate {
        Self::today() - chrono::Duration::days(7)
    }

    pub(crate) fn week_ahead() -> NaiveDate {
        Self::today() + chrono::Duration::days(7)
    }

    /// Get tasks completed in the past week
    pub(crate) fn completed_this_week(&self) -> Vec<&Task> {
        let week_ago = Self::week_ago();
        self.model
            .tasks
            .values()
            .filter(|t| {
                t.status.is_complete() && t.completed_at.is_some_and(|d| d.date_naive() >= week_ago)
            })
            .collect()
    }

    /// Get overdue tasks
    pub(crate) fn overdue_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
            .collect()
    }

    /// Get tasks due in the next 7 days
    pub(crate) fn upcoming_week_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        let week_ahead = Self::week_ahead();
        self.model
            .tasks
            .values()
            .filter(|t| {
                !t.status.is_complete() && t.due_date.is_some_and(|d| d >= today && d <= week_ahead)
            })
            .collect()
    }

    /// Get projects with no recent activity (stale)
    pub(crate) fn stale_projects(&self) -> Vec<(&ProjectId, &Project, usize)> {
        let week_ago = Self::week_ago();

        self.model
            .projects
            .iter()
            .filter_map(|(id, project)| {
                // Count incomplete tasks in this project
                let task_count = self
                    .model
                    .tasks
                    .values()
                    .filter(|t| t.project_id.as_ref() == Some(id) && !t.status.is_complete())
                    .count();

                // Check if any task was modified in the past week
                let has_recent_activity = self.model.tasks.values().any(|t| {
                    t.project_id.as_ref() == Some(id) && t.updated_at.date_naive() >= week_ago
                });

                // Stale if has tasks but no recent activity
                if task_count > 0 && !has_recent_activity {
                    Some((id, project, task_count))
                } else {
                    None
                }
            })
            .collect()
    }
}
