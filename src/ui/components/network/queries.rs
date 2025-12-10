//! Query methods for network view.

use crate::domain::TaskId;

use super::Network;

impl Network<'_> {
    /// Get the selected task ID based on the current selection index
    pub(crate) fn get_selected_task_id(&self) -> Option<TaskId> {
        let network_tasks = self.model.network_tasks();
        network_tasks.get(self.selected_task_index).copied()
    }

    /// Get tasks with no dependencies (roots)
    pub(crate) fn get_root_tasks(&self) -> Vec<TaskId> {
        self.model
            .tasks
            .values()
            .filter(|t| {
                t.dependencies.is_empty()
                    && (self
                        .model
                        .tasks
                        .values()
                        .any(|other| other.dependencies.contains(&t.id))
                        || t.next_task_id.is_some())
            })
            .map(|t| t.id)
            .collect()
    }
}
