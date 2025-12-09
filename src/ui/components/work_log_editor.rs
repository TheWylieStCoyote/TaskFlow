//! Work log editor popup widget.
//!
//! Displays and allows editing of work log entries for a task.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap},
};

use crate::config::Theme;
use crate::domain::{WorkLogEntry, WorkLogEntryId};

/// Mode for work log editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkLogMode {
    /// Browsing entries list
    #[default]
    Browse,
    /// Viewing a single entry's full content
    View,
    /// Adding a new entry (multi-line input)
    Add,
    /// Editing an existing entry (multi-line input)
    Edit,
    /// Confirming deletion
    ConfirmDelete,
    /// Searching/filtering entries
    Search,
}

/// Work log editor popup widget
pub struct WorkLogEditor<'a> {
    entries: Vec<&'a WorkLogEntry>,
    selected: usize,
    mode: WorkLogMode,
    edit_buffer: &'a [String],
    cursor_line: usize,
    cursor_col: usize,
    search_query: &'a str,
    theme: &'a Theme,
}

impl<'a> WorkLogEditor<'a> {
    #[must_use]
    pub fn new(
        entries: Vec<&'a WorkLogEntry>,
        selected: usize,
        mode: WorkLogMode,
        edit_buffer: &'a [String],
        cursor_line: usize,
        cursor_col: usize,
        search_query: &'a str,
        theme: &'a Theme,
    ) -> Self {
        Self {
            entries,
            selected,
            mode,
            edit_buffer,
            cursor_line,
            cursor_col,
            search_query,
            theme,
        }
    }

    /// Get the selected entry ID if any
    #[must_use]
    pub fn selected_entry_id(&self) -> Option<&WorkLogEntryId> {
        self.entries.get(self.selected).map(|e| &e.id)
    }
}

impl Widget for WorkLogEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        match self.mode {
            WorkLogMode::Browse => self.render_browse(area, buf),
            WorkLogMode::View => self.render_view(area, buf),
            WorkLogMode::Add | WorkLogMode::Edit => self.render_edit(area, buf),
            WorkLogMode::ConfirmDelete => self.render_confirm_delete(area, buf),
            WorkLogMode::Search => self.render_search(area, buf),
        }
    }
}

impl WorkLogEditor<'_> {
    fn render_browse(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Build list items
        let items: Vec<ListItem<'_>> = self
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = idx == self.selected;

                // Format: [timestamp] [summary...] [line count]
                let timestamp = entry.relative_time();
                let summary = entry.summary();
                let line_count = entry.line_count();

                let mut spans = vec![
                    Span::styled(
                        format!("{timestamp:<15}"),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        truncate_string(summary, 40),
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                ];

                if line_count > 1 {
                    spans.push(Span::styled(
                        format!(" ({line_count} lines)"),
                        Style::default()
                            .fg(theme.colors.muted.to_color())
                            .add_modifier(Modifier::ITALIC),
                    ));
                }

                let style = if is_selected {
                    Style::default()
                        .bg(theme.colors.accent_secondary.to_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(style)
            })
            .collect();

        // Show filter indicator if search is active
        let title = if self.search_query.is_empty() {
            " Work Log (a=add, /=search, Enter=view, e=edit, d=delete, Esc=close) ".to_string()
        } else {
            format!(
                " Work Log [filter: \"{}\"] (a=add, /=search, Esc=clear) ",
                truncate_string(self.search_query, 20)
            )
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));

        let list = if items.is_empty() {
            List::new(vec![ListItem::new(Line::from(Span::styled(
                "  No work log entries yet. Press 'a' to add one.",
                Style::default().fg(theme.colors.muted.to_color()),
            )))])
        } else {
            List::new(items)
        };

        let list = list.block(block);
        list.render(area, buf);
    }

    fn render_view(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let content = if let Some(entry) = self.entries.get(self.selected) {
            let header = format!(
                "{} ({})",
                entry.formatted_timestamp(),
                entry.relative_time()
            );
            let separator = "-".repeat(header.len().min(area.width as usize - 4));

            format!("{}\n{}\n\n{}", header, separator, entry.content)
        } else {
            "No entry selected".to_string()
        };

        let title = " View Entry (Enter/Esc=back, e=edit, d=delete) ";

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.colors.foreground.to_color()));

        paragraph.render(area, buf);
    }

    fn render_edit(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Render the multi-line edit buffer with cursor
        let mut lines: Vec<Line<'_>> = Vec::new();

        for (line_idx, line_text) in self.edit_buffer.iter().enumerate() {
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

        let title = match self.mode {
            WorkLogMode::Add => " Add Entry (Ctrl+S=save, Esc=cancel) ",
            WorkLogMode::Edit => " Edit Entry (Ctrl+S=save, Esc=cancel) ",
            _ => " Edit ",
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(lines).block(block);

        paragraph.render(area, buf);
    }

    fn render_confirm_delete(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let content = if let Some(entry) = self.entries.get(self.selected) {
            format!(
                "Delete this entry?\n\n{}\n\nPress 'y' to confirm, 'n' to cancel.",
                entry.summary()
            )
        } else {
            "No entry selected".to_string()
        };

        let title = " Confirm Delete ";

        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.colors.foreground.to_color()));

        paragraph.render(area, buf);
    }

    fn render_search(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::{Constraint, Direction, Layout};

        let theme = self.theme;

        // Split into search input (top) and results (bottom)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);

        // Render search input
        let search_block = Block::default()
            .title(" Search Work Log (Enter=apply, Esc=cancel) ")
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));

        let search_inner = search_block.inner(chunks[0]);
        search_block.render(chunks[0], buf);

        // Show search query with cursor
        let query = self.search_query;
        let cursor_char = if query.is_empty() { '_' } else { ' ' };
        let search_line = Line::from(vec![
            Span::styled("/ ", Style::default().fg(theme.colors.muted.to_color())),
            Span::raw(query),
            Span::styled(
                cursor_char.to_string(),
                Style::default()
                    .bg(theme.colors.accent.to_color())
                    .fg(theme.colors.background.to_color()),
            ),
        ]);

        buf.set_line(
            search_inner.x,
            search_inner.y,
            &search_line,
            search_inner.width,
        );

        // Render filtered results preview
        let results_block = Block::default()
            .title(format!(" {} matches ", self.entries.len()))
            .title_style(Style::default().fg(theme.colors.muted.to_color()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let results_inner = results_block.inner(chunks[1]);
        results_block.render(chunks[1], buf);

        // Show preview of matching entries
        let items: Vec<ListItem<'_>> = self
            .entries
            .iter()
            .take(results_inner.height as usize)
            .map(|entry| {
                let timestamp = entry.relative_time();
                let summary = entry.summary();
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{timestamp:<15}"),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        truncate_string(summary, 40),
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                ]))
            })
            .collect();

        if items.is_empty() {
            let no_results = Paragraph::new("No matching entries")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            no_results.render(results_inner, buf);
        } else {
            let list = List::new(items);
            list.render(results_inner, buf);
        }
    }
}

/// Truncate a string to a maximum length, adding ellipsis if needed.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use crate::domain::TaskId;
    use ratatui::buffer::Buffer;

    fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
        buffer
    }

    fn buffer_content(buffer: &Buffer) -> String {
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

    #[test]
    fn test_work_log_editor_empty() {
        let theme = Theme::default();
        let empty_buffer = vec![String::new()];
        let editor = WorkLogEditor::new(
            vec![],
            0,
            WorkLogMode::Browse,
            &empty_buffer,
            0,
            0,
            "",
            &theme,
        );
        let buffer = render_widget(editor, 70, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Work Log"));
        assert!(content.contains("No work log entries"));
    }

    #[test]
    fn test_work_log_editor_with_entry() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Test entry content");
        let empty_buffer = vec![String::new()];

        let editor = WorkLogEditor::new(
            vec![&entry],
            0,
            WorkLogMode::Browse,
            &empty_buffer,
            0,
            0,
            "",
            &theme,
        );
        let buffer = render_widget(editor, 80, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Work Log"));
        assert!(content.contains("Test entry"));
    }

    #[test]
    fn test_work_log_editor_view_mode() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Full content\nwith multiple\nlines");
        let empty_buffer = vec![String::new()];

        let editor = WorkLogEditor::new(
            vec![&entry],
            0,
            WorkLogMode::View,
            &empty_buffer,
            0,
            0,
            "",
            &theme,
        );
        let buffer = render_widget(editor, 80, 15);
        let content = buffer_content(&buffer);

        assert!(content.contains("View Entry"));
        assert!(content.contains("Full content"));
    }

    #[test]
    fn test_work_log_editor_add_mode() {
        let theme = Theme::default();
        let buffer_content_vec = vec!["First line".to_string(), "Second line".to_string()];

        let editor = WorkLogEditor::new(
            vec![],
            0,
            WorkLogMode::Add,
            &buffer_content_vec,
            0,
            5,
            "",
            &theme,
        );
        let buffer = render_widget(editor, 80, 15);
        let content = buffer_content(&buffer);

        assert!(content.contains("Add Entry"));
        assert!(content.contains("First line"));
    }

    #[test]
    fn test_work_log_editor_confirm_delete() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Entry to delete");
        let empty_buffer = vec![String::new()];

        let editor = WorkLogEditor::new(
            vec![&entry],
            0,
            WorkLogMode::ConfirmDelete,
            &empty_buffer,
            0,
            0,
            "",
            &theme,
        );
        let buffer = render_widget(editor, 80, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Confirm Delete"));
        assert!(content.contains("Entry to delete"));
    }

    #[test]
    fn test_work_log_search_mode() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let entry1 = WorkLogEntry::new(task_id, "Meeting notes from Monday");
        let entry2 = WorkLogEntry::new(task_id, "Bug fix details");
        let empty_buffer = vec![String::new()];

        let editor = WorkLogEditor::new(
            vec![&entry1, &entry2],
            0,
            WorkLogMode::Search,
            &empty_buffer,
            0,
            0,
            "meeting",
            &theme,
        );
        let buffer = render_widget(editor, 80, 15);
        let content = buffer_content(&buffer);

        assert!(content.contains("Search Work Log"));
        assert!(content.contains("2 matches")); // Shows count of filtered entries passed in
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("very long string here", 10), "very lo...");
    }
}
