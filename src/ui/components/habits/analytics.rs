//! Habit analytics popup widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;

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
                format!("{current_streak} days"),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("🏆 Best Streak: "),
            Span::styled(
                format!("{longest_streak} days"),
                Style::default().fg(theme.colors.success.to_color()),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("📊 Completion: "),
            Span::styled(
                format!("{completion_rate:.1}%"),
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
                format!("{trend_symbol} {trend_text}"),
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
