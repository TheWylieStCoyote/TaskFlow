//! View state handlers (toggle completed, sidebar, help, focus mode).

use crate::app::Model;

/// Toggle showing completed tasks.
pub fn toggle_show_completed(model: &mut Model) {
    model.show_completed = !model.show_completed;
    model.refresh_visible_tasks();
}

/// Toggle sidebar visibility.
pub fn toggle_sidebar(model: &mut Model) {
    model.show_sidebar = !model.show_sidebar;
}

/// Show help panel.
pub fn show_help(model: &mut Model) {
    model.show_help = true;
}

/// Hide help panel.
pub fn hide_help(model: &mut Model) {
    model.show_help = false;
}

/// Toggle focus mode (only if a task is selected).
pub fn toggle_focus_mode(model: &mut Model) {
    if model.selected_task().is_some() {
        model.focus_mode = !model.focus_mode;
    }
}
