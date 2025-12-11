//! Task selection helper methods.

use crate::domain::{Task, TaskId};

use super::super::Model;

impl Model {
    /// Returns the TaskId of the currently selected task, if any.
    ///
    /// Returns `None` if no task is selected or the selection index is out of bounds.
    #[inline]
    pub fn selected_task_id(&self) -> Option<TaskId> {
        self.visible_tasks.get(self.selected_index).copied()
    }

    /// Returns the currently selected task, if any.
    ///
    /// Returns `None` if no tasks are visible or the selection is invalid.
    #[must_use]
    pub fn selected_task(&self) -> Option<&Task> {
        self.visible_tasks
            .get(self.selected_index)
            .and_then(|id| self.tasks.get(id))
    }

    /// Returns the currently selected task mutably, if any.
    #[must_use]
    pub fn selected_task_mut(&mut self) -> Option<&mut Task> {
        let id = *self.visible_tasks.get(self.selected_index)?;
        self.tasks.get_mut(&id)
    }
}
