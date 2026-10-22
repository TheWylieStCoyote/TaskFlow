//! Calendar task list rendering.
//!
//! Displays tasks and calendar events for the selected day.

use crate::domain::text::truncate_chars;
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

        // Get events for the selected day
        let events = date
            .map(|d| self.model.events_for_day(d))
            .unwrap_or_default();
        let event_count = events.len();

        let title = if let Some(d) = date {
            if event_count > 0 {
                format!(" {}/{} ({} events) ", d.month(), d.day(), event_count)
            } else {
                format!(" Tasks for {}/{} ", d.month(), d.day())
            }
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

        if tasks.is_empty() && events.is_empty() {
            let msg = if date.is_some() {
                "No tasks or events"
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

        // Render task items first
        let mut items: Vec<ListItem<'_>> = tasks
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
                let title_display = truncate_chars(&task.title, max_title_len, "…");

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{priority_symbol} "), priority_style),
                    Span::styled(format!("{} ", task.status.symbol()), status_style),
                    Span::styled(title_display, title_style),
                ]))
            })
            .collect();

        // Add separator and events if there are any
        if !events.is_empty() && !tasks.is_empty() {
            // Add a visual separator between tasks and events
            items.push(ListItem::new(Line::from(Span::styled(
                "── Events ──",
                Style::default().fg(theme.colors.muted.to_color()),
            ))));
        }

        // Add event items
        let remaining_slots = inner.height as usize - items.len();
        for event in events.iter().take(remaining_slots) {
            // Truncate title to fit
            let max_title_len = inner.width.saturating_sub(12) as usize;
            let title_display = truncate_chars(&event.title, max_title_len, "…");

            // Format time range
            let time_range = event.formatted_time_range();

            // Style based on event status
            let event_style = match event.status {
                crate::domain::CalendarEventStatus::Tentative => {
                    Style::default().fg(theme.colors.muted.to_color())
                }
                crate::domain::CalendarEventStatus::Cancelled => Style::default()
                    .fg(theme.colors.muted.to_color())
                    .add_modifier(Modifier::CROSSED_OUT),
                crate::domain::CalendarEventStatus::Confirmed => Style::default(),
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled("📅 ", Style::default()),
                Span::styled(
                    format!("{time_range} "),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(title_display, event_style),
            ])));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::domain::{today, CalendarEvent, CalendarEventStatus, Priority, Task, TaskStatus};
    use crate::ui::test_utils::buffer_content;
    use chrono::{Datelike, Local, TimeZone, Utc};

    fn render_tasks(model: &Model, width: u16, height: u16) -> String {
        let theme = Theme::default();
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        Calendar::new(model, &theme).render_task_list(area, &mut buf, &theme);
        buffer_content(&buf)
    }

    /// Build an event that starts today in local time (events are matched to
    /// days by local date).
    fn event_today(title: &str, hour: u32, status: CalendarEventStatus) -> CalendarEvent {
        let date = today();
        let start = Local
            .with_ymd_and_hms(date.year(), date.month(), date.day(), hour, 0, 0)
            .single()
            .expect("valid local time")
            .with_timezone(&Utc);
        CalendarEvent::new(title)
            .with_start(start)
            .with_end(start + chrono::Duration::minutes(45))
            .with_status(status)
    }

    #[test]
    fn test_no_selected_day_prompts_selection() {
        let mut model = Model::new();
        model.calendar_state.selected_day = None;
        let content = render_tasks(&model, 40, 10);
        assert!(content.contains(" Tasks "));
        assert!(content.contains("Select a day"));
    }

    #[test]
    fn test_selected_day_without_items() {
        let model = Model::new();
        let content = render_tasks(&model, 40, 10);
        let d = today();
        assert!(content.contains(&format!(" Tasks for {}/{} ", d.month(), d.day())));
        assert!(content.contains("No tasks or events"));
    }

    #[test]
    fn test_tasks_render_with_priority_and_status_symbols() {
        let mut model = Model::new();
        let mut urgent = Task::new("Urgent thing");
        urgent.priority = Priority::Urgent;
        urgent.due_date = Some(today());
        let mut done = Task::new("Done thing");
        done.status = TaskStatus::Done;
        done.due_date = Some(today());
        model.tasks.insert(urgent.id, urgent);
        model.tasks.insert(done.id, done);
        model.filtering.show_completed = true;
        model.refresh_visible_tasks();
        model.selected_index = 0;

        let content = render_tasks(&model, 50, 10);

        assert!(content.contains("Urgent thing"));
        assert!(content.contains("Done thing"));
        assert!(content.contains(&format!("{} ", Priority::Urgent.symbol())));
        assert!(content.contains(&format!("{} ", TaskStatus::Done.symbol())));
        assert!(!content.contains("── Events ──"));
    }

    #[test]
    fn test_long_task_titles_are_truncated() {
        let mut model = Model::new();
        let task = Task::new("An exceptionally long task title that cannot fit in the column");
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let content = render_tasks(&model, 30, 6);

        assert!(content.contains("…"));
        assert!(!content.contains("cannot fit"));
    }

    #[test]
    fn test_events_render_after_tasks_with_separator() {
        let mut model = Model::new();
        let task = Task::new("Morning task");
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();
        let confirmed = event_today("Standup", 9, CalendarEventStatus::Confirmed);
        let standup_range = confirmed.formatted_time_range();
        let tentative = event_today("Maybe lunch", 12, CalendarEventStatus::Tentative);
        let cancelled = event_today("Cancelled sync", 15, CalendarEventStatus::Cancelled);
        for event in [confirmed, tentative, cancelled] {
            model.calendar_events.insert(event.id, event);
        }

        let content = render_tasks(&model, 60, 12);

        let d = today();
        assert!(content.contains(&format!(" {}/{} (3 events) ", d.month(), d.day())));
        assert!(content.contains("Morning task"));
        assert!(content.contains("── Events ──"));
        assert!(content.contains("📅"));
        // The time range is rendered in UTC by `formatted_time_range`
        assert!(content.contains(&format!("{standup_range} Standup")));
        assert!(content.contains("Maybe lunch"));
        assert!(content.contains("Cancelled sync"));
        let task_pos = content.find("Morning task").unwrap();
        let sep_pos = content.find("── Events ──").unwrap();
        let event_pos = content.find("Standup").unwrap();
        assert!(task_pos < sep_pos && sep_pos < event_pos);
    }

    #[test]
    fn test_events_only_day_has_no_separator() {
        let mut model = Model::new();
        let event = event_today("Solo event", 10, CalendarEventStatus::Confirmed);
        model.calendar_events.insert(event.id, event);

        let content = render_tasks(&model, 50, 8);

        assert!(content.contains("Solo event"));
        assert!(!content.contains("── Events ──"));
        assert!(!content.contains("No tasks or events"));
    }

    #[test]
    fn test_all_day_event_label() {
        let mut model = Model::new();
        let event = event_today("Holiday", 0, CalendarEventStatus::Confirmed).with_all_day(true);
        model.calendar_events.insert(event.id, event);

        let content = render_tasks(&model, 50, 8);
        assert!(content.contains("All day Holiday"));
    }

    #[test]
    fn test_event_rows_are_limited_by_height() {
        let mut model = Model::new();
        for i in 0..10 {
            let event = event_today(&format!("Event {i}"), 8 + i, CalendarEventStatus::Confirmed);
            model.calendar_events.insert(event.id, event);
        }

        // inner height is 3 rows: only the first three events fit
        let content = render_tasks(&model, 50, 5);
        assert!(content.contains("Event 0"));
        assert!(content.contains("Event 2"));
        assert!(!content.contains("Event 3"));
    }

    #[test]
    fn test_focused_task_list_still_renders_title() {
        let mut model = Model::new();
        model.calendar_state.focus_task_list = true;
        let content = render_tasks(&model, 40, 8);
        assert!(content.contains("Tasks for"));
    }

    #[test]
    fn test_zero_height_inner_area_returns_early() {
        let model = Model::new();
        let content = render_tasks(&model, 40, 2);
        assert!(!content.contains("No tasks or events"));
    }
}
