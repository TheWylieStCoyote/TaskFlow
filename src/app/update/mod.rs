//! Update module - heart of the TEA (The Elm Architecture) pattern
//!
//! This module contains all message handlers organized by category:
//! - `navigation`: Movement, view switching, sidebar/calendar navigation
//! - `task`: Task CRUD operations, completion, priority
//! - `time`: Time tracking and Pomodoro timer
//! - `habit`: Habit tracking and check-ins
//! - `ui`: Input handling, multi-select, templates, keybindings
//! - `system`: Quit, save, undo/redo, import/export

mod habit;
mod navigation;
mod sync;
pub mod system;
mod task;
mod time;
mod ui;

use tracing::{debug, trace};

use crate::app::{Message, Model};

pub use navigation::days_in_month;
pub use sync::init_git_sync;
pub use task::create_next_recurring_task;
pub use ui::create_task_from_quick_add;

/// Main update function - heart of TEA pattern
///
/// Routes messages to appropriate handlers based on message type.
/// Also records messages for macro playback if recording is active.
pub fn update(model: &mut Model, message: Message) {
    // Record message if we're recording a macro
    if model.macro_state.is_recording() {
        model.macro_state.record(&message);
    }

    // Log message at trace level (very verbose)
    trace!(?message, "Processing message");

    match message {
        Message::Navigation(msg) => {
            debug!(?msg, "Navigation");
            navigation::handle_navigation(model, msg);
        }
        Message::Task(msg) => {
            debug!(?msg, "Task operation");
            task::handle_task(model, msg);
        }
        Message::Time(msg) => {
            debug!(?msg, "Time tracking");
            time::handle_time(model, msg);
        }
        Message::Pomodoro(msg) => {
            trace!(?msg, "Pomodoro tick"); // trace level since this fires every second
            time::handle_pomodoro(model, msg);
        }
        Message::Habit(msg) => {
            debug!(?msg, "Habit tracking");
            habit::handle_habit(model, msg);
        }
        Message::Ui(msg) => {
            trace!(?msg, "UI event"); // trace level since these are frequent
            ui::handle_ui(model, msg);
        }
        Message::System(msg) => {
            debug!(?msg, "System operation");
            system::handle_system(model, msg);
        }
        Message::Sync(msg) => {
            debug!(?msg, "Git sync operation");
            sync::handle_sync(model, msg);
        }
        Message::None => {}
    }
}

#[cfg(test)]
mod tests;
