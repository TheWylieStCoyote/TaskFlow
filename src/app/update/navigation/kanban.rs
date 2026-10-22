//! Kanban view navigation handlers.
//!
//! Only the selection moves here; the view scrolls the selected column so
//! the selection stays visible for whatever height the terminal has.

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
            }
        }
        NavigationMessage::KanbanRight => {
            if model.view_selection.kanban_column < 3 {
                model.view_selection.kanban_column += 1;
                model.view_selection.kanban_task_index = 0; // Reset task selection
            }
        }
        NavigationMessage::KanbanUp => {
            if model.view_selection.kanban_task_index > 0 {
                model.view_selection.kanban_task_index -= 1;
            }
        }
        NavigationMessage::KanbanDown => {
            let column_tasks = model.kanban_column_tasks(model.view_selection.kanban_column);
            if model.view_selection.kanban_task_index + 1 < column_tasks.len() {
                model.view_selection.kanban_task_index += 1;
            }
        }
        NavigationMessage::KanbanSelectColumn(column) if column < 4 => {
            model.view_selection.kanban_column = column;
            model.view_selection.kanban_task_index = 0; // Reset task selection
            model.selected_index = 0;
        }
        _ => {}
    }
}
