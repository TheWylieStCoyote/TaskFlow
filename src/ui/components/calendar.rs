use chrono::{Datelike, NaiveDate, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Calendar view widget showing a month grid with tasks
pub struct Calendar<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Calendar<'a> {
    pub fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get the first day of the month
    fn first_day_of_month(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(
            self.model.calendar_state.year,
            self.model.calendar_state.month,
            1,
        )
        .unwrap_or_else(|| Utc::now().date_naive())
    }

    /// Get the number of days in the current month
    fn days_in_month(&self) -> u32 {
        let year = self.model.calendar_state.year;
        let month = self.model.calendar_state.month;

        // Get the first day of next month, then subtract one day
        if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28)
    }

    /// Get the weekday of the first day (0=Mon, 6=Sun)
    fn first_weekday(&self) -> u32 {
        self.first_day_of_month().weekday().num_days_from_monday()
    }

    /// Get month name
    fn month_name(&self) -> &'static str {
        match self.model.calendar_state.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }
    }
}

impl Widget for Calendar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let today = Utc::now().date_naive();

        // Split into calendar grid and task list
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(24), // Calendar grid (at least 24 chars: 7 days * 3 chars + borders)
                Constraint::Length(30), // Task list panel
            ])
            .split(area);

        // Render calendar grid
        self.render_calendar_grid(chunks[0], buf, today, theme);

        // Render task list for selected day
        self.render_task_list(chunks[1], buf, theme);
    }
}

impl Calendar<'_> {
    fn render_calendar_grid(&self, area: Rect, buf: &mut Buffer, today: NaiveDate, theme: &Theme) {
        let title = format!(
            " {} {} (</>) ",
            self.month_name(),
            self.model.calendar_state.year
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 3 || inner.width < 21 {
            return; // Not enough space
        }

        // Day headers
        let header_style = Style::default()
            .fg(theme.colors.muted.to_color())
            .add_modifier(Modifier::BOLD);
        let headers = "Mo Tu We Th Fr Sa Su";
        let header_x = inner.x + (inner.width.saturating_sub(20)) / 2;
        buf.set_string(header_x, inner.y, headers, header_style);

        // Calendar grid
        let days_in_month = self.days_in_month();
        let first_weekday = self.first_weekday();
        let selected_day = self.model.calendar_state.selected_day;

        let mut day = 1u32;
        let mut row = 0u32;

        // Calculate cell width (we have 7 columns)
        let cell_width = 3u16;
        let grid_width = cell_width * 7;
        let start_x = inner.x + (inner.width.saturating_sub(grid_width)) / 2;

        while day <= days_in_month {
            let y = inner.y + 1 + row as u16;
            if y >= inner.y + inner.height {
                break;
            }

            for weekday in 0..7u32 {
                if row == 0 && weekday < first_weekday {
                    continue; // Empty cells before first day
                }
                if day > days_in_month {
                    break;
                }

                let x = start_x + (weekday as u16 * cell_width);
                let date = NaiveDate::from_ymd_opt(
                    self.model.calendar_state.year,
                    self.model.calendar_state.month,
                    day,
                );

                let task_count = date.map(|d| self.model.task_count_for_day(d)).unwrap_or(0);
                let has_overdue = date
                    .map(|d| self.model.has_overdue_on_day(d))
                    .unwrap_or(false);

                // Determine style
                let is_today = date.map(|d| d == today).unwrap_or(false);
                let is_selected = selected_day == Some(day);

                let style = if is_selected {
                    Style::default()
                        .bg(theme.colors.accent.to_color())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else if is_today {
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD)
                } else if has_overdue {
                    Style::default().fg(theme.colors.danger.to_color())
                } else if task_count > 0 {
                    Style::default().fg(theme.colors.warning.to_color())
                } else {
                    Style::default().fg(theme.colors.foreground.to_color())
                };

                let day_str = format!("{:2}", day);
                buf.set_string(x, y, &day_str, style);

                // Add task indicator
                if task_count > 0 && x + 2 < inner.x + inner.width {
                    let indicator = if task_count > 9 { "+" } else { "·" };
                    let indicator_style = if is_selected {
                        Style::default()
                            .bg(theme.colors.accent.to_color())
                            .fg(Color::Black)
                    } else {
                        Style::default().fg(theme.colors.muted.to_color())
                    };
                    buf.set_string(x + 2, y, indicator, indicator_style);
                }

                day += 1;
            }
            row += 1;
        }

        // Navigation hint at bottom
        if inner.y + inner.height > inner.y + 1 + row as u16 {
            let hint_y = inner.y + inner.height - 1;
            let hint = "←/→ day  ↑/↓ week  </> month";
            let hint_x = inner.x + (inner.width.saturating_sub(hint.len() as u16)) / 2;
            buf.set_string(
                hint_x,
                hint_y,
                hint,
                Style::default().fg(theme.colors.muted.to_color()),
            );
        }
    }

    fn render_task_list(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
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

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        // Get tasks for the selected day
        let tasks: Vec<_> = if let Some(d) = date {
            self.model.tasks_for_day(d)
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
        let items: Vec<ListItem> = tasks
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
                    Span::styled(format!("{} ", priority_symbol), priority_style),
                    Span::styled(format!("{} ", task.status.symbol()), status_style),
                    Span::styled(title_display, title_style),
                ]))
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
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
    use crate::config::Theme;
    use crate::domain::Task;

    /// Helper to render a widget into a buffer
    fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
        buffer
    }

    /// Extract text content from buffer
    fn buffer_content(buffer: &Buffer) -> String {
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push(
                    buffer
                        .cell((x, y))
                        .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
                );
            }
            content.push('\n');
        }
        content
    }

    #[test]
    fn test_calendar_renders_month_name() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 60, 20);
        let content = buffer_content(&buffer);

        // Should show current month name
        let current_month = model.calendar_state.month;
        let month_name = match current_month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };
        assert!(content.contains(month_name), "Month name should be visible");
    }

    #[test]
    fn test_calendar_renders_year() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 60, 20);
        let content = buffer_content(&buffer);

        let year = model.calendar_state.year.to_string();
        assert!(content.contains(&year), "Year should be visible");
    }

    #[test]
    fn test_calendar_renders_day_headers() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 60, 20);
        let content = buffer_content(&buffer);

        // Day headers: Mo Tu We Th Fr Sa Su
        assert!(content.contains("Mo"), "Monday header should be visible");
        assert!(content.contains("Tu"), "Tuesday header should be visible");
        assert!(content.contains("We"), "Wednesday header should be visible");
        assert!(content.contains("Th"), "Thursday header should be visible");
        assert!(content.contains("Fr"), "Friday header should be visible");
        assert!(content.contains("Sa"), "Saturday header should be visible");
        assert!(content.contains("Su"), "Sunday header should be visible");
    }

    #[test]
    fn test_calendar_renders_day_numbers() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 60, 20);
        let content = buffer_content(&buffer);

        // Should render day numbers
        assert!(content.contains(" 1"), "Day 1 should be visible");
        assert!(content.contains("15"), "Day 15 should be visible");
    }

    #[test]
    fn test_calendar_renders_navigation_hint() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 60, 20);
        let content = buffer_content(&buffer);

        // Navigation hints should be visible
        assert!(
            content.contains("day") || content.contains("week") || content.contains("month"),
            "Navigation hints should be visible"
        );
    }

    #[test]
    fn test_calendar_renders_task_panel() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 60, 20);
        let content = buffer_content(&buffer);

        // Task panel title "Tasks" or "Tasks for"
        assert!(content.contains("Tasks"), "Tasks panel should be visible");
    }

    #[test]
    fn test_calendar_shows_no_tasks_message() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 60, 20);
        let content = buffer_content(&buffer);

        // When no tasks, should show appropriate message
        assert!(
            content.contains("No tasks due") || content.contains("Select a day"),
            "Should show message when no tasks"
        );
    }

    #[test]
    fn test_calendar_renders_tasks_for_selected_day() {
        let mut model = Model::new();
        let today = Utc::now().date_naive();

        // Add a task due today
        let task = Task::new("Today's Task").with_due_date(today);
        let task_id = task.id.clone();
        model.tasks.insert(task_id, task);
        model.calendar_state.selected_day = Some(today.day());

        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 80, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Today's Task"),
            "Task title should be visible in calendar task list"
        );
    }

    #[test]
    fn test_calendar_with_sample_data() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        let buffer = render_widget(calendar, 80, 20);

        // Should render without panic
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_calendar_handles_small_area() {
        let model = Model::new();
        let theme = Theme::default();
        let calendar = Calendar::new(&model, &theme);
        // Very small area - should handle gracefully
        let buffer = render_widget(calendar, 20, 5);

        // Should render without panic
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_calendar_month_name_method() {
        let mut model = Model::new();
        let theme = Theme::default();

        // Test each month
        for month in 1..=12 {
            model.calendar_state.month = month;
            let calendar = Calendar::new(&model, &theme);
            let name = calendar.month_name();
            assert!(!name.is_empty(), "Month {} should have a name", month);
        }
    }

    #[test]
    fn test_calendar_days_in_month() {
        let mut model = Model::new();
        let theme = Theme::default();

        // Test typical months
        model.calendar_state.year = 2024;
        model.calendar_state.month = 1; // January
        let calendar = Calendar::new(&model, &theme);
        assert_eq!(calendar.days_in_month(), 31);

        model.calendar_state.month = 2; // February (leap year)
        let calendar = Calendar::new(&model, &theme);
        assert_eq!(calendar.days_in_month(), 29);

        model.calendar_state.month = 4; // April
        let calendar = Calendar::new(&model, &theme);
        assert_eq!(calendar.days_in_month(), 30);
    }
}
