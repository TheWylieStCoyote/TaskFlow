//! Focus panel rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::ui::components::charts::{BarChart, StatBox};

use super::super::ReportsView;

impl ReportsView<'_> {
    pub(crate) fn render_focus(&self, area: Rect, buf: &mut Buffer) {
        let stats = &self.model.pomodoro_stats;

        // Split vertically
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stats row
                Constraint::Length(5), // Streak info
                Constraint::Min(0),    // Chart
            ])
            .split(area);

        // Stats row
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[0]);

        // Today's cycles
        let today_str = stats.cycles_today().to_string();
        let today_stat = StatBox::new("Today 🍅", &today_str);
        today_stat.render(stat_chunks[0], buf);

        // Total cycles
        let total_str = stats.total_cycles.to_string();
        let total_stat = StatBox::new("Total Cycles", &total_str);
        total_stat.render(stat_chunks[1], buf);

        // Total hours
        let total_hours = stats.total_work_mins / 60;
        let hours_str = format!("{total_hours}h");
        let hours_stat = StatBox::new("Focus Time", &hours_str);
        hours_stat.render(stat_chunks[2], buf);

        // Current streak
        let streak_str = format!("{} days", stats.current_streak());
        let streak_stat = StatBox::new("Streak", &streak_str);
        streak_stat.render(stat_chunks[3], buf);

        // Streak info section
        let streak_info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Current Streak: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} days", stats.current_streak()),
                    Style::default()
                        .fg(if stats.current_streak() > 0 {
                            Color::Green
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Longest Streak: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} days", stats.longest_streak),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Avg Minutes/Cycle: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if stats.total_cycles > 0 {
                        format!(
                            "{:.0}",
                            stats.total_work_mins as f32 / stats.total_cycles as f32
                        )
                    } else {
                        "N/A".to_string()
                    },
                    Style::default().fg(Color::White),
                ),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Streak Stats ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        streak_info.render(chunks[1], buf);

        // Weekly activity chart (last 7 days)
        let today = chrono::Utc::now().date_naive();
        let weekly_data: Vec<(String, u32)> = (0..7)
            .rev()
            .map(|i| {
                let date = today - chrono::Duration::days(i);
                let cycles = stats.cycles_by_date.get(&date).copied().unwrap_or(0);
                let day_name = date.format("%a").to_string();
                (day_name, cycles)
            })
            .collect();

        if chunks[2].height > 3 {
            let chart =
                BarChart::new("Last 7 Days (Pomodoro Cycles)", &weekly_data).bar_color(Color::Red);
            chart.render(chunks[2], buf);
        }
    }
}
