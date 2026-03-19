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

#[cfg(test)]
fn buffer_to_string(buffer: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            result.push_str(buffer[(x, y)].symbol());
        }
        result.push('\n');
    }
    result
}

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
                    "  Press `r` to scan git repository for TODOs.",
                    Style::default().fg(theme.colors.muted.to_color()),
                )),
            ]);
            msg.render(inner, buf);
            return;
        }

        // Build flat list with file headers and tasks
        let mut items: Vec<ListItem<'_>> = Vec::new();
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

            // Tasks under this file
            for (task_id, line_num) in tasks {
                if let Some(task) = self.model.tasks.get(task_id) {
                    // Check if this task's position matches selected_index
                    let is_selected = selected_index == items.len();
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
            }

            // Add spacing between files
            items.push(ListItem::new(Line::from("")));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;

    #[test]
    fn test_git_todos_renders_empty_message() {
        let model = Model::new();
        let theme = Theme::default();
        let widget = GitTodos::new(&model, &theme);

        let area = Rect::new(0, 0, 60, 15);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains("Git TODOs"));
        assert!(content.contains("No git TODOs found"));
    }

    #[test]
    fn test_git_todos_renders_scan_hint_when_empty() {
        let model = Model::new();
        let theme = Theme::default();
        let widget = GitTodos::new(&model, &theme);

        let area = Rect::new(0, 0, 60, 15);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains('r'));
    }

    #[test]
    fn test_git_todos_renders_without_panic_small_area() {
        let model = Model::new();
        let theme = Theme::default();
        let widget = GitTodos::new(&model, &theme);

        let area = Rect::new(0, 0, 10, 5);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer); // Should not panic
    }

    #[test]
    fn test_git_todos_with_task_renders_title() {
        use crate::domain::Task;

        let mut model = Model::new();
        let mut task = Task::new("TODO: fix this bug");
        // Git tasks use description with "git:<file>:<line>" format
        task.description = Some("git:src/main.rs:42".to_string());
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let widget = GitTodos::new(&model, &theme);

        let area = Rect::new(0, 0, 80, 20);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer); // Should not panic
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Git TODOs"));
        // Should show the file path in header
        assert!(content.contains("src/main.rs") || content.contains("main.rs"));
    }

    #[test]
    fn test_git_todos_shows_task_title() {
        use crate::domain::Task;

        let mut model = Model::new();
        let mut task = Task::new("Fix the auth bug");
        task.description = Some("git:src/auth.rs:10".to_string());
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let widget = GitTodos::new(&model, &theme);

        let area = Rect::new(0, 0, 80, 25);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains("Fix the auth bug"));
    }
}
