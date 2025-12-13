//! Kanban view navigation handlers.

use crate::app::{Model, NavigationMessage, ViewId};

/// Handle kanban-specific navigation messages.
pub fn handle_kanban_navigation(model: &mut Model, msg: NavigationMessage) {
    if model.current_view != ViewId::Kanban {
        return;
    }

    match msg {
        NavigationMessage::KanbanLeft => {
            if model.view_selection.kanban_column > 0 {
                model.view_selection.kanban_column -= 1;
                model.view_selection.kanban_task_index = 0; // Reset task selection
                                                            // Reset scroll offset for new column
                model.view_selection.kanban_scroll_offsets[model.view_selection.kanban_column] = 0;
            }
        }
        NavigationMessage::KanbanRight => {
            if model.view_selection.kanban_column < 3 {
                model.view_selection.kanban_column += 1;
                model.view_selection.kanban_task_index = 0; // Reset task selection
                                                            // Reset scroll offset for new column
                model.view_selection.kanban_scroll_offsets[model.view_selection.kanban_column] = 0;
            }
        }
        NavigationMessage::KanbanUp => {
            if model.view_selection.kanban_task_index > 0 {
                model.view_selection.kanban_task_index -= 1;
                // Adjust scroll offset to keep selection visible
                let col = model.view_selection.kanban_column;
                let idx = model.view_selection.kanban_task_index;
                if idx < model.view_selection.kanban_scroll_offsets[col] {
                    model.view_selection.kanban_scroll_offsets[col] = idx;
                }
            }
        }
        NavigationMessage::KanbanDown => {
            let column_tasks = model.kanban_column_tasks(model.view_selection.kanban_column);
            if model.view_selection.kanban_task_index + 1 < column_tasks.len() {
                model.view_selection.kanban_task_index += 1;
                // Adjust scroll offset to keep selection visible (estimate viewport ~10 rows)
                let col = model.view_selection.kanban_column;
                let idx = model.view_selection.kanban_task_index;
                let scroll = model.view_selection.kanban_scroll_offsets[col];
                const ESTIMATED_VIEWPORT: usize = 10;
                if idx >= scroll + ESTIMATED_VIEWPORT {
                    model.view_selection.kanban_scroll_offsets[col] =
                        idx.saturating_sub(ESTIMATED_VIEWPORT - 1);
                }
            }
        }
        NavigationMessage::KanbanSelectColumn(column) => {
            if column < 4 {
                model.view_selection.kanban_column = column;
                model.view_selection.kanban_task_index = 0; // Reset task selection
                model.view_selection.kanban_scroll_offsets[column] = 0; // Reset scroll
                model.selected_index = 0;
            }
        }
        _ => {}
    }
}
