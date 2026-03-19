//! Build a `ModelSnapshot` from the live `Model`.

use std::time::SystemTime;

use crate::app::Model;

use super::types::{ModelSnapshot, TaskSummary};

impl ModelSnapshot {
    /// Construct a snapshot from the current model state.
    pub fn from_model(model: &Model) -> Self {
        let tasks: Vec<TaskSummary> = model
            .tasks
            .values()
            .map(|t| TaskSummary {
                id: t.id.to_string(),
                title: t.title.clone(),
                status: t.status,
                priority: t.priority,
                project_id: t.project_id.map(|p| p.to_string()),
                due_date: t.due_date.map(|d| d.to_string()),
            })
            .collect();

        let incomplete_count = tasks.iter().filter(|t| !t.status.is_complete()).count();

        // Determine which task is "focused": selected task in task list.
        let focus_task_id = model
            .visible_tasks
            .get(model.selected_index)
            .map(std::string::ToString::to_string);

        ModelSnapshot {
            tasks,
            focus_task_id,
            incomplete_count,
            protocol_version: "1",
            snapshot_at: SystemTime::now(),
        }
    }
}
