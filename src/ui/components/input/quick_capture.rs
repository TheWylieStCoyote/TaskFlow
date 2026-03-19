//! Quick capture dialog with syntax hints.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Widget},
};

use crate::config::Theme;
use crate::ui::primitives::accent_block;

/// Quick capture dialog with syntax hints
pub struct QuickCaptureDialog<'a> {
    input: &'a str,
    cursor_position: usize,
    theme: &'a Theme,
}

impl<'a> QuickCaptureDialog<'a> {
    #[must_use]
    pub const fn new(input: &'a str, cursor_position: usize, theme: &'a Theme) -> Self {
        Self {
            input,
            cursor_position,
            theme,
        }
    }
}

impl Widget for QuickCaptureDialog<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let block = accent_block("Quick Capture (Esc to close, Enter to add)", self.theme);

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into input line and hints
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        // Render input with cursor
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

        let input_line = Paragraph::new(display_text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()));
        input_line.render(chunks[0], buf);

        // Render hints using theme colors
        let hints = [
            Line::from(vec![
                Span::styled(
                    "#tag ",
                    Style::default().fg(self.theme.colors.success.to_color()),
                ),
                Span::styled(
                    "@project ",
                    Style::default().fg(self.theme.priority.high.to_color()),
                ),
                Span::styled(
                    "!priority ",
                    Style::default().fg(self.theme.colors.warning.to_color()),
                ),
                Span::styled(
                    "due:date ",
                    Style::default().fg(self.theme.colors.danger.to_color()),
                ),
                Span::styled(
                    "sched:date ",
                    Style::default().fg(self.theme.colors.accent.to_color()),
                ),
                Span::styled(
                    "at:time",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Examples: ",
                    Style::default()
                        .fg(self.theme.colors.muted.to_color())
                        .add_modifier(Modifier::ITALIC),
                ),
                Span::styled(
                    "Buy milk #groceries @Home !high due:+3d at 9am",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]),
        ];

        for (i, line) in hints.iter().enumerate() {
            if i + 2 < chunks.len() {
                let hint_para = Paragraph::new(line.clone());
                hint_para.render(chunks[i + 2], buf);
            }
        }
    }
}
