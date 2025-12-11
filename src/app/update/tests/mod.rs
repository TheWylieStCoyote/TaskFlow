//! Tests for the update function.
//!
//! Tests are organized into modules by functionality:
//! - `navigation` - Navigation (up, down, page, view switching)
//! - `task_crud` - Task create, delete, toggle complete
//! - `ui` - UI state (input, sidebar, help)
//! - `system` - System messages (quit, resize)
//! - `sidebar` - Sidebar navigation and selection
//! - `project` - Project creation and management
//! - `editing` - Task editing (title, due date, tags, description)
//! - `move_to_project` - Moving tasks between projects
//! - `tag_filter` - Tag filtering
//! - `undo_redo` - Undo/redo functionality
//! - `subtasks` - Subtask creation
//! - `bulk` - Bulk operations (multi-select, delete)
//! - `recurrence` - Recurring tasks
//! - `chains` - Task chains (blocking/linked tasks)
//! - `pomodoro` - Pomodoro timer
//! - `keybindings` - Keybindings editor
//! - `calendar` - Calendar view and focus
//! - `import_export` - Import/export and reports
//! - `cascade` - Cascade completion (parent/child)
//! - `time_tracking` - Time entry management and cleanup
//! - `habit` - Habit tracking operations

mod bulk;
mod calendar;
mod cascade;
mod chains;
mod editing;
mod editors;
mod filters;
mod habit;
mod import_export;
mod keybindings;
mod macros;
mod move_to_project;
mod navigation;
mod pomodoro;
mod project;
mod recurrence;
mod reviews;
mod sidebar;
mod subtasks;
mod system;
mod tag_filter;
mod task_crud;
mod templates;
mod time_log;
mod time_tracking;
mod ui;
mod undo_redo;

use crate::app::Model;
use crate::domain::{Task, TaskId};

/// Creates a test model with 5 sample tasks.
pub fn create_test_model_with_tasks() -> Model {
    let mut model = Model::new();

    for i in 0..5 {
        let task = Task::new(format!("Task {i}"));
        model.tasks.insert(task.id, task);
    }
    model.refresh_visible_tasks();
    model
}

/// Safely gets a visible task ID with bounds checking.
/// Panics with a descriptive message if index is out of bounds.
pub fn visible_task_id(model: &Model, index: usize) -> TaskId {
    assert!(
        index < model.visible_tasks.len(),
        "Test requires visible task at index {}, but only {} tasks are visible",
        index,
        model.visible_tasks.len()
    );
    model.visible_tasks[index]
}

/// Safely gets a reference to a visible task with bounds checking.
/// Panics with a descriptive message if index is out of bounds.
#[allow(dead_code)]
pub fn visible_task(model: &Model, index: usize) -> &Task {
    let task_id = visible_task_id(model, index);
    model
        .tasks
        .get(&task_id)
        .expect("Task ID in visible_tasks should exist in tasks map")
}
