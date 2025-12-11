//! Utility methods for focus view.

use std::time::Duration;

use crate::domain::Task;

use super::FocusView;

impl FocusView<'_> {
    /// Format a duration as HH:MM:SS
    pub(crate) fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }

    /// Get time tracked for a task including current session
    pub(crate) fn get_time_tracked(&self, task: &Task) -> Duration {
        let mut total_minutes: u64 = 0;

        // Sum completed time entries
        for entry in self.model.time_entries.values() {
            if entry.task_id == task.id {
                total_minutes += u64::from(entry.calculated_duration_minutes());
            }
        }

        // Add current active session if tracking this task
        if let Some(active) = self.model.active_time_entry() {
            if active.task_id == task.id {
                let now = chrono::Utc::now();
                let elapsed_secs = (now - active.started_at).num_seconds().max(0) as u64;
                return Duration::from_secs(total_minutes * 60 + elapsed_secs);
            }
        }

        Duration::from_secs(total_minutes * 60)
    }

    /// Get the previous task in a chain (the task that links to this one)
    pub(crate) fn get_prev_task_in_chain(
        &self,
        task_id: crate::domain::TaskId,
    ) -> Option<crate::domain::TaskId> {
        self.model
            .tasks
            .values()
            .find(|t| t.next_task_id == Some(task_id))
            .map(|t| t.id)
    }
}
