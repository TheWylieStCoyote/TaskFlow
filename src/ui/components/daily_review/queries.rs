//! Query methods for daily review data.

use chrono::{NaiveDate, Utc};

use crate::domain::Task;

use super::DailyReview;

impl DailyReview<'_> {
    pub(crate) fn today() -> NaiveDate {
        Utc::now().date_naive()
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

    /// Get tasks due today
    pub(crate) fn today_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.due_date == Some(today))
            .collect()
    }

    /// Get tasks scheduled for today (but not due today)
    pub(crate) fn scheduled_today_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| {
                !t.status.is_complete()
                    && t.scheduled_date == Some(today)
                    && t.due_date != Some(today)
            })
            .collect()
    }
}
