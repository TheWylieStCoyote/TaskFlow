//! View switching navigation handlers.

use crate::app::{FocusPane, Model, NavigationMessage, ViewId};
use crate::domain::duplicate_detector::find_all_duplicates;

/// Handle view switching navigation messages.
pub fn handle_view_navigation(model: &mut Model, msg: NavigationMessage) {
    if let NavigationMessage::GoToView(view_id) = msg {
        model.current_view = view_id;
        model.selected_index = 0;
        model.selected_project = None;
        model.focus_pane = FocusPane::TaskList;
        model.habit_view.show_analytics = false; // Clear modal state when switching views

        // Reset task list scroll position when changing views
        *model.task_list_state.borrow_mut() = ratatui::widgets::ListState::default();

        // Special handling for Duplicates view - refresh duplicate pairs
        if view_id == ViewId::Duplicates {
            model.duplicates_view.pairs =
                find_all_duplicates(&model.tasks, model.duplicates_view.threshold);
            model.duplicates_view.selected = 0;
            model.duplicates_view.scroll_offset = 0;
        }

        // Special handling for Reports view - ensure cache is populated
        if view_id == ViewId::Reports {
            model.ensure_report_cache_populated();
        }

        model.refresh_visible_tasks();
    }
}
