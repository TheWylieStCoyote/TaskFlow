//! Calendar task list rendering.

use chrono::{Datelike, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::config::Theme;

use super::Calendar;

impl Calendar<'_> {
    pub(crate) fn render_task_list(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let selected_day = self.model.calendar_state.selected_day;
        let date = selected_day.and_then(|day| {
            NaiveDate::from_ymd_opt(
                self.model.calendar_state.year,
                self.model.calendar_state.month,
                day,
            )
        });

        let title = if let Some(d) = date {
            format!(" Tasks for {}/{} ", d.month(), d.day())
        } else {
            " Tasks ".to_string()
        };

        // Highlight border when task list has focus
        let border_color = if self.model.calendar_state.focus_task_list {
            theme.colors.accent.to_color()
        } else {
            theme.colors.muted.to_color()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        // Get tasks for the selected day using visible_tasks for consistency with selection
        // This ensures selected_index refers to the same task in both rendering and actions
        let tasks: Vec<_> = if date.is_some() {
            self.model
                .visible_tasks
                .iter()
                .filter_map(|id| self.model.tasks.get(id))
                .collect()
        } else {
            Vec::new()
        };

        if tasks.is_empty() {
            let msg = if date.is_some() {
                "No tasks due"
            } else {
                "Select a day"
            };
            buf.set_string(
                inner.x + 1,
                inner.y,
                msg,
                Style::default().fg(theme.colors.muted.to_color()),
            );
            return;
        }

        // Render task items
        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .take(inner.height as usize)
            .map(|task| {
                let status_style = if task.status.is_complete() {
                    Style::default().fg(theme.status.done.to_color())
                } else {
                    Style::default().fg(theme.status.pending.to_color())
                };

                let title_style = if task.status.is_complete() {
                    Style::default()
                        .fg(theme.colors.muted.to_color())
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                };

                let priority_symbol = task.priority.symbol();
                let priority_style = match task.priority {
                    crate::domain::Priority::Urgent => {
                        Style::default().fg(theme.priority.urgent.to_color())
                    }
                    crate::domain::Priority::High => {
                        Style::default().fg(theme.priority.high.to_color())
                    }
                    crate::domain::Priority::Medium => {
                        Style::default().fg(theme.priority.medium.to_color())
                    }
                    crate::domain::Priority::Low => {
                        Style::default().fg(theme.priority.low.to_color())
                    }
                    crate::domain::Priority::None => Style::default(),
                };

                // Truncate title to fit
                let max_title_len = inner.width.saturating_sub(8) as usize;
                let title_display = if task.title.len() > max_title_len {
                    format!("{}…", &task.title[..max_title_len.saturating_sub(1)])
                } else {
                    task.title.clone()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{priority_symbol} "), priority_style),
                    Span::styled(format!("{} ", task.status.symbol()), status_style),
                    Span::styled(title_display, title_style),
                ]))
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(self.theme.colors.accent_secondary.to_color())
                .add_modifier(Modifier::BOLD),
        );

        // Use selected_index for highlighting if in calendar view
        let mut state = ListState::default();
        if self.model.selected_index < tasks.len() {
            state.select(Some(self.model.selected_index));
        }

        StatefulWidget::render(list, inner, buf, &mut state);
    }
}
