//! Duplicate detection UI handlers.
//!
//! Handles duplicate task detection and resolution:
//! - Dismissing duplicate pairs
//! - Merging duplicates
//! - Refreshing duplicate detection

use crate::app::{Model, UiMessage, UndoAction, ViewId};

/// Handle duplicate detection UI messages.
pub fn handle_ui_duplicates(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::DismissDuplicate => {
            if model.current_view == ViewId::Duplicates && !model.duplicates_view.pairs.is_empty() {
                let selected = model.duplicates_view.selected;
                if selected < model.duplicates_view.pairs.len() {
                    model.duplicates_view.pairs.remove(selected);
                    // Clamp selection
                    if !model.duplicates_view.pairs.is_empty() {
                        model.duplicates_view.selected = model
                            .duplicates_view
                            .selected
                            .min(model.duplicates_view.pairs.len() - 1);
                    }
                    model.alerts.status_message = Some("Duplicate pair dismissed".to_string());
                }
            }
        }
        UiMessage::MergeDuplicates => {
            if model.current_view == ViewId::Duplicates && !model.duplicates_view.pairs.is_empty() {
                let selected = model.duplicates_view.selected;
                if let Some(pair) = model.duplicates_view.pairs.get(selected).cloned() {
                    // Collect time entries for the task being deleted
                    let task_entries: Vec<_> = model
                        .time_entries
                        .values()
                        .filter(|e| e.task_id == pair.task2_id)
                        .cloned()
                        .collect();

                    // Delete the second task
                    if let Some(task) = model.tasks.remove(&pair.task2_id) {
                        model.undo_stack.push(UndoAction::TaskDeleted {
                            task: Box::new(task),
                            time_entries: task_entries,
                        });
                        model.delete_task_from_storage(&pair.task2_id);
                        // Remove the pair from the list
                        model.duplicates_view.pairs.remove(selected);
                        // Clamp selection
                        if !model.duplicates_view.pairs.is_empty() {
                            model.duplicates_view.selected = model
                                .duplicates_view
                                .selected
                                .min(model.duplicates_view.pairs.len() - 1);
                        }
                        model.alerts.status_message =
                            Some("Tasks merged (duplicate deleted)".to_string());
                        model.refresh_visible_tasks();
                    }
                }
            }
        }
        UiMessage::RefreshDuplicates => {
            if model.current_view == ViewId::Duplicates {
                model.duplicates_view.pairs =
                    crate::domain::duplicate_detector::find_all_duplicates(
                        &model.tasks,
                        model.duplicates_view.threshold,
                    );
                model.duplicates_view.selected = 0;
                let count = model.duplicates_view.pairs.len();
                model.alerts.status_message = Some(format!("Found {count} duplicate pairs"));
            }
        }
        _ => {}
    }
}
