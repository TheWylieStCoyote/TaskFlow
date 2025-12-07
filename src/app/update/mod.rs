//! Update module - heart of the TEA (The Elm Architecture) pattern
//!
//! This module contains all message handlers organized by category:
//! - `navigation`: Movement, view switching, sidebar/calendar navigation
//! - `task`: Task CRUD operations, completion, priority
//! - `time`: Time tracking and Pomodoro timer
//! - `ui`: Input handling, multi-select, templates, keybindings
//! - `system`: Quit, save, undo/redo, import/export

mod navigation;
pub mod system;
mod task;
mod time;
mod ui;

use crate::app::{Message, Model};

pub use navigation::days_in_month;
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

    match message {
        Message::Navigation(msg) => navigation::handle_navigation(model, msg),
        Message::Task(msg) => task::handle_task(model, msg),
        Message::Time(msg) => time::handle_time(model, msg),
        Message::Pomodoro(msg) => time::handle_pomodoro(model, msg),
        Message::Ui(msg) => ui::handle_ui(model, msg),
        Message::System(msg) => system::handle_system(model, msg),
        Message::None => {}
    }
}

#[cfg(test)]
mod tests;
