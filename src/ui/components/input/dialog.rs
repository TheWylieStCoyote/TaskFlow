//! Input dialog widget for text entry.

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph, Widget},
};

use crate::config::Theme;
use crate::ui::primitives::accent_block;

/// Input dialog for creating/editing items
pub struct InputDialog<'a> {
    title: &'a str,
    input: &'a str,
    cursor_position: usize,
    theme: &'a Theme,
}

impl<'a> InputDialog<'a> {
    #[must_use]
    pub const fn new(
        title: &'a str,
        input: &'a str,
        cursor_position: usize,
        theme: &'a Theme,
    ) -> Self {
        Self {
            title,
            input,
            cursor_position,
            theme,
        }
    }
}

impl Widget for InputDialog<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        // Build the input text with cursor indicator
        // Clamp cursor_position to valid char boundary to prevent panics
        let cursor = self.cursor_position.min(self.input.len());
        let cursor = if self.input.is_char_boundary(cursor) {
            cursor
        } else {
            // Find previous valid char boundary (manual implementation for MSRV compatibility)
            (0..cursor)
                .rev()
                .find(|&i| self.input.is_char_boundary(i))
                .unwrap_or(0)
        };
        let display_text = if cursor < self.input.len() {
            let (before, after) = self.input.split_at(cursor);
            let char_len = after.chars().next().map_or(1, char::len_utf8);
            let rest = &after[char_len..];
            format!("{before}▌{rest}")
        } else {
            format!("{}▌", self.input)
        };

        let paragraph = Paragraph::new(display_text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(accent_block(self.title, self.theme));

        paragraph.render(area, buf);
    }
}
