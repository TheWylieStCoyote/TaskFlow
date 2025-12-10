//! Multi-line editors (work log and description)

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
                model.work_log_editor.buffer = vec![String::new()];
                model.work_log_editor.cursor_line = 0;
                model.work_log_editor.cursor_col = 0;
            }
        }
        UiMessage::HideWorkLog => {
            model.work_log_editor.visible = false;
            model.work_log_editor.mode = WorkLogMode::Browse;
            model.work_log_editor.buffer = vec![String::new()];
            model.work_log_editor.cursor_line = 0;
            model.work_log_editor.cursor_col = 0;
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
            model.work_log_editor.buffer = vec![String::new()];
            model.work_log_editor.cursor_line = 0;
            model.work_log_editor.cursor_col = 0;
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
                    // Load content into buffer as lines
                    model.work_log_editor.buffer = content.lines().map(String::from).collect();
                    if model.work_log_editor.buffer.is_empty() {
                        model.work_log_editor.buffer.push(String::new());
                    }
                    model.work_log_editor.cursor_line = 0;
                    model.work_log_editor.cursor_col = 0;
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
            model.work_log_editor.buffer = vec![String::new()];
            model.work_log_editor.cursor_line = 0;
            model.work_log_editor.cursor_col = 0;
        }
        UiMessage::WorkLogSubmit => {
            if let Some(task_id) = model.selected_task_id() {
                let content = model.work_log_editor.buffer.join("\n");

                // Don't save empty entries
                if content.trim().is_empty() {
                    model.alerts.status_message = Some("Cannot save empty work log entry".to_string());
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
                                model.alerts.status_message = Some("Work log entry updated".to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            model.work_log_editor.mode = WorkLogMode::Browse;
            model.work_log_editor.buffer = vec![String::new()];
            model.work_log_editor.cursor_line = 0;
            model.work_log_editor.cursor_col = 0;
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
                                .push(UndoAction::WorkLogDeleted(Box::new(removed.clone())));
                            model.delete_work_log_from_storage(&entry_id);

                            // Adjust selection
                            let remaining = model.work_logs_for_task(&task_id);
                            if model.work_log_editor.selected >= remaining.len()
                                && !remaining.is_empty()
                            {
                                model.work_log_editor.selected = remaining.len() - 1;
                            }
                            model.alerts.status_message = Some("Work log entry deleted".to_string());
                        }
                    }
                }
                model.work_log_editor.mode = WorkLogMode::Browse;
            }
        }
        // Multi-line input handling
        UiMessage::WorkLogInputChar(c) => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                // Ensure we have at least one line
                if model.work_log_editor.buffer.is_empty() {
                    model.work_log_editor.buffer.push(String::new());
                }

                let line_idx = model
                    .work_log_editor
                    .cursor_line
                    .min(model.work_log_editor.buffer.len() - 1);
                let col = model
                    .work_log_editor
                    .cursor_col
                    .min(model.work_log_editor.buffer[line_idx].len());

                model.work_log_editor.buffer[line_idx].insert(col, c);
                model.work_log_editor.cursor_col = col + 1;
            }
        }
        UiMessage::WorkLogInputBackspace => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                let line_idx = model.work_log_editor.cursor_line;
                if line_idx < model.work_log_editor.buffer.len() {
                    if model.work_log_editor.cursor_col > 0 {
                        // Delete character before cursor
                        let col = model
                            .work_log_editor
                            .cursor_col
                            .min(model.work_log_editor.buffer[line_idx].len());
                        if col > 0 {
                            model.work_log_editor.buffer[line_idx].remove(col - 1);
                            model.work_log_editor.cursor_col = col - 1;
                        }
                    } else if line_idx > 0 {
                        // At beginning of line - join with previous line
                        let current_line = model.work_log_editor.buffer.remove(line_idx);
                        model.work_log_editor.cursor_line = line_idx - 1;
                        model.work_log_editor.cursor_col =
                            model.work_log_editor.buffer[line_idx - 1].len();
                        model.work_log_editor.buffer[line_idx - 1].push_str(&current_line);
                    }
                }
            }
        }
        UiMessage::WorkLogInputDelete => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                let line_idx = model.work_log_editor.cursor_line;
                if line_idx < model.work_log_editor.buffer.len() {
                    let line_len = model.work_log_editor.buffer[line_idx].len();
                    if model.work_log_editor.cursor_col < line_len {
                        // Delete character at cursor
                        model.work_log_editor.buffer[line_idx]
                            .remove(model.work_log_editor.cursor_col);
                    } else if line_idx + 1 < model.work_log_editor.buffer.len() {
                        // At end of line - join with next line
                        let next_line = model.work_log_editor.buffer.remove(line_idx + 1);
                        model.work_log_editor.buffer[line_idx].push_str(&next_line);
                    }
                }
            }
        }
        UiMessage::WorkLogCursorLeft => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                if model.work_log_editor.cursor_col > 0 {
                    model.work_log_editor.cursor_col -= 1;
                } else if model.work_log_editor.cursor_line > 0 {
                    // Move to end of previous line
                    model.work_log_editor.cursor_line -= 1;
                    model.work_log_editor.cursor_col =
                        model.work_log_editor.buffer[model.work_log_editor.cursor_line].len();
                }
            }
        }
        UiMessage::WorkLogCursorRight => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                let line_idx = model.work_log_editor.cursor_line;
                if line_idx < model.work_log_editor.buffer.len() {
                    let line_len = model.work_log_editor.buffer[line_idx].len();
                    if model.work_log_editor.cursor_col < line_len {
                        model.work_log_editor.cursor_col += 1;
                    } else if line_idx + 1 < model.work_log_editor.buffer.len() {
                        // Move to start of next line
                        model.work_log_editor.cursor_line += 1;
                        model.work_log_editor.cursor_col = 0;
                    }
                }
            }
        }
        UiMessage::WorkLogCursorUp => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) && model.work_log_editor.cursor_line > 0
            {
                model.work_log_editor.cursor_line -= 1;
                // Clamp column to line length
                let line_len =
                    model.work_log_editor.buffer[model.work_log_editor.cursor_line].len();
                model.work_log_editor.cursor_col = model.work_log_editor.cursor_col.min(line_len);
            }
        }
        UiMessage::WorkLogCursorDown => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) && model.work_log_editor.cursor_line + 1 < model.work_log_editor.buffer.len()
            {
                model.work_log_editor.cursor_line += 1;
                // Clamp column to line length
                let line_len =
                    model.work_log_editor.buffer[model.work_log_editor.cursor_line].len();
                model.work_log_editor.cursor_col = model.work_log_editor.cursor_col.min(line_len);
            }
        }
        UiMessage::WorkLogNewline => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                let line_idx = model.work_log_editor.cursor_line;
                if line_idx < model.work_log_editor.buffer.len() {
                    // Split current line at cursor position
                    let col = model
                        .work_log_editor
                        .cursor_col
                        .min(model.work_log_editor.buffer[line_idx].len());
                    let remainder = model.work_log_editor.buffer[line_idx].split_off(col);
                    model.work_log_editor.buffer.insert(line_idx + 1, remainder);
                    model.work_log_editor.cursor_line += 1;
                    model.work_log_editor.cursor_col = 0;
                }
            }
        }
        UiMessage::WorkLogCursorHome => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                model.work_log_editor.cursor_col = 0;
            }
        }
        UiMessage::WorkLogCursorEnd => {
            if matches!(
                model.work_log_editor.mode,
                WorkLogMode::Add | WorkLogMode::Edit
            ) {
                let line_idx = model.work_log_editor.cursor_line;
                if line_idx < model.work_log_editor.buffer.len() {
                    model.work_log_editor.cursor_col = model.work_log_editor.buffer[line_idx].len();
                }
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
                    // Load description into buffer as lines
                    let description = task.description.clone().unwrap_or_default();
                    model.description_editor.buffer = if description.is_empty() {
                        vec![String::new()]
                    } else {
                        description.lines().map(String::from).collect()
                    };
                    if model.description_editor.buffer.is_empty() {
                        model.description_editor.buffer.push(String::new());
                    }
                    model.description_editor.cursor_line = 0;
                    model.description_editor.cursor_col = 0;
                    model.description_editor.visible = true;
                }
            }
        }
        UiMessage::HideDescriptionEditor => {
            model.description_editor.visible = false;
            model.description_editor.buffer = vec![String::new()];
            model.description_editor.cursor_line = 0;
            model.description_editor.cursor_col = 0;
        }
        UiMessage::DescriptionSubmit => {
            if let Some(task_id) = model.selected_task_id() {
                let content = model.description_editor.buffer.join("\n");
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
            model.description_editor.buffer = vec![String::new()];
            model.description_editor.cursor_line = 0;
            model.description_editor.cursor_col = 0;
        }
        UiMessage::DescriptionInputChar(c) => {
            if model.description_editor.visible {
                // Ensure we have at least one line
                if model.description_editor.buffer.is_empty() {
                    model.description_editor.buffer.push(String::new());
                }

                let line_idx = model
                    .description_editor
                    .cursor_line
                    .min(model.description_editor.buffer.len() - 1);
                let col = model
                    .description_editor
                    .cursor_col
                    .min(model.description_editor.buffer[line_idx].len());

                model.description_editor.buffer[line_idx].insert(col, c);
                model.description_editor.cursor_col = col + 1;
            }
        }
        UiMessage::DescriptionInputBackspace => {
            if model.description_editor.visible {
                let line_idx = model.description_editor.cursor_line;
                if line_idx < model.description_editor.buffer.len() {
                    if model.description_editor.cursor_col > 0 {
                        // Delete character before cursor
                        let col = model
                            .description_editor
                            .cursor_col
                            .min(model.description_editor.buffer[line_idx].len());
                        if col > 0 {
                            model.description_editor.buffer[line_idx].remove(col - 1);
                            model.description_editor.cursor_col = col - 1;
                        }
                    } else if line_idx > 0 {
                        // At beginning of line - join with previous line
                        let current_line = model.description_editor.buffer.remove(line_idx);
                        model.description_editor.cursor_line = line_idx - 1;
                        model.description_editor.cursor_col =
                            model.description_editor.buffer[line_idx - 1].len();
                        model.description_editor.buffer[line_idx - 1].push_str(&current_line);
                    }
                }
            }
        }
        UiMessage::DescriptionInputDelete => {
            if model.description_editor.visible {
                let line_idx = model.description_editor.cursor_line;
                if line_idx < model.description_editor.buffer.len() {
                    let col = model.description_editor.cursor_col;
                    let line_len = model.description_editor.buffer[line_idx].len();
                    if col < line_len {
                        // Delete character at cursor
                        model.description_editor.buffer[line_idx].remove(col);
                    } else if line_idx + 1 < model.description_editor.buffer.len() {
                        // At end of line - join with next line
                        let next_line = model.description_editor.buffer.remove(line_idx + 1);
                        model.description_editor.buffer[line_idx].push_str(&next_line);
                    }
                }
            }
        }
        UiMessage::DescriptionCursorLeft => {
            if model.description_editor.visible {
                if model.description_editor.cursor_col > 0 {
                    model.description_editor.cursor_col -= 1;
                } else if model.description_editor.cursor_line > 0 {
                    // Move to end of previous line
                    model.description_editor.cursor_line -= 1;
                    model.description_editor.cursor_col =
                        model.description_editor.buffer[model.description_editor.cursor_line].len();
                }
            }
        }
        UiMessage::DescriptionCursorRight => {
            if model.description_editor.visible {
                let line_idx = model.description_editor.cursor_line;
                if line_idx < model.description_editor.buffer.len() {
                    let line_len = model.description_editor.buffer[line_idx].len();
                    if model.description_editor.cursor_col < line_len {
                        model.description_editor.cursor_col += 1;
                    } else if line_idx + 1 < model.description_editor.buffer.len() {
                        // Move to beginning of next line
                        model.description_editor.cursor_line += 1;
                        model.description_editor.cursor_col = 0;
                    }
                }
            }
        }
        UiMessage::DescriptionCursorUp => {
            if model.description_editor.visible && model.description_editor.cursor_line > 0 {
                model.description_editor.cursor_line -= 1;
                // Clamp column to new line length
                let new_line_len =
                    model.description_editor.buffer[model.description_editor.cursor_line].len();
                if model.description_editor.cursor_col > new_line_len {
                    model.description_editor.cursor_col = new_line_len;
                }
            }
        }
        UiMessage::DescriptionCursorDown => {
            if model.description_editor.visible
                && model.description_editor.cursor_line + 1 < model.description_editor.buffer.len()
            {
                model.description_editor.cursor_line += 1;
                // Clamp column to new line length
                let new_line_len =
                    model.description_editor.buffer[model.description_editor.cursor_line].len();
                if model.description_editor.cursor_col > new_line_len {
                    model.description_editor.cursor_col = new_line_len;
                }
            }
        }
        UiMessage::DescriptionNewline => {
            if model.description_editor.visible {
                let line_idx = model.description_editor.cursor_line;
                if line_idx < model.description_editor.buffer.len() {
                    // Split current line at cursor position
                    let col = model
                        .description_editor
                        .cursor_col
                        .min(model.description_editor.buffer[line_idx].len());
                    let remainder = model.description_editor.buffer[line_idx].split_off(col);
                    model
                        .description_editor
                        .buffer
                        .insert(line_idx + 1, remainder);
                    model.description_editor.cursor_line += 1;
                    model.description_editor.cursor_col = 0;
                }
            }
        }
        UiMessage::DescriptionCursorHome => {
            if model.description_editor.visible {
                model.description_editor.cursor_col = 0;
            }
        }
        UiMessage::DescriptionCursorEnd => {
            if model.description_editor.visible {
                let line_idx = model.description_editor.cursor_line;
                if line_idx < model.description_editor.buffer.len() {
                    model.description_editor.cursor_col =
                        model.description_editor.buffer[line_idx].len();
                }
            }
        }
        _ => {}
    }
}
