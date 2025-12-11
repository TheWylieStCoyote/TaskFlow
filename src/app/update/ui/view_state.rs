//! View state handlers (toggle completed, sidebar, help, focus mode).

use crate::app::Model;

/// Toggle showing completed tasks.
pub fn toggle_show_completed(model: &mut Model) {
    model.filtering.show_completed = !model.filtering.show_completed;
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

/// Toggle full-screen mode in focus view.
pub fn toggle_full_screen_focus(model: &mut Model) {
    model.pomodoro.full_screen = !model.pomodoro.full_screen;
}

/// Add selected task to focus queue.
pub fn add_to_focus_queue(model: &mut Model) {
    if let Some(task_id) = model.selected_task_id() {
        if !model.pomodoro.focus_queue.contains(&task_id) {
            model.pomodoro.focus_queue.push(task_id);
        }
    }
}

/// Clear the focus queue.
pub fn clear_focus_queue(model: &mut Model) {
    model.pomodoro.focus_queue.clear();
}

/// Advance to next task in focus queue.
pub fn advance_focus_queue(model: &mut Model) {
    if !model.pomodoro.focus_queue.is_empty() {
        // Remove first task from queue
        let next_task_id = model.pomodoro.focus_queue.remove(0);
        // Find and select this task in visible_tasks
        if let Some(idx) = model
            .visible_tasks
            .iter()
            .position(|&id| id == next_task_id)
        {
            model.selected_index = idx;
        }
    }
}
