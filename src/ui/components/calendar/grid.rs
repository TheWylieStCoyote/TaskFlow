//! Calendar grid rendering.

use chrono::NaiveDate;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use crate::config::Theme;

use super::Calendar;

impl Calendar<'_> {
    pub(crate) fn render_calendar_grid(
        &self,
        area: Rect,
        buf: &mut Buffer,
        today: NaiveDate,
        theme: &Theme,
    ) {
        let title = format!(
            " {} {} (</>) ",
            self.month_name(),
            self.model.calendar_state.year
        );

        // Highlight border when calendar grid has focus
        let border_color = if self.model.calendar_state.focus_task_list {
            theme.colors.muted.to_color()
        } else {
            theme.colors.accent.to_color()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

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

                let task_count = date.map_or(0, |d| self.model.task_count_for_day(d));
                let event_count = date.map_or(0, |d| self.model.events_for_day(d).len());
                let has_overdue = date.is_some_and(|d| self.model.has_overdue_on_day(d));

                // Determine style
                let is_today = date.is_some_and(|d| d == today);
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
                } else if event_count > 0 {
                    Style::default().fg(theme.colors.accent_secondary.to_color())
                } else {
                    Style::default().fg(theme.colors.foreground.to_color())
                };

                let day_str = format!("{day:2}");
                buf.set_string(x, y, &day_str, style);

                // Add indicator for tasks and/or events
                let has_items = task_count > 0 || event_count > 0;
                if has_items && x + 2 < inner.x + inner.width {
                    // Use different indicators:
                    // · = tasks only, ◆ = events only, ● = both
                    let indicator = match (task_count > 0, event_count > 0) {
                        (true, true) => "●",
                        (true, false) => "·",
                        (false, true) => "◆",
                        (false, false) => " ",
                    };
                    let indicator_style = if is_selected {
                        Style::default()
                            .bg(theme.colors.accent.to_color())
                            .fg(Color::Black)
                    } else if event_count > 0 && task_count == 0 {
                        Style::default().fg(theme.colors.accent_secondary.to_color())
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
}
