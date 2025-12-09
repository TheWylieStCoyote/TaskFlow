//! Eisenhower matrix view component.
//!
//! Displays tasks in a 2x2 grid based on urgency and importance.
//! - Urgent + Important: Do First
//! - Not Urgent + Important: Schedule
//! - Urgent + Not Important: Delegate
//! - Not Urgent + Not Important: Eliminate

use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Priority, Task};

/// Eisenhower matrix widget showing tasks in urgency/importance quadrants.
pub struct Eisenhower<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Eisenhower<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Determine if a task is "urgent" based on due date.
    /// Urgent = due within 2 days or overdue.
    fn is_urgent(task: &Task) -> bool {
        task.due_date.is_some_and(|due| {
            let today = Utc::now().date_naive();
            let days_until = (due - today).num_days();
            days_until <= 2
        })
    }

    /// Determine if a task is "important" based on priority.
    /// Important = High or Urgent priority.
    fn is_important(task: &Task) -> bool {
        matches!(task.priority, Priority::High | Priority::Urgent)
    }
}

impl Widget for Eisenhower<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Split into 2 rows
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Split each row into 2 columns
        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);

        let bottom_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        // Get all visible tasks
        let tasks: Vec<_> = self
            .model
            .visible_tasks
            .iter()
            .filter_map(|id| self.model.tasks.get(id))
            .filter(|t| !t.status.is_complete())
            .collect();

        // Categorize tasks
        let (
            urgent_important,
            not_urgent_important,
            urgent_not_important,
            not_urgent_not_important,
        ): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) = {
            let mut ui = Vec::new();
            let mut nui = Vec::new();
            let mut uni = Vec::new();
            let mut nuni = Vec::new();

            for task in tasks {
                let urgent = Self::is_urgent(task);
                let important = Self::is_important(task);

                match (urgent, important) {
                    (true, true) => ui.push(task),
                    (false, true) => nui.push(task),
                    (true, false) => uni.push(task),
                    (false, false) => nuni.push(task),
                }
            }

            (ui, nui, uni, nuni)
        };

        // Render quadrants (0=TL, 1=TR, 2=BL, 3=BR)
        let selected = self.model.eisenhower_selected_quadrant;

        // Top-left: Urgent + Important (DO FIRST)
        self.render_quadrant(
            top_cols[0],
            buf,
            "🔥 DO FIRST",
            "Urgent & Important",
            &urgent_important,
            theme.colors.danger.to_color(),
            selected == 0,
        );

        // Top-right: Not Urgent + Important (SCHEDULE)
        self.render_quadrant(
            top_cols[1],
            buf,
            "📅 SCHEDULE",
            "Important, Not Urgent",
            &not_urgent_important,
            theme.colors.accent.to_color(),
            selected == 1,
        );

        // Bottom-left: Urgent + Not Important (DELEGATE)
        self.render_quadrant(
            bottom_cols[0],
            buf,
            "👋 DELEGATE",
            "Urgent, Not Important",
            &urgent_not_important,
            theme.colors.warning.to_color(),
            selected == 2,
        );

        // Bottom-right: Not Urgent + Not Important (ELIMINATE)
        self.render_quadrant(
            bottom_cols[1],
            buf,
            "🗑️  ELIMINATE",
            "Not Urgent or Important",
            &not_urgent_not_important,
            theme.colors.muted.to_color(),
            selected == 3,
        );
    }
}

impl Eisenhower<'_> {
    fn render_quadrant(
        &self,
        area: Rect,
        buf: &mut Buffer,
        title: &str,
        subtitle: &str,
        tasks: &[&Task],
        title_color: Color,
        is_selected: bool,
    ) {
        let theme = self.theme;

        let border_color = if is_selected {
            theme.colors.accent.to_color()
        } else {
            theme.colors.border.to_color()
        };

        let block = Block::default()
            .title(format!(" {} ({}) ", title, tasks.len()))
            .title_style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 2 {
            return;
        }

        // Show subtitle on first line
        let subtitle_line = Line::from(Span::styled(
            subtitle,
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        buf.set_line(inner.x, inner.y, &subtitle_line, inner.width);

        let tasks_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };

        if tasks.is_empty() {
            let empty_msg = Paragraph::new("No tasks")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(tasks_area, buf);
            return;
        }

        // Create list items
        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .take(tasks_area.height as usize) // Limit to visible area
            .map(|task| {
                // Check if task is blocked by incomplete dependencies
                let is_blocked = self.model.is_task_blocked(&task.id);
                let has_deps = self.model.has_dependencies(&task.id);

                let status_icon = match task.status {
                    crate::domain::TaskStatus::Todo => "◇",
                    crate::domain::TaskStatus::InProgress => "⊙",
                    crate::domain::TaskStatus::Blocked => "⊠",
                    crate::domain::TaskStatus::Done => "✓",
                    crate::domain::TaskStatus::Cancelled => "✕",
                };

                // Truncate title if needed
                let max_len = tasks_area.width.saturating_sub(6) as usize;
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

                let mut spans = vec![Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(theme.colors.muted.to_color()),
                )];

                // Show dependency indicator
                if has_deps {
                    let dep_icon = if is_blocked { "🔒" } else { "🔗" };
                    spans.push(Span::styled(
                        format!("{} ", dep_icon),
                        Style::default().fg(if is_blocked {
                            theme.colors.warning.to_color()
                        } else {
                            theme.colors.muted.to_color()
                        }),
                    ));
                }

                spans.push(Span::styled(title, Style::default().fg(title_color)));

                // Add overdue indicator
                if task.is_overdue() {
                    spans.push(Span::styled(
                        " ⚠",
                        Style::default().fg(theme.colors.danger.to_color()),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items);
        list.render(tasks_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eisenhower_renders_without_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let matrix = Eisenhower::new(&model, &theme);

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        matrix.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_urgency_detection() {
        let mut task = Task::new("Test");

        // No due date = not urgent
        assert!(!Eisenhower::is_urgent(&task));

        // Due in 5 days = not urgent
        let future = Utc::now().date_naive() + chrono::Duration::days(5);
        task.due_date = Some(future);
        assert!(!Eisenhower::is_urgent(&task));

        // Due tomorrow = urgent
        let tomorrow = Utc::now().date_naive() + chrono::Duration::days(1);
        task.due_date = Some(tomorrow);
        assert!(Eisenhower::is_urgent(&task));

        // Overdue = urgent
        let yesterday = Utc::now().date_naive() - chrono::Duration::days(1);
        task.due_date = Some(yesterday);
        assert!(Eisenhower::is_urgent(&task));
    }

    #[test]
    fn test_importance_detection() {
        let mut task = Task::new("Test");

        // Default priority = not important
        assert!(!Eisenhower::is_important(&task));

        task.priority = Priority::Low;
        assert!(!Eisenhower::is_important(&task));

        task.priority = Priority::Medium;
        assert!(!Eisenhower::is_important(&task));

        task.priority = Priority::High;
        assert!(Eisenhower::is_important(&task));

        task.priority = Priority::Urgent;
        assert!(Eisenhower::is_important(&task));
    }
}
