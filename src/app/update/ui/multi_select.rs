//! Multi-select and bulk operation handlers.

use crate::app::{Model, UndoAction};
use crate::ui::{InputMode, InputTarget};

/// Toggle multi-select mode.
pub fn toggle_multi_select(model: &mut Model) {
    model.multi_select_mode = !model.multi_select_mode;
    if !model.multi_select_mode {
        // Exiting multi-select mode clears selection
        model.selected_tasks.clear();
    }
}

/// Toggle selection of the current task.
pub fn toggle_task_selection(model: &mut Model) {
    if model.multi_select_mode {
        if let Some(task_id) = model.selected_task_id() {
            if model.selected_tasks.contains(&task_id) {
                model.selected_tasks.remove(&task_id);
            } else {
                model.selected_tasks.insert(task_id);
            }
        }
    }
}

/// Select all visible tasks.
pub fn select_all(model: &mut Model) {
    model.multi_select_mode = true;
    model.selected_tasks = model.visible_tasks.iter().copied().collect();
}

/// Clear all selections.
pub fn clear_selection(model: &mut Model) {
    model.selected_tasks.clear();
    model.multi_select_mode = false;
}

/// Bulk delete all selected tasks.
pub fn bulk_delete(model: &mut Model) {
    if model.multi_select_mode && !model.selected_tasks.is_empty() {
        let tasks_to_delete: Vec<_> = model.selected_tasks.iter().copied().collect();
        for task_id in tasks_to_delete {
            if let Some(task) = model.tasks.remove(&task_id) {
                // Collect time entries for this task before deleting
                let task_entries: Vec<_> = model
                    .time_entries
                    .values()
                    .filter(|e| e.task_id == task_id)
                    .cloned()
                    .collect();

                // Clear active time entry if it belongs to this task
                if model
                    .active_time_entry
                    .as_ref()
                    .and_then(|id| model.time_entries.get(id))
                    .is_some_and(|e| e.task_id == task_id)
                {
                    model.active_time_entry = None;
                }

                // Delete time entries (collect IDs first to avoid borrow issues)
                let entry_ids: Vec<_> = task_entries.iter().map(|e| e.id).collect();
                for entry_id in entry_ids {
                    model.delete_time_entry(&entry_id);
                }

                model.delete_task_from_storage(&task_id);
                model.undo_stack.push(UndoAction::TaskDeleted {
                    task: Box::new(task),
                    time_entries: task_entries,
                });
            }
        }
        model.selected_tasks.clear();
        model.multi_select_mode = false;
        model.refresh_visible_tasks();
    }
}

/// Start bulk move to project input.
pub fn start_bulk_move_to_project(model: &mut Model) {
    if model.multi_select_mode && !model.selected_tasks.is_empty() {
        model.input_mode = InputMode::Editing;
        model.input_target = InputTarget::BulkMoveToProject;
        // Build project list string
        let mut options = vec!["0: (none)".to_string()];
        for (i, project) in model.projects.values().enumerate() {
            options.push(format!("{}: {}", i + 1, project.name));
        }
        model.input_buffer = options.join(", ");
        model.cursor_position = model.input_buffer.len();
    }
}

/// Start bulk set status input.
pub fn start_bulk_set_status(model: &mut Model) {
    if model.multi_select_mode && !model.selected_tasks.is_empty() {
        model.input_mode = InputMode::Editing;
        model.input_target = InputTarget::BulkSetStatus;
        model.input_buffer =
            "1: Todo, 2: In Progress, 3: Blocked, 4: Done, 5: Cancelled".to_string();
        model.cursor_position = model.input_buffer.len();
    }
}
