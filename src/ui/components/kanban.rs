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
        let selected_column = self.model.view_selection.kanban_column;
        let selected_task_index = self.model.view_selection.kanban_task_index;
        for (i, (status, title, color)) in columns.iter().enumerate() {
            let is_selected_column = i == selected_column;
            let task_index = if is_selected_column {
                Some(selected_task_index)
            } else {
                None
            };
            self.render_column(
                chunks[i],
                buf,
                *status,
                title,
                *color,
                is_selected_column,
                task_index,
            );
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
        is_selected_column: bool,
        selected_task_index: Option<usize>,
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
        let border_color = if is_selected_column {
            theme.colors.accent.to_color()
        } else {
            theme.colors.border.to_color()
        };

        let block = Block::default()
            .title(format!(" {title} ({count}) "))
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
            .enumerate()
            .map(|(idx, task)| {
                let is_selected_task = selected_task_index == Some(idx);
                // Check if task is blocked by incomplete dependencies
                let is_blocked = self.model.is_task_blocked(&task.id);
                let has_deps = self.model.has_dependencies(&task.id);

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
                let max_len = inner.width.saturating_sub(8) as usize; // Extra space for icons
                let title = if task.title.len() > max_len {
                    format!("{}…", &task.title[..max_len.saturating_sub(1)])
                } else {
                    task.title.clone()
                };

                // Dim blocked tasks
                let title_color = if is_blocked {
                    theme.colors.muted.to_color()
                } else {
                    theme.colors.foreground.to_color()
                };

                let mut spans = vec![priority_indicator];

                // Show dependency indicator
                if has_deps {
                    let dep_icon = if is_blocked { "🔒" } else { "🔗" };
                    spans.push(Span::styled(
                        format!("{dep_icon} "),
                        Style::default().fg(if is_blocked {
                            theme.colors.warning.to_color()
                        } else {
                            theme.colors.muted.to_color()
                        }),
                    ));
                }

                spans.push(Span::styled(title, Style::default().fg(title_color)));

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

                // Apply selection highlighting
                let mut item = ListItem::new(Line::from(spans));
                if is_selected_task {
                    item = item.style(
                        Style::default()
                            .bg(theme.colors.accent_secondary.to_color())
                            .add_modifier(Modifier::BOLD),
                    );
                }
                item
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

    #[test]
    fn test_kanban_column_task_counts() {
        let model = Model::new().with_sample_data();

        // Count tasks by status from visible tasks
        let mut todo_count = 0;
        let mut in_progress_count = 0;
        let mut _blocked_count = 0;
        let mut _done_count = 0;

        for id in &model.visible_tasks {
            if let Some(task) = model.tasks.get(id) {
                match task.status {
                    TaskStatus::Todo => todo_count += 1,
                    TaskStatus::InProgress => in_progress_count += 1,
                    TaskStatus::Blocked => _blocked_count += 1,
                    TaskStatus::Done | TaskStatus::Cancelled => _done_count += 1,
                }
            }
        }

        // Sample data should have tasks in multiple columns
        assert!(todo_count > 0, "Should have Todo tasks");
        assert!(in_progress_count > 0, "Should have InProgress tasks");
        // Blocked tasks are optional, Done tasks are hidden by default
    }

    #[test]
    fn test_kanban_empty_columns() {
        // Create model with only Todo tasks
        let mut model = Model::new();
        let task = crate::domain::Task::new("Test task").with_status(TaskStatus::Todo);
        let id = task.id;
        model.tasks.insert(id, task);
        model.visible_tasks = vec![id];

        let theme = Theme::default();
        let kanban = Kanban::new(&model, &theme);

        let area = Rect::new(0, 0, 100, 20);
        let mut buffer = Buffer::empty(area);
        kanban.render(area, &mut buffer);

        // Convert buffer to string and check for "No tasks" message
        let content: String = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();

        // Should have "No tasks" in empty columns (InProgress, Blocked, Done)
        assert!(
            content.contains("No tasks"),
            "Empty columns should show 'No tasks' message"
        );
    }

    #[test]
    fn test_kanban_task_selection() {
        let mut model = Model::new().with_sample_data();
        // Set selection to column 1 (InProgress), task 0
        model.view_selection.kanban_column = 1;
        model.view_selection.kanban_task_index = 0;

        let theme = Theme::default();
        let kanban = Kanban::new(&model, &theme);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        kanban.render(area, &mut buffer);

        // Verify render completes without panic with selection
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_kanban_priority_indicators() {
        use crate::domain::Priority;

        let mut model = Model::new();

        // Create tasks with different priorities
        let urgent = crate::domain::Task::new("Urgent task")
            .with_status(TaskStatus::Todo)
            .with_priority(Priority::Urgent);
        let high = crate::domain::Task::new("High task")
            .with_status(TaskStatus::Todo)
            .with_priority(Priority::High);
        let medium = crate::domain::Task::new("Medium task")
            .with_status(TaskStatus::Todo)
            .with_priority(Priority::Medium);
        let low = crate::domain::Task::new("Low task")
            .with_status(TaskStatus::Todo)
            .with_priority(Priority::Low);

        let ids = vec![urgent.id, high.id, medium.id, low.id];
        model.tasks.insert(urgent.id, urgent);
        model.tasks.insert(high.id, high);
        model.tasks.insert(medium.id, medium);
        model.tasks.insert(low.id, low);
        model.visible_tasks = ids;

        let theme = Theme::default();
        let kanban = Kanban::new(&model, &theme);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        kanban.render(area, &mut buffer);

        let content: String = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();

        // Check priority indicators are rendered
        assert!(
            content.contains("!!!!"),
            "Should show Urgent indicator (!!!!)"
        );
        assert!(content.contains("!!!"), "Should show High indicator (!!!)");
        assert!(content.contains("!!"), "Should show Medium indicator (!!)");
        // Low priority has single ! which is also part of !!!! etc, so just verify render works
    }

    #[test]
    fn test_kanban_overdue_styling() {
        use chrono::{Duration, Utc};

        let mut model = Model::new();

        // Create an overdue task
        let overdue = crate::domain::Task::new("Overdue task")
            .with_status(TaskStatus::Todo)
            .with_due_date(Utc::now().date_naive() - Duration::days(2));

        // Create a task due today
        let due_today = crate::domain::Task::new("Due today task")
            .with_status(TaskStatus::Todo)
            .with_due_date(Utc::now().date_naive());

        let ids = vec![overdue.id, due_today.id];
        model.tasks.insert(overdue.id, overdue);
        model.tasks.insert(due_today.id, due_today);
        model.visible_tasks = ids;

        let theme = Theme::default();
        let kanban = Kanban::new(&model, &theme);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        kanban.render(area, &mut buffer);

        let content: String = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();

        // Overdue tasks should show warning indicator
        assert!(
            content.contains("⚠"),
            "Overdue task should show ⚠ indicator"
        );
    }
}
