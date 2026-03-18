//! Render a model into a ratatui Buffer using TestBackend.

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use taskflow::{app::Model, config::Theme, ui};

/// Render `model` at the given terminal size and return the resulting buffer.
pub fn render_view(model: &mut Model, theme: &Theme, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| ui::view(model, frame, theme))
        .unwrap();
    terminal.backend().buffer().clone()
}
