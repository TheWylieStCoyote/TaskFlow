//! Habit tracking view component.
//!
//! Displays habits with streaks, check-in calendar, and analytics.

use chrono::{TimeDelta, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Habit tracking view widget.
pub struct HabitsView<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> HabitsView<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for HabitsView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Split into habit list (left) and analytics (right)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_habit_list(chunks[0], buf);
        self.render_analytics(chunks[1], buf);
    }
}

impl HabitsView<'_> {
    fn render_habit_list(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let today = Utc::now().date_naive();

        let title = if self.model.show_archived_habits {
            " Habits (showing archived) "
        } else {
            " Habits "
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.model.visible_habits.is_empty() {
            let empty_msg = Paragraph::new("No habits yet. Press 'n' to create one.")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(inner, buf);
            return;
        }

        // Create list items for each habit
        let items: Vec<ListItem<'_>> = self
            .model
            .visible_habits
            .iter()
            .enumerate()
            .filter_map(|(idx, id)| {
                let habit = self.model.habits.get(id)?;
                let is_selected = idx == self.model.habit_selected;

                // Check-in status for today
                let completed_today = habit.is_completed_on(today);
                let checkbox = if completed_today {
                    Span::styled("[x] ", Style::default().fg(theme.colors.success.to_color()))
                } else if habit.frequency.is_due_on(today, habit.start_date) {
                    Span::styled("[ ] ", Style::default().fg(theme.colors.muted.to_color()))
                } else {
                    Span::styled("[·] ", Style::default().fg(theme.colors.muted.to_color()))
                };

                // Habit name (with archived indicator)
                let name_style = if habit.archived {
                    Style::default()
                        .fg(theme.colors.muted.to_color())
                        .add_modifier(Modifier::ITALIC)
                } else if is_selected {
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.foreground.to_color())
                };
                let name = Span::styled(&habit.name, name_style);

                // Current streak
                let streak = habit.current_streak();
                let streak_text = if streak > 0 {
                    format!(" {}d", streak)
                } else {
                    String::new()
                };
                let streak_color = if streak >= 7 {
                    theme.colors.success.to_color()
                } else if streak >= 3 {
                    theme.colors.accent.to_color()
                } else {
                    theme.colors.muted.to_color()
                };
                let streak_span = Span::styled(streak_text, Style::default().fg(streak_color));

                // 7-day calendar visualization
                let calendar = self.render_week_calendar(habit);

                let line = Line::from(vec![checkbox, name, streak_span, Span::raw("  "), calendar]);

                let item = if is_selected {
                    ListItem::new(line).style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .fg(theme.colors.foreground.to_color()),
                    )
                } else {
                    ListItem::new(line)
                };

                Some(item)
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
    }

    fn render_week_calendar(&self, habit: &crate::domain::Habit) -> Span<'_> {
        let theme = self.theme;
        let today = Utc::now().date_naive();

        // Show last 7 days
        let mut calendar = String::new();
        for i in (0..7).rev() {
            let date = today - TimeDelta::days(i);
            let completed = habit.is_completed_on(date);
            let is_due = habit.frequency.is_due_on(date, habit.start_date);

            if completed {
                calendar.push('●');
            } else if is_due {
                calendar.push('○');
            } else {
                calendar.push('·');
            }
        }

        Span::styled(calendar, Style::default().fg(theme.colors.muted.to_color()))
    }

    fn render_analytics(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let block = Block::default()
            .title(" Analytics ")
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Get selected habit for detailed analytics
        let Some(habit) = self.model.selected_habit() else {
            let empty_msg = Paragraph::new("Select a habit to view analytics")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(inner, buf);
            return;
        };

        // Build analytics text
        let current_streak = habit.current_streak();
        let longest_streak = habit.longest_streak();
        let completion_rate = habit.overall_completion_rate();
        let weekday_rates = habit.completion_rate_by_weekday();

        let mut lines: Vec<Line<'_>> = Vec::new();

        // Streaks
        lines.push(Line::from(vec![
            Span::styled(
                "Current Streak: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                format!("{} days", current_streak),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Longest Streak: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                format!("{} days", longest_streak),
                Style::default().fg(theme.colors.success.to_color()),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Completion Rate: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                format!("{:.0}%", completion_rate * 100.0),
                Style::default().fg(self.completion_rate_color(completion_rate)),
            ),
        ]));

        // Blank line
        lines.push(Line::from(""));

        // Weekday breakdown header
        lines.push(Line::from(Span::styled(
            "By Weekday:",
            Style::default()
                .fg(theme.colors.muted.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Weekday completion rates as a simple bar chart
        let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (i, day) in weekdays.iter().enumerate() {
            let rate = weekday_rates[i];
            let bar_width = (rate * 10.0) as usize;
            let bar = "█".repeat(bar_width);
            let empty = "░".repeat(10 - bar_width);

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", day),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(bar, Style::default().fg(self.completion_rate_color(rate))),
                Span::styled(empty, Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(" {:.0}%", rate * 100.0),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
            ]));
        }

        // Blank line
        lines.push(Line::from(""));

        // Frequency info
        let freq_text = match &habit.frequency {
            crate::domain::HabitFrequency::Daily => "Daily".to_string(),
            crate::domain::HabitFrequency::Weekly { days } => {
                if days.is_empty() {
                    "Weekly".to_string()
                } else {
                    let day_names: Vec<&str> = days
                        .iter()
                        .map(|d| match d {
                            chrono::Weekday::Mon => "Mon",
                            chrono::Weekday::Tue => "Tue",
                            chrono::Weekday::Wed => "Wed",
                            chrono::Weekday::Thu => "Thu",
                            chrono::Weekday::Fri => "Fri",
                            chrono::Weekday::Sat => "Sat",
                            chrono::Weekday::Sun => "Sun",
                        })
                        .collect();
                    format!("Weekly: {}", day_names.join(", "))
                }
            }
            crate::domain::HabitFrequency::EveryNDays { n } => format!("Every {} days", n),
        };

        lines.push(Line::from(vec![
            Span::styled(
                "Frequency: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                freq_text,
                Style::default().fg(theme.colors.foreground.to_color()),
            ),
        ]));

        // Start date
        lines.push(Line::from(vec![
            Span::styled(
                "Started: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                habit.start_date.format("%Y-%m-%d").to_string(),
                Style::default().fg(theme.colors.foreground.to_color()),
            ),
        ]));

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }

    fn completion_rate_color(&self, rate: f64) -> Color {
        let theme = self.theme;
        if rate >= 0.8 {
            theme.colors.success.to_color()
        } else if rate >= 0.5 {
            theme.colors.accent.to_color()
        } else if rate >= 0.3 {
            theme.colors.warning.to_color()
        } else {
            theme.colors.danger.to_color()
        }
    }
}

/// Habit analytics popup widget (detailed view).
pub struct HabitAnalyticsPopup<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> HabitAnalyticsPopup<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for HabitAnalyticsPopup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Get selected habit
        let Some(habit) = self.model.selected_habit() else {
            return;
        };

        let block = Block::default()
            .title(format!(" {} - Analytics ", habit.name))
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Build detailed analytics
        let current_streak = habit.current_streak();
        let longest_streak = habit.longest_streak();
        let completion_rate = habit.overall_completion_rate();

        let mut lines: Vec<Line<'_>> = Vec::new();

        // Header
        lines.push(Line::from(Span::styled(
            &habit.name,
            Style::default()
                .fg(theme.colors.foreground.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        if let Some(ref desc) = habit.description {
            lines.push(Line::from(Span::styled(
                desc,
                Style::default()
                    .fg(theme.colors.muted.to_color())
                    .add_modifier(Modifier::ITALIC),
            )));
        }

        lines.push(Line::from(""));

        // Streak info with emoji
        lines.push(Line::from(vec![
            Span::raw("🔥 Current Streak: "),
            Span::styled(
                format!("{} days", current_streak),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("🏆 Best Streak: "),
            Span::styled(
                format!("{} days", longest_streak),
                Style::default().fg(theme.colors.success.to_color()),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("📊 Completion: "),
            Span::styled(
                format!("{:.1}%", completion_rate),
                Style::default().fg(theme.colors.foreground.to_color()),
            ),
        ]));

        // Trend analysis
        let trend_symbol = habit.trend_symbol();
        let trend_color = match habit.trend() {
            Some(crate::domain::HabitTrend::Improving) => theme.colors.success.to_color(),
            Some(crate::domain::HabitTrend::Declining) => theme.colors.danger.to_color(),
            Some(crate::domain::HabitTrend::Stable) => theme.colors.muted.to_color(),
            None => theme.colors.muted.to_color(),
        };
        let trend_text = match habit.trend() {
            Some(crate::domain::HabitTrend::Improving) => "Improving",
            Some(crate::domain::HabitTrend::Declining) => "Declining",
            Some(crate::domain::HabitTrend::Stable) => "Stable",
            None => "Not enough data",
        };
        lines.push(Line::from(vec![
            Span::raw("📈 Trend: "),
            Span::styled(
                format!("{} {}", trend_symbol, trend_text),
                Style::default().fg(trend_color),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press 'q' or Esc to close",
            Style::default().fg(theme.colors.muted.to_color()),
        )));

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_habits_view_renders_without_panic() {
        let model = Model::new();
        let theme = Theme::default();
        let view = HabitsView::new(&model, &theme);

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        view.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_habits_view_with_habits() {
        let mut model = Model::new();
        let habit = crate::domain::Habit::new("Exercise".to_string());
        model.habits.insert(habit.id, habit);
        model.refresh_visible_habits();

        let theme = Theme::default();
        let view = HabitsView::new(&model, &theme);

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        view.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }
}
