//! Confirmation dialog widget.

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph, Widget},
};

use crate::config::Theme;
use crate::ui::primitives::warning_block;

/// Confirmation dialog
pub struct ConfirmDialog<'a> {
    title: &'a str,
    message: &'a str,
    theme: &'a Theme,
}

impl<'a> ConfirmDialog<'a> {
    #[must_use]
    pub const fn new(title: &'a str, message: &'a str, theme: &'a Theme) -> Self {
        Self {
            title,
            message,
            theme,
        }
    }
}

impl Widget for ConfirmDialog<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let text = format!("{}\n\n[y]es / [n]o", self.message);

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(warning_block(self.title, self.theme));

        paragraph.render(area, buf);
    }
}
