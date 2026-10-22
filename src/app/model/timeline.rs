//! Timeline view queries for the Model.
//!
//! The timeline draws only the visible tasks that carry a scheduled or due
//! date, in a fixed order. Update handlers and the view share this one
//! definition so the selection index always refers to a drawn row.

use crate::domain::{Priority, Task, TaskId};

use super::Model;

impl Model {
    /// Tasks the timeline draws: visible tasks with a scheduled or due date,
    /// sorted by start date (scheduled, else due) ascending, then priority
    /// descending, then title.
    #[must_use]
    pub fn timeline_tasks(&self) -> Vec<&Task> {
        let mut tasks: Vec<_> = self
            .visible_tasks
            .iter()
            .filter_map(|id| self.tasks.get(id))
            .filter(|t| t.scheduled_date.is_some() || t.due_date.is_some())
            .collect();

        // Priority ordering (higher number = higher priority for sorting)
        let priority_order = |p: &Priority| match p {
            Priority::Urgent => 4,
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
            Priority::None => 0,
        };

        tasks.sort_by(|a, b| {
            let a_start = a.scheduled_date.or(a.due_date);
            let b_start = b.scheduled_date.or(b.due_date);
            a_start
                .cmp(&b_start)
                .then_with(|| priority_order(&b.priority).cmp(&priority_order(&a.priority)))
                .then_with(|| a.title.cmp(&b.title))
        });

        tasks
    }

    /// IDs of [`Model::timeline_tasks`], in drawing order.
    #[must_use]
    pub fn timeline_task_ids(&self) -> Vec<TaskId> {
        self.timeline_tasks().iter().map(|t| t.id).collect()
    }

    /// Snap `timeline_state.selected_task_index` back onto the last drawn
    /// row when the timeline's task set has shrunk beneath it.
    pub fn clamp_timeline_selection(&mut self) {
        let max_index = self.timeline_tasks().len().saturating_sub(1);
        if self.timeline_state.selected_task_index > max_index {
            self.timeline_state.selected_task_index = max_index;
        }
    }
}
