//! Weekly planner rendering methods.

use chrono::Datelike;
use ratatui::{
    buffer::Buffer,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use super::{DayColumnParams, WeeklyPlanner};

impl WeeklyPlanner<'_> {
    pub(crate) fn render_day_column(&self, buf: &mut Buffer, params: DayColumnParams<'_>) {
        let DayColumnParams {
            area,
            date,
            day_name,
            tasks,
            is_today,
            is_past,
            is_selected,
            selected_task_index,
        } = params;
        let theme = self.theme;

        // Determine title color based on day status
        let title_color = if is_today {
            theme.colors.accent.to_color()
        } else if is_past {
            theme.colors.muted.to_color()
        } else {
            theme.colors.foreground.to_color()
        };

        // Format title with day name and date
        let title = format!(" {} {} ({}) ", day_name, date.day(), tasks.len());

        // Selection takes precedence over today highlight for border
        let border_color = if is_selected {
            theme.colors.accent.to_color()
        } else if is_today {
            theme.colors.success.to_color()
        } else {
            theme.colors.border.to_color()
        };

        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(title_color).add_modifier(if is_today {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if tasks.is_empty() {
            // Show empty message
            let msg = if is_past { "—" } else { "No tasks" };
            let empty_msg =
                Paragraph::new(msg).style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(inner, buf);
            return;
        }

        // Create list items for tasks
        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .enumerate()
            .take(inner.height as usize)
            .map(|(idx, task)| {
                let is_selected_task = selected_task_index == Some(idx);
                // Status indicator
                let status_style = if task.status.is_complete() {
                    Style::default().fg(theme.colors.success.to_color())
                } else if is_past && !task.status.is_complete() {
                    Style::default().fg(theme.colors.danger.to_color())
                } else {
                    Style::default().fg(theme.colors.foreground.to_color())
                };

                let checkbox = if task.status.is_complete() {
                    "✓ "
                } else {
                    "◇ "
                };

                // Priority indicator (compact)
                let priority = match task.priority {
                    crate::domain::Priority::Urgent => "!",
                    crate::domain::Priority::High => "!",
                    _ => "",
                };

                let priority_style = match task.priority {
                    crate::domain::Priority::Urgent => {
                        Style::default().fg(theme.colors.danger.to_color())
                    }
                    crate::domain::Priority::High => {
                        Style::default().fg(theme.colors.warning.to_color())
                    }
                    _ => Style::default(),
                };

                // Truncate title to fit
                let max_len = inner.width.saturating_sub(4) as usize;
                let title = if task.title.len() > max_len {
                    format!("{}…", &task.title[..max_len.saturating_sub(1)])
                } else {
                    task.title.clone()
                };

                // Indicator for scheduled vs due
                let type_indicator =
                    if task.scheduled_date == Some(date) && task.due_date != Some(date) {
                        "○" // Scheduled only
                    } else if task.due_date == Some(date) {
                        "●" // Due date
                    } else {
                        ""
                    };

                let mut spans = vec![
                    Span::styled(checkbox, status_style),
                    Span::styled(priority, priority_style),
                ];

                if !type_indicator.is_empty() {
                    spans.push(Span::styled(
                        format!("{type_indicator} "),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ));
                }

                spans.push(Span::styled(title, status_style));

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
