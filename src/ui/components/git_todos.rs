//! Git TODOs view component.
//!
//! Displays tasks extracted from git repositories, grouped by source file.
//! Shows file headers with line numbers for easy navigation.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Git TODOs widget showing tasks grouped by source file.
pub struct GitTodos<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> GitTodos<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for GitTodos<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let grouped = self.model.get_git_tasks_grouped_by_file();

        // Create block with border
        let border_color = theme.colors.border.to_color();
        let block = Block::default()
            .title(" Git TODOs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if grouped.is_empty() {
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No git TODOs found.",
                    Style::default().fg(theme.colors.muted.to_color()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Run 'taskflow git-todos' to extract TODOs from a repository.",
                    Style::default().fg(theme.colors.muted.to_color()),
                )),
            ]);
            msg.render(inner, buf);
            return;
        }

        // Build flat list with file headers and tasks
        let mut items: Vec<ListItem<'_>> = Vec::new();
        let mut flat_index = 0;
        let selected_index = self.model.selected_index;

        for (file, tasks) in &grouped {
            // File header
            let file_display = if file.len() > 60 {
                format!("...{}", &file[file.len() - 57..])
            } else {
                file.clone()
            };

            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!("📁 {file_display}"),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )])));
            flat_index += 1;

            // Tasks under this file
            for (task_id, line_num) in tasks {
                if let Some(task) = self.model.tasks.get(task_id) {
                    let is_selected = self.model.selected_index == flat_index - 1; // -1 for header offset
                    let is_completed = task.completed_at.is_some();

                    // Truncate title if needed
                    let title = if task.title.len() > 50 {
                        format!("{}...", &task.title[..47])
                    } else {
                        task.title.clone()
                    };

                    // Style based on completion and selection
                    let (title_style, line_style) = if is_completed {
                        (
                            Style::default()
                                .fg(theme.colors.muted.to_color())
                                .add_modifier(Modifier::CROSSED_OUT),
                            Style::default().fg(theme.colors.muted.to_color()),
                        )
                    } else if is_selected {
                        (
                            Style::default()
                                .fg(theme.colors.foreground.to_color())
                                .add_modifier(Modifier::BOLD),
                            Style::default().fg(theme.colors.accent.to_color()),
                        )
                    } else {
                        (
                            Style::default().fg(theme.colors.foreground.to_color()),
                            Style::default().fg(theme.colors.muted.to_color()),
                        )
                    };

                    // Checkbox
                    let checkbox = if is_completed { "[x] " } else { "[ ] " };

                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!(":{line_num:<4} "), line_style),
                        Span::styled(checkbox, line_style),
                        Span::styled(title, title_style),
                    ])));
                }
                flat_index += 1;
            }

            // Add spacing between files
            items.push(ListItem::new(Line::from("")));
            flat_index += 1;
        }

        // Render the list with selection
        let list = List::new(items).highlight_style(
            Style::default()
                .bg(theme.colors.accent_secondary.to_color())
                .add_modifier(Modifier::BOLD),
        );

        // Use stateful rendering for selection highlight
        let mut state = ListState::default();
        state.select(Some(selected_index));
        StatefulWidget::render(list, inner, buf, &mut state);
    }
}
