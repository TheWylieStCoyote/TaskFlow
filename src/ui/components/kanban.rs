//! Kanban board view component.
//!
//! Displays tasks in columns organized by status.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::TaskStatus;

/// Kanban board widget showing tasks in status columns.
pub struct Kanban<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Kanban<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for Kanban<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Define the columns (status categories)
        let columns = [
            (
                TaskStatus::Todo,
                "📋 Todo",
                theme.colors.foreground.to_color(),
            ),
            (
                TaskStatus::InProgress,
                "⏳ In Progress",
                theme.colors.accent.to_color(),
            ),
            (
                TaskStatus::Blocked,
                "🔒 Blocked",
                theme.colors.warning.to_color(),
            ),
            (TaskStatus::Done, "✅ Done", theme.colors.success.to_color()),
        ];

        // Split area into columns
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                columns
                    .iter()
                    .map(|_| Constraint::Percentage(25))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        // Render each column
        let selected_column = self.model.kanban_selected_column;
        for (i, (status, title, color)) in columns.iter().enumerate() {
            self.render_column(chunks[i], buf, *status, title, *color, i == selected_column);
        }
    }
}

impl Kanban<'_> {
    fn render_column(
        &self,
        area: Rect,
        buf: &mut Buffer,
        status: TaskStatus,
        title: &str,
        title_color: Color,
        is_selected: bool,
    ) {
        let theme = self.theme;

        // Get tasks for this column
        let tasks: Vec<_> = self
            .model
            .visible_tasks
            .iter()
            .filter_map(|id| self.model.tasks.get(id))
            .filter(|t| t.status == status)
            .collect();

        let count = tasks.len();

        // Create the column block with selection highlight
        let border_color = if is_selected {
            theme.colors.accent.to_color()
        } else {
            theme.colors.border.to_color()
        };

        let block = Block::default()
            .title(format!(" {} ({}) ", title, count))
            .title_style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if tasks.is_empty() {
            // Show empty message
            let empty_msg = Paragraph::new("No tasks")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(inner, buf);
            return;
        }

        // Create list items for each task
        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .map(|task| {
                let priority_indicator = match task.priority {
                    crate::domain::Priority::Urgent => {
                        Span::styled("!!!! ", Style::default().fg(theme.colors.danger.to_color()))
                    }
                    crate::domain::Priority::High => {
                        Span::styled("!!! ", Style::default().fg(theme.colors.warning.to_color()))
                    }
                    crate::domain::Priority::Medium => {
                        Span::styled("!! ", Style::default().fg(theme.colors.accent.to_color()))
                    }
                    crate::domain::Priority::Low => {
                        Span::styled("! ", Style::default().fg(theme.colors.muted.to_color()))
                    }
                    crate::domain::Priority::None => Span::raw(""),
                };

                // Truncate title if needed
                let max_len = inner.width.saturating_sub(6) as usize;
                let title = if task.title.len() > max_len {
                    format!("{}…", &task.title[..max_len.saturating_sub(1)])
                } else {
                    task.title.clone()
                };

                let mut spans = vec![priority_indicator];
                spans.push(Span::styled(
                    title,
                    Style::default().fg(theme.colors.foreground.to_color()),
                ));

                // Add due date indicator if overdue or due today
                if task.is_overdue() {
                    spans.push(Span::styled(
                        " ⚠",
                        Style::default()
                            .fg(theme.colors.danger.to_color())
                            .add_modifier(Modifier::BOLD),
                    ));
                } else if task.is_due_today() {
                    spans.push(Span::styled(
                        " !",
                        Style::default()
                            .fg(theme.colors.warning.to_color())
                            .add_modifier(Modifier::BOLD),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kanban_renders_without_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let kanban = Kanban::new(&model, &theme);

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        kanban.render(area, &mut buffer);

        // Basic assertion that something was rendered
        assert!(buffer.area.width > 0);
    }
}
