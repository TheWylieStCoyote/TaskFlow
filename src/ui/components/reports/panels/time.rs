//! Time panel rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Widget,
};

use crate::app::analytics::AnalyticsEngine;
use crate::domain::analytics::ReportConfig;
use crate::ui::components::charts::{BarChart, StatBox};

use super::super::ReportsView;

impl ReportsView<'_> {
    pub(crate) fn render_time(&self, area: Rect, buf: &mut Buffer) {
        // Use cached 30-day report if available, otherwise generate on-the-fly
        let fallback_report;
        let report = if let Some(ref cached) = self.model.report_cache.report_30d {
            cached
        } else {
            let engine = AnalyticsEngine::new(self.model);
            let config = ReportConfig::last_n_days(30);
            fallback_report = engine.generate_report(&config);
            &fallback_report
        };

        // Split vertically
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stats
                Constraint::Min(0),    // Charts
            ])
            .split(area);

        // Time stats
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[0]);

        let total_hours = report.time_analytics.total_hours();
        let hours_str = format!("{total_hours:.1}");
        let hours_stat = StatBox::new("Total Hours", &hours_str);
        hours_stat.render(stat_chunks[0], buf);

        if let Some(day) = report.time_analytics.most_productive_day() {
            let day_str = format!("{day:?}");
            let day_stat = StatBox::new("Best Day", &day_str);
            day_stat.render(stat_chunks[1], buf);
        }

        if let Some(hour) = report.time_analytics.peak_hour() {
            let hour_str = format!("{hour}:00");
            let hour_stat = StatBox::new("Peak Hour", &hour_str);
            hour_stat.render(stat_chunks[2], buf);
        }

        // Day of week chart
        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let day_data: Vec<(String, u32)> = report
            .time_analytics
            .by_day_of_week
            .iter()
            .enumerate()
            .map(|(i, &v)| (day_names[i].to_string(), v))
            .collect();

        if chunks[1].height > 3 {
            let chart = BarChart::new("Minutes by Day of Week", &day_data).bar_color(Color::Yellow);
            chart.render(chunks[1], buf);
        }
    }
}
