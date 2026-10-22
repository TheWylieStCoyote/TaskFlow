//! Utility methods for focus view.

use std::time::Duration;

use crate::domain::Task;

use super::FocusView;

impl FocusView<'_> {
    /// Format a duration as HH:MM:SS
    pub(crate) fn format_duration(duration: Duration) -> String {
        crate::domain::format_hms(duration.as_secs())
    }

    /// Get time tracked for a task including current session
    pub(crate) fn get_time_tracked(&self, task: &Task) -> Duration {
        let now = chrono::Utc::now();
        let total_secs: u64 = self
            .model
            .time_entries
            .values()
            .filter(|entry| entry.task_id == task.id)
            .map(|entry| {
                if entry.is_running() {
                    // Live session: second precision, counted exactly once.
                    (now - entry.started_at).num_seconds().max(0) as u64
                } else {
                    u64::from(entry.calculated_duration_minutes()) * 60
                }
            })
            .sum();
        Duration::from_secs(total_secs)
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
