//! Test utilities for UI component testing.
//!
//! Provides common helpers for rendering widgets and extracting buffer content.
//! Used across all UI component tests to avoid code duplication.
//!
//! # Example
//!
//! ```ignore
//! use crate::ui::test_utils::{render_widget, buffer_content};
//!
//! let widget = MyWidget::new();
//! let buffer = render_widget(widget, 80, 25);
//! let content = buffer_content(&buffer);
//! assert!(content.contains("expected text"));
//! ```

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::app::Model;
use crate::config::Theme;

/// Render a widget into a buffer of specified dimensions.
///
/// Creates an empty buffer of the given size, renders the widget into it,
/// and returns the buffer for inspection.
///
/// # Arguments
///
/// * `widget` - Any widget implementing the `Widget` trait
/// * `width` - Buffer width in columns
/// * `height` - Buffer height in rows
///
/// # Returns
///
/// A `Buffer` containing the rendered widget output
#[must_use]
pub fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    buffer
}

/// Extract text content from a buffer as a string.
///
/// Iterates through each cell in the buffer and extracts the first character
/// of each symbol. Each row is separated by a newline.
///
/// Useful for assertion checks on rendered output.
///
/// # Arguments
///
/// * `buffer` - The buffer to extract content from
///
/// # Returns
///
/// A `String` containing the text content of the buffer
#[must_use]
pub fn buffer_content(buffer: &Buffer) -> String {
    let mut content = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            content.push(
                buffer
                    .cell((x, y))
                    .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
            );
        }
        content.push('\n');
    }
    content
}

/// Create a default theme for testing.
///
/// Returns the default theme configuration, useful when tests
/// don't need specific theme customization.
#[must_use]
pub fn test_theme() -> Theme {
    Theme::default()
}

/// Create a default model for testing.
///
/// Returns an empty model with no tasks or projects.
#[must_use]
pub fn test_model() -> Model {
    Model::new()
}

/// Create a model with sample data for testing.
///
/// Returns a model populated with sample tasks, projects, and other data.
/// Useful for testing rendering of populated views.
#[must_use]
pub fn test_model_with_data() -> Model {
    Model::new().with_sample_data()
}
