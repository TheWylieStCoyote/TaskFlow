//! Query methods for evening review data.

use chrono::{Duration, NaiveDate, Utc};

use crate::domain::{Task, TimeEntry};

use super::EveningReview;

impl EveningReview<'_> {
    /// Get today's date.
    pub(crate) fn today() -> NaiveDate {
        Utc::now().date_naive()
    }

    /// Get tomorrow's date.
    pub(crate) fn tomorrow() -> NaiveDate {
        Self::today() + Duration::days(1)
    }

    /// Get tasks completed today.
    ///
    /// Returns tasks where completed_at date equals today.
    pub(crate) fn completed_today(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| t.completed_at.is_some_and(|c| c.date_naive() == today))
            .collect()
    }

    /// Get incomplete tasks that were due today.
    ///
    /// These are tasks that should have been done but weren't.
    #[allow(dead_code)]
    pub(crate) fn incomplete_due_today(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.due_date == Some(today))
            .collect()
    }

    /// Get incomplete tasks that were scheduled for today.
    ///
    /// Tasks that were planned for today but not completed.
    /// Excludes tasks that are also due today (to avoid duplicates).
    #[allow(dead_code)]
    pub(crate) fn incomplete_scheduled_today(&self) -> Vec<&Task> {
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

    /// Get all incomplete tasks from today (due or scheduled).
    ///
    /// Combined list for the "Unfinished Business" phase.
    pub(crate) fn all_incomplete_today(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| {
                !t.status.is_complete()
                    && (t.due_date == Some(today) || t.scheduled_date == Some(today))
            })
            .collect()
    }

    /// Get tasks due or scheduled for tomorrow.
    ///
    /// Preview of what's coming up next.
    pub(crate) fn tomorrow_tasks(&self) -> Vec<&Task> {
        let tomorrow = Self::tomorrow();
        self.model
            .tasks
            .values()
            .filter(|t| {
                !t.status.is_complete()
                    && (t.due_date == Some(tomorrow) || t.scheduled_date == Some(tomorrow))
            })
            .collect()
    }

    /// Get time entries for today.
    ///
    /// Returns entries where started_at date equals today.
    pub(crate) fn time_entries_today(&self) -> Vec<&TimeEntry> {
        let today = Self::today();
        self.model
            .time_entries
            .values()
            .filter(|e| e.started_at.date_naive() == today)
            .collect()
    }

    /// Get total time tracked today in minutes.
    pub(crate) fn total_time_today(&self) -> u32 {
        self.time_entries_today()
            .iter()
            .filter_map(|e| e.duration_minutes)
            .sum()
    }

    /// Check if there are any time entries for today.
    ///
    /// Used to determine whether to auto-skip the TimeReview phase.
    #[allow(dead_code)]
    pub(crate) fn has_time_entries_today(&self) -> bool {
        let today = Self::today();
        self.model
            .time_entries
            .values()
            .any(|e| e.started_at.date_naive() == today)
    }

    /// Get the currently selected task ID for the incomplete tasks phase.
    ///
    /// Returns None if no tasks or selection out of bounds.
    #[allow(dead_code)]
    pub(crate) fn selected_incomplete_task_id(&self) -> Option<crate::domain::TaskId> {
        let tasks = self.all_incomplete_today();
        tasks.get(self.selected).map(|t| t.id)
    }

    /// Calculate completion rate for today.
    ///
    /// Returns the percentage of tasks completed out of total tasks
    /// that were due or scheduled for today.
    pub(crate) fn today_completion_rate(&self) -> f64 {
        let completed = self.completed_today().len();
        let incomplete = self.all_incomplete_today().len();
        let total = completed + incomplete;
        if total == 0 {
            100.0
        } else {
            (completed as f64 / total as f64) * 100.0
        }
    }
}
