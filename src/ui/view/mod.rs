//! Main view rendering module.
//!
//! This module contains the primary [`view`] function that renders the entire
//! application UI based on the current model state. It composes the various
//! UI components (sidebar, task list, modals, etc.) into a cohesive layout.
//!
//! # Layout Structure
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              Header (title)             │
//! ├──────────┬──────────────────────────────┤
//! │          │                              │
//! │ Sidebar  │       Main Content           │
//! │          │    (view-dependent)          │
//! │          │                              │
//! ├──────────┴──────────────────────────────┤
//! │              Footer (status)            │
//! └─────────────────────────────────────────┘
//! ```

mod footer;
mod layout;
mod popups;
#[cfg(test)]
mod tests;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::app::Model;
use crate::config::Theme;

/// Main view function - renders the entire UI based on model state
pub fn view(model: &Model, frame: &mut Frame<'_>, theme: &Theme) {
    let area = frame.area();

    // Full-screen focus mode: hide header and footer for minimal distraction
    let is_full_screen_focus = model.focus_mode && model.pomodoro.full_screen;

    if is_full_screen_focus {
        // Render only the content (FocusView takes over entire area)
        layout::render_content(model, frame, area, theme);
    } else {
        // Main layout: header, content, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header
        layout::render_header(frame, chunks[0], theme);

        // Render main content
        layout::render_content(model, frame, chunks[1], theme);

        // Render footer
        footer::render_footer(model, frame, chunks[2], theme);
    }

    // Render popups (help, dialogs, editors, alerts, reviews)
    popups::render_popups(model, frame, area, theme);
}
