//! Work log editor rendering methods.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap},
};

use super::{truncate_string, WorkLogEditor, WorkLogMode};

impl WorkLogEditor<'_> {
    pub(crate) fn render_browse(&self, area: Rect, buf: &mut Buffer) {
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

    pub(crate) fn render_view(&self, area: Rect, buf: &mut Buffer) {
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

    pub(crate) fn render_edit(&self, area: Rect, buf: &mut Buffer) {
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

    pub(crate) fn render_confirm_delete(&self, area: Rect, buf: &mut Buffer) {
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

    pub(crate) fn render_search(&self, area: Rect, buf: &mut Buffer) {
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
