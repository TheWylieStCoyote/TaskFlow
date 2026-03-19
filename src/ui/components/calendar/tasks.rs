//! Calendar task list rendering.
//!
//! Displays the selected day's tasks and calendar events. Tasks scheduled
//! (time-blocked) on the day come first with an `HH:MM–HH:MM` label, tasks
//! that are only due that day follow under a `── Due ──` separator, then the
//! day's calendar events.

use crate::domain::text::truncate_chars;
use chrono::{Datelike, NaiveDate, NaiveTime};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::config::Theme;
use crate::domain::{Priority, Task};

use super::Calendar;

/// Width of the time-label column (`HH:MM–HH:MM`).
const TIME_LABEL_WIDTH: usize = 11;

/// Format a task's time block as a fixed-width `HH:MM–HH:MM` label.
///
/// A start without an end gives `HH:MM` padded to the column width; no start
/// gives a blank column so titles stay aligned.
pub(crate) fn format_time_label(start: Option<NaiveTime>, end: Option<NaiveTime>) -> String {
    match (start, end) {
        (Some(s), Some(e)) => format!("{}–{}", s.format("%H:%M"), e.format("%H:%M")),
        (Some(s), None) => format!("{:<TIME_LABEL_WIDTH$}", s.format("%H:%M")),
        (None, _) => " ".repeat(TIME_LABEL_WIDTH),
    }
}

/// Short title for the day panel: `Thu 19`, or `Thu 19 (2 events)`.
fn panel_title(date: Option<NaiveDate>, event_count: usize) -> String {
    match date {
        Some(d) if event_count > 0 => {
            format!(" {} {} ({event_count} events) ", d.weekday(), d.day())
        }
        Some(d) => format!(" {} {} ", d.weekday(), d.day()),
        None => " Tasks ".to_string(),
    }
}

fn priority_style(priority: Priority, theme: &Theme) -> Style {
    match priority {
        Priority::Urgent => Style::default().fg(theme.priority.urgent.to_color()),
        Priority::High => Style::default().fg(theme.priority.high.to_color()),
        Priority::Medium => Style::default().fg(theme.priority.medium.to_color()),
        Priority::Low => Style::default().fg(theme.priority.low.to_color()),
        Priority::None => Style::default(),
    }
}

/// One task row: priority, status, optional time label, truncated title.
fn task_line(task: &Task, time_label: Option<String>, width: u16, theme: &Theme) -> Line<'static> {
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

    // Columns before the title: priority, status and optional time label,
    // each followed by a space.
    let reserved = task.priority.symbol().chars().count()
        + task.status.symbol().chars().count()
        + time_label.as_ref().map_or(0, |l| l.chars().count() + 1)
        + 2;
    let max_title_len = (width as usize).saturating_sub(reserved);
    let title_display = truncate_chars(&task.title, max_title_len, "…");

    let mut spans = vec![
        Span::styled(
            format!("{} ", task.priority.symbol()),
            priority_style(task.priority, theme),
        ),
        Span::styled(format!("{} ", task.status.symbol()), status_style),
    ];
    if let Some(label) = time_label {
        spans.push(Span::styled(
            format!("{label} "),
            Style::default().fg(theme.colors.muted.to_color()),
        ));
    }
    spans.push(Span::styled(title_display, title_style));
    Line::from(spans)
}

impl Calendar<'_> {
    pub(crate) fn render_task_list(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let date = self.model.calendar_state.to_date();

        // Get events for the selected day
        let events = date
            .map(|d| self.model.events_for_day(d))
            .unwrap_or_default();

        let title = panel_title(date, events.len());

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

        // The day's tasks come from visible_tasks so that selected_index refers
        // to the same task here and in the actions. refresh_visible_tasks()
        // already orders them: scheduled (by start time) first, due-only after.
        let tasks: Vec<&Task> = if date.is_some() {
            self.model
                .visible_tasks
                .iter()
                .filter_map(|id| self.model.tasks.get(id))
                .collect()
        } else {
            Vec::new()
        };
        let scheduled_count = tasks
            .iter()
            .filter(|t| t.scheduled_date.is_some() && t.scheduled_date == date)
            .count();
        let due_only_count = tasks.len() - scheduled_count;

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

        let separator = |label: &str| {
            ListItem::new(Line::from(Span::styled(
                label.to_string(),
                Style::default().fg(theme.colors.muted.to_color()),
            )))
        };

        let mut items: Vec<ListItem<'_>> = Vec::new();

        // Scheduled tasks with their time block label
        for task in &tasks[..scheduled_count] {
            let label = format_time_label(task.scheduled_start_time, task.scheduled_end_time);
            items.push(ListItem::new(task_line(
                task,
                Some(label),
                inner.width,
                theme,
            )));
        }

        // Due-only tasks, separated when both sections are present
        let has_due_separator = scheduled_count > 0 && due_only_count > 0;
        if has_due_separator {
            items.push(separator("── Due ──"));
        }
        for task in &tasks[scheduled_count..] {
            items.push(ListItem::new(task_line(task, None, inner.width, theme)));
        }

        // Add separator and events if there are any
        if !events.is_empty() && !tasks.is_empty() {
            items.push(separator("── Events ──"));
        }

        // Add event items
        let remaining_slots = (inner.height as usize).saturating_sub(items.len());
        for event in events.iter().take(remaining_slots) {
            let max_title_len = inner.width.saturating_sub(12) as usize;
            let title_display = truncate_chars(&event.title, max_title_len, "…");

            let time_range = event.formatted_time_range();

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

        // Map the selected task to its row: the Due separator shifts the
        // due-only rows down by one.
        let selected = self.model.selected_index;
        let mut state = ListState::default();
        if selected < tasks.len() {
            let row = if has_due_separator && selected >= scheduled_count {
                selected + 1
            } else {
                selected
            };
            state.select(Some(row));
        }

        StatefulWidget::render(list, inner, buf, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Model, ViewId};
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

    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    /// Calendar model pinned to Thursday 2024-03-14 so the title and labels
    /// never depend on the wall clock.
    fn pinned_calendar_model() -> (Model, NaiveDate) {
        let date = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
        let mut model = Model::new();
        model.current_view = ViewId::Calendar;
        model.calendar_state.year = 2024;
        model.calendar_state.month = 3;
        model.calendar_state.selected_day = Some(14);
        (model, date)
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
    fn test_format_time_label_full_range() {
        assert_eq!(
            format_time_label(Some(time(9, 0)), Some(time(10, 30))),
            "09:00–10:30"
        );
        assert_eq!(
            format_time_label(Some(time(9, 0)), Some(time(10, 30)))
                .chars()
                .count(),
            11
        );
    }

    #[test]
    fn test_format_time_label_start_only_is_padded() {
        let label = format_time_label(Some(time(14, 5)), None);
        assert_eq!(label, "14:05      ");
        assert_eq!(label.chars().count(), 11);
    }

    #[test]
    fn test_format_time_label_without_start_is_blank_column() {
        assert_eq!(format_time_label(None, None), " ".repeat(11));
        assert_eq!(format_time_label(None, Some(time(10, 0))), " ".repeat(11));
    }

    #[test]
    fn test_panel_title_uses_weekday_and_day() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
        assert_eq!(panel_title(Some(date), 0), " Thu 14 ");
        assert_eq!(panel_title(Some(date), 2), " Thu 14 (2 events) ");
        assert_eq!(panel_title(None, 0), " Tasks ");
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
        let (model, _) = pinned_calendar_model();
        let content = render_tasks(&model, 40, 10);
        assert!(content.contains(" Thu 14 "));
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
        assert!(
            !content.contains("── Due ──"),
            "no separator without scheduled tasks"
        );
    }

    #[test]
    fn test_scheduled_tasks_show_time_label_before_title() {
        let (mut model, date) = pinned_calendar_model();
        let mut task = Task::new("Deep work");
        task.scheduled_date = Some(date);
        task.scheduled_start_time = Some(time(9, 0));
        task.scheduled_end_time = Some(time(10, 30));
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let content = render_tasks(&model, 50, 8);

        assert!(
            content.contains("09:00–10:30 Deep work"),
            "time label precedes the title: {content}"
        );
    }

    #[test]
    fn test_scheduled_tasks_are_ordered_by_start_time_with_due_only_after() {
        let (mut model, date) = pinned_calendar_model();
        let mut late = Task::new("Late block");
        late.scheduled_date = Some(date);
        late.scheduled_start_time = Some(time(15, 0));
        let mut early = Task::new("Early block");
        early.scheduled_date = Some(date);
        early.scheduled_start_time = Some(time(8, 0));
        early.scheduled_end_time = Some(time(9, 0));
        let mut untimed = Task::new("Untimed block");
        untimed.scheduled_date = Some(date);
        let mut due = Task::new("Just due");
        due.due_date = Some(date);
        for t in [late, early, untimed, due] {
            model.tasks.insert(t.id, t);
        }
        model.refresh_visible_tasks();

        let content = render_tasks(&model, 50, 10);

        let pos = |s: &str| {
            content
                .find(s)
                .unwrap_or_else(|| panic!("{s} missing: {content}"))
        };
        assert!(pos("08:00–09:00 Early block") < pos("15:00       Late block"));
        assert!(pos("Late block") < pos("Untimed block"));
        assert!(pos("Untimed block") < pos("── Due ──"));
        assert!(pos("── Due ──") < pos("Just due"));
    }

    #[test]
    fn test_task_due_and_scheduled_same_day_listed_once() {
        let (mut model, date) = pinned_calendar_model();
        let mut task = Task::new("Both");
        task.scheduled_date = Some(date);
        task.due_date = Some(date);
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let content = render_tasks(&model, 40, 8);
        assert_eq!(content.matches("Both").count(), 1);
        assert!(!content.contains("── Due ──"));
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
    fn test_long_scheduled_titles_leave_room_for_time_label() {
        let (mut model, date) = pinned_calendar_model();
        let mut task = Task::new("An exceptionally long scheduled task title");
        task.scheduled_date = Some(date);
        task.scheduled_start_time = Some(time(9, 0));
        task.scheduled_end_time = Some(time(10, 0));
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let content = render_tasks(&model, 30, 6);
        assert!(content.contains("09:00–10:00 An"));
        assert!(content.contains("…"));
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
        assert!(content.contains(&format!(" {} {} (3 events) ", d.weekday(), d.day())));
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
        let (mut model, _) = pinned_calendar_model();
        model.calendar_state.focus_task_list = true;
        let content = render_tasks(&model, 40, 8);
        assert!(content.contains(" Thu 14 "));
    }

    #[test]
    fn test_zero_height_inner_area_returns_early() {
        let model = Model::new();
        let content = render_tasks(&model, 40, 2);
        assert!(!content.contains("No tasks or events"));
    }
}
