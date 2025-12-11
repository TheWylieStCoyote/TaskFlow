//! Multi-line description editor popup widget.
//!
//! Displays a multi-line text editor for task descriptions.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::config::Theme;

/// Multi-line description editor popup widget
pub struct DescriptionEditor<'a> {
    buffer: &'a [String],
    cursor_line: usize,
    cursor_col: usize,
    theme: &'a Theme,
}

impl<'a> DescriptionEditor<'a> {
    #[must_use]
    pub fn new(
        buffer: &'a [String],
        cursor_line: usize,
        cursor_col: usize,
        theme: &'a Theme,
    ) -> Self {
        Self {
            buffer,
            cursor_line,
            cursor_col,
            theme,
        }
    }
}

impl Widget for DescriptionEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        let theme = self.theme;

        // Render the multi-line edit buffer with cursor
        let mut lines: Vec<Line<'_>> = Vec::new();

        for (line_idx, line_text) in self.buffer.iter().enumerate() {
            let is_cursor_line = line_idx == self.cursor_line;

            if is_cursor_line {
                // Show cursor position
                let cursor_col = self.cursor_col.min(line_text.len());
                let before_cursor = &line_text[..cursor_col];
                let cursor_char = line_text.chars().nth(cursor_col).unwrap_or(' ');
                let after_cursor = if cursor_col < line_text.len() {
                    &line_text[cursor_col + cursor_char.len_utf8()..]
                } else {
                    ""
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:>3} ", line_idx + 1),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::raw(before_cursor),
                    Span::styled(
                        cursor_char.to_string(),
                        Style::default()
                            .bg(theme.colors.accent.to_color())
                            .fg(theme.colors.background.to_color()),
                    ),
                    Span::raw(after_cursor),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:>3} ", line_idx + 1),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::raw(line_text.as_str()),
                ]));
            }
        }

        let title = " Edit Description (Ctrl+S=save, Esc=cancel) ";

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.colors.warning.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.warning.to_color()));

        let paragraph = Paragraph::new(lines).block(block);

        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use crate::ui::test_utils::{buffer_content, render_widget};

    #[test]
    fn test_description_editor_empty() {
        let theme = Theme::default();
        let buffer = vec![String::new()];
        let editor = DescriptionEditor::new(&buffer, 0, 0, &theme);
        let rendered = render_widget(editor, 70, 10);
        let content = buffer_content(&rendered);

        assert!(content.contains("Edit Description"));
    }

    #[test]
    fn test_description_editor_with_content() {
        let theme = Theme::default();
        let buffer = vec![
            "First line of description".to_string(),
            "Second line".to_string(),
        ];
        let editor = DescriptionEditor::new(&buffer, 0, 5, &theme);
        let rendered = render_widget(editor, 80, 10);
        let content = buffer_content(&rendered);

        assert!(content.contains("Edit Description"));
        assert!(content.contains("First line"));
    }

    #[test]
    fn test_description_editor_cursor_on_second_line() {
        let theme = Theme::default();
        let buffer = vec!["Line one".to_string(), "Line two".to_string()];
        let editor = DescriptionEditor::new(&buffer, 1, 3, &theme);
        let rendered = render_widget(editor, 80, 10);
        let content = buffer_content(&rendered);

        assert!(content.contains("Line one"));
        assert!(content.contains("Line two"));
    }
}
