//! Delete confirmation handlers.

use crate::app::{Model, UndoAction};

/// Show delete confirmation if task has no subtasks.
pub fn show_delete_confirm(model: &mut Model) {
    if let Some(task_id) = model.selected_task_id() {
        if model.has_subtasks(&task_id) {
            model.status_message =
                Some("Cannot delete: task has subtasks. Delete subtasks first.".to_string());
        } else {
            model.show_confirm_delete = true;
        }
    }
}

/// Confirm and execute task deletion.
pub fn confirm_delete(model: &mut Model) {
    if let Some(id) = model.selected_task_id() {
        if let Some(task) = model.tasks.remove(&id) {
            // Save the task title for feedback message
            let task_title = task.title.clone();

            // Collect time entries for this task before deleting
            let task_entries: Vec<_> = model
                .time_entries
                .values()
                .filter(|e| e.task_id == id)
                .cloned()
                .collect();

            // Clear active time entry if it belongs to this task
            if model
                .active_time_entry
                .as_ref()
                .and_then(|entry_id| model.time_entries.get(entry_id))
                .is_some_and(|e| e.task_id == id)
            {
                model.active_time_entry = None;
            }

            // Delete time entries (collect IDs first to avoid borrow issues)
            let entry_ids: Vec<_> = task_entries.iter().map(|e| e.id).collect();
            for entry_id in entry_ids {
                model.delete_time_entry(&entry_id);
            }

            model.delete_task_from_storage(&id);
            model.undo_stack.push(UndoAction::TaskDeleted {
                task: Box::new(task),
                time_entries: task_entries,
            });

            // Truncate long titles for display
            let display_title = if task_title.len() > 40 {
                format!("{}...", &task_title[..37])
            } else {
                task_title
            };
            model.status_message = Some(format!("Deleted: {display_title}"));
        }
        model.refresh_visible_tasks();
    }
    model.show_confirm_delete = false;
}

/// Cancel delete confirmation.
pub fn cancel_delete(model: &mut Model) {
    model.show_confirm_delete = false;
}
