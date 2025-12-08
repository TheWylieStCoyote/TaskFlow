//! Velocity panel rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Widget,
};

use crate::app::analytics::AnalyticsEngine;
use crate::domain::analytics::ReportConfig;
use crate::ui::components::charts::{BarChart, Sparkline, StatBox};

use super::super::ReportsView;

impl ReportsView<'_> {
    pub(crate) fn render_velocity(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(60);
        let report = engine.generate_report(&config);

        // Split vertically
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Sparkline
                Constraint::Length(4), // Stats
                Constraint::Min(0),    // Weekly bar chart
            ])
            .split(area);

        // Velocity sparkline
        let velocity_values: Vec<f64> = report
            .velocity
            .weekly_velocity
            .iter()
            .map(|(_, v)| *v as f64)
            .collect();

        if !velocity_values.is_empty() {
            let spark = Sparkline::new("Weekly Velocity Trend", &velocity_values).line_color(
                if report.velocity.is_improving() {
                    Color::Green
                } else {
                    Color::Red
                },
            );
            spark.render(chunks[0], buf);
        }

        // Velocity stats
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[1]);

        let avg_str = format!("{:.1}", report.velocity.avg_weekly);
        let avg_stat = StatBox::new("Avg/Week", &avg_str).trend(report.velocity.trend);
        avg_stat.render(stat_chunks[0], buf);

        if let Some((_, best)) = report.velocity.best_week() {
            let best_str = best.to_string();
            let best_stat = StatBox::new("Best Week", &best_str);
            best_stat.render(stat_chunks[1], buf);
        }

        let trend_str = if report.velocity.trend > 0.0 {
            "Improving"
        } else if report.velocity.trend < 0.0 {
            "Declining"
        } else {
            "Stable"
        };
        let trend_stat = StatBox::new("Trend", trend_str);
        trend_stat.render(stat_chunks[2], buf);

        // Weekly bar chart
        let weekly_data: Vec<(String, u32)> = report
            .velocity
            .weekly_velocity
            .iter()
            .rev()
            .take(8)
            .rev()
            .map(|(date, v)| (date.format("W%U").to_string(), *v))
            .collect();

        if !weekly_data.is_empty() && chunks[2].height > 3 {
            let chart = BarChart::new("Weekly Completions", &weekly_data).bar_color(Color::Blue);
            chart.render(chunks[2], buf);
        }
    }
}
