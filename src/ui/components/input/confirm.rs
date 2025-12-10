//! Confirmation dialog widget.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::config::Theme;

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
        let warning = self.theme.colors.warning.to_color();

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
                    .title_style(Style::default().fg(warning).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(warning)),
            );

        paragraph.render(area, buf);
    }
}
