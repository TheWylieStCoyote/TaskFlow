//! Multi-line editors (work log and description)

use crate::app::model::MultilineEditor;
use crate::app::{Model, UiMessage, UndoAction};
use crate::domain::WorkLogEntry;
use crate::ui::WorkLogMode;

/// Handle work log UI messages
pub fn handle_ui_work_log(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowWorkLog => {
            // Only show if a task is selected
            if model.selected_index < model.visible_tasks.len() {
                model.work_log_editor.visible = true;
                model.work_log_editor.selected = 0;
                model.work_log_editor.mode = WorkLogMode::Browse;
                model.work_log_editor.clear();
            }
        }
        UiMessage::HideWorkLog => {
            model.work_log_editor.visible = false;
            model.work_log_editor.mode = WorkLogMode::Browse;
            model.work_log_editor.clear();
        }
        UiMessage::WorkLogUp => {
            if model.work_log_editor.selected > 0 {
                model.work_log_editor.selected -= 1;
            }
        }
        UiMessage::WorkLogDown => {
            if let Some(task_id) = model.selected_task_id() {
                let entries = model.work_logs_for_task(&task_id);
                if model.work_log_editor.selected < entries.len().saturating_sub(1) {
                    model.work_log_editor.selected += 1;
                }
            }
        }
        UiMessage::WorkLogView => {
            if model.work_log_editor.mode == WorkLogMode::Browse {
                if let Some(task_id) = model.selected_task_id() {
                    let entries = model.work_logs_for_task(&task_id);
                    if entries.get(model.work_log_editor.selected).is_some() {
                        model.work_log_editor.mode = WorkLogMode::View;
                    }
                }
            } else if model.work_log_editor.mode == WorkLogMode::View {
                // Return to browse mode when viewing and pressing Enter again
                model.work_log_editor.mode = WorkLogMode::Browse;
            }
        }
        UiMessage::WorkLogAdd => {
            model.work_log_editor.mode = WorkLogMode::Add;
            model.work_log_editor.clear();
        }
        UiMessage::WorkLogEdit => {
            if let Some(task_id) = model.selected_task_id() {
                let entries = model.work_logs_for_task(&task_id);
                // Clone content to avoid borrow conflict with model
                let content = entries
                    .get(model.work_log_editor.selected)
                    .map(|e| e.content.clone());
                if let Some(content) = content {
                    model.work_log_editor.mode = WorkLogMode::Edit;
                    model.work_log_editor.set_content(&content);
                }
            }
        }
        UiMessage::WorkLogConfirmDelete => {
            if model.work_log_editor.mode == WorkLogMode::Browse
                || model.work_log_editor.mode == WorkLogMode::View
            {
                model.work_log_editor.mode = WorkLogMode::ConfirmDelete;
            }
        }
        UiMessage::WorkLogCancel => {
            // Return to browse mode from any other mode
            model.work_log_editor.mode = WorkLogMode::Browse;
            model.work_log_editor.clear();
        }
        UiMessage::WorkLogSubmit => {
            if let Some(task_id) = model.selected_task_id() {
                let content = model.work_log_editor.content();

                // Don't save empty entries
                if content.trim().is_empty() {
                    model.alerts.status_message =
                        Some("Cannot save empty work log entry".to_string());
                    return;
                }

                match model.work_log_editor.mode {
                    WorkLogMode::Add => {
                        let entry = WorkLogEntry::new(task_id, content);
                        model
                            .undo_stack
                            .push(UndoAction::WorkLogCreated(Box::new(entry.clone())));
                        model.sync_work_log(&entry);
                        model.work_logs.insert(entry.id, entry);
                        model.work_log_editor.selected = 0; // New entry will be at top
                        model.alerts.status_message = Some("Work log entry added".to_string());
                    }
                    WorkLogMode::Edit => {
                        let entries = model.work_logs_for_task(&task_id);
                        if let Some(entry) = entries.get(model.work_log_editor.selected) {
                            let entry_id = entry.id;
                            if let Some(existing) = model.work_logs.get_mut(&entry_id) {
                                let before = existing.clone();
                                existing.update_content(content);
                                let after = existing.clone();
                                model.undo_stack.push(UndoAction::WorkLogModified {
                                    before: Box::new(before),
                                    after: Box::new(after.clone()),
                                });
                                model.sync_work_log(&after);
                                model.alerts.status_message =
                                    Some("Work log entry updated".to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            model.work_log_editor.mode = WorkLogMode::Browse;
            model.work_log_editor.clear();
        }
        UiMessage::WorkLogDelete => {
            if model.work_log_editor.mode == WorkLogMode::ConfirmDelete {
                if let Some(task_id) = model.selected_task_id() {
                    let entries = model.work_logs_for_task(&task_id);
                    if let Some(entry) = entries.get(model.work_log_editor.selected) {
                        let entry_id = entry.id;

                        if let Some(removed) = model.work_logs.remove(&entry_id) {
                            model
                                .undo_stack
                                .push(UndoAction::WorkLogDeleted(Box::new(removed)));
                            model.delete_work_log_from_storage(&entry_id);

                            // Adjust selection
                            let remaining = model.work_logs_for_task(&task_id);
                            if model.work_log_editor.selected >= remaining.len()
                                && !remaining.is_empty()
                            {
                                model.work_log_editor.selected = remaining.len() - 1;
                            }
                            model.alerts.status_message =
                                Some("Work log entry deleted".to_string());
                        }
                    }
                }
                model.work_log_editor.mode = WorkLogMode::Browse;
            }
        }
        // Multi-line input handling - use trait methods
        UiMessage::WorkLogInputChar(c) => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.insert_char(c);
            }
        }
        UiMessage::WorkLogInputBackspace => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.backspace();
            }
        }
        UiMessage::WorkLogInputDelete => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.delete_char();
            }
        }
        UiMessage::WorkLogCursorLeft => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.cursor_left();
            }
        }
        UiMessage::WorkLogCursorRight => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.cursor_right();
            }
        }
        UiMessage::WorkLogCursorUp => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.cursor_up();
            }
        }
        UiMessage::WorkLogCursorDown => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.cursor_down();
            }
        }
        UiMessage::WorkLogNewline => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.newline();
            }
        }
        UiMessage::WorkLogCursorHome => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.cursor_home();
            }
        }
        UiMessage::WorkLogCursorEnd => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.cursor_end();
            }
        }
        // Work log search messages
        UiMessage::WorkLogSearchStart => {
            model.work_log_editor.mode = WorkLogMode::Search;
        }
        UiMessage::WorkLogSearchCancel => {
            // Return to browse without applying search
            model.work_log_editor.mode = WorkLogMode::Browse;
            model.work_log_editor.search_query.clear();
        }
        UiMessage::WorkLogSearchApply => {
            // Return to browse with search filter active
            model.work_log_editor.mode = WorkLogMode::Browse;
            model.work_log_editor.selected = 0; // Reset selection after filtering
        }
        UiMessage::WorkLogSearchClear => {
            // Clear the search filter
            model.work_log_editor.search_query.clear();
            model.work_log_editor.selected = 0;
        }
        UiMessage::WorkLogSearchChar(c) => {
            if matches!(model.work_log_editor.mode, WorkLogMode::Search) {
                model.work_log_editor.search_query.push(c);
            }
        }
        UiMessage::WorkLogSearchBackspace => {
            if matches!(model.work_log_editor.mode, WorkLogMode::Search) {
                model.work_log_editor.search_query.pop();
            }
        }
        _ => {}
    }
}

/// Handle description editor (multi-line) messages
pub fn handle_ui_description_editor(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::StartEditDescriptionMultiline => {
            // Only open if a task is selected
            if let Some(task_id) = model.selected_task_id() {
                if let Some(task) = model.tasks.get(&task_id) {
                    // Load description into buffer using trait method
                    let description = task.description.clone().unwrap_or_default();
                    model.description_editor.set_content(&description);
                    model.description_editor.visible = true;
                }
            }
        }
        UiMessage::HideDescriptionEditor => {
            model.description_editor.visible = false;
            model.description_editor.clear();
        }
        UiMessage::DescriptionSubmit => {
            if let Some(task_id) = model.selected_task_id() {
                let content = model.description_editor.content();
                let description = if content.trim().is_empty() {
                    None
                } else {
                    Some(content)
                };

                // Use modify_task_with_undo for proper undo support
                model.modify_task_with_undo(&task_id, |task| {
                    task.description = description;
                });

                model.alerts.status_message = Some("Description updated".to_string());
            }
            model.description_editor.visible = false;
            model.description_editor.clear();
        }
        // Multi-line input handling - use trait methods
        UiMessage::DescriptionInputChar(c) => {
            if model.description_editor.visible {
                model.description_editor.insert_char(c);
            }
        }
        UiMessage::DescriptionInputBackspace => {
            if model.description_editor.visible {
                model.description_editor.backspace();
            }
        }
        UiMessage::DescriptionInputDelete => {
            if model.description_editor.visible {
                model.description_editor.delete_char();
            }
        }
        UiMessage::DescriptionCursorLeft => {
            if model.description_editor.visible {
                model.description_editor.cursor_left();
            }
        }
        UiMessage::DescriptionCursorRight => {
            if model.description_editor.visible {
                model.description_editor.cursor_right();
            }
        }
        UiMessage::DescriptionCursorUp => {
            if model.description_editor.visible {
                model.description_editor.cursor_up();
            }
        }
        UiMessage::DescriptionCursorDown => {
            if model.description_editor.visible {
                model.description_editor.cursor_down();
            }
        }
        UiMessage::DescriptionNewline => {
            if model.description_editor.visible {
                model.description_editor.newline();
            }
        }
        UiMessage::DescriptionCursorHome => {
            if model.description_editor.visible {
                model.description_editor.cursor_home();
            }
        }
        UiMessage::DescriptionCursorEnd => {
            if model.description_editor.visible {
                model.description_editor.cursor_end();
            }
        }
        _ => {}
    }
}
