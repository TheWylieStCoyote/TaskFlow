//! Overview panel rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Widget,
};

use crate::app::analytics::AnalyticsEngine;
use crate::domain::analytics::ReportConfig;
use crate::ui::components::charts::{BarChart, ProgressGauge, StatBox};

use super::super::ReportsView;

impl ReportsView<'_> {
    pub(crate) fn render_overview(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(30);
        let report = engine.generate_report(&config);

        // Split into stat boxes row and charts
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stat boxes
                Constraint::Min(0),    // Charts
            ])
            .split(area);

        // Render stat boxes
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[0]);

        // Total tasks
        let total_str = report.status_breakdown.total().to_string();
        let total_stat = StatBox::new("Total", &total_str);
        total_stat.render(stat_chunks[0], buf);

        // Done tasks
        let done_str = report.status_breakdown.done.to_string();
        let done_stat = StatBox::new("Done", &done_str).trend(report.velocity.trend);
        done_stat.render(stat_chunks[1], buf);

        // In Progress
        let progress_str = report.status_breakdown.in_progress.to_string();
        let progress_stat = StatBox::new("In Progress", &progress_str);
        progress_stat.render(stat_chunks[2], buf);

        // Completion rate
        let rate = (report.status_breakdown.completion_rate() * 100.0).round() as u32;
        let rate_str = format!("{rate}%");
        let rate_stat = StatBox::new("Complete", &rate_str);
        rate_stat.render(stat_chunks[3], buf);

        // Render progress gauge
        if chunks[1].height > 1 {
            let gauge_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, 1);
            let gauge = ProgressGauge::new(
                "Overall Progress",
                report.status_breakdown.completion_rate(),
            );
            gauge.render(gauge_area, buf);
        }

        // Render priority breakdown as bar chart
        if chunks[1].height > 3 {
            let chart_area = Rect::new(
                chunks[1].x,
                chunks[1].y + 2,
                chunks[1].width,
                chunks[1].height.saturating_sub(2),
            );

            let priority_data = vec![
                ("Urgent".to_string(), report.priority_breakdown.urgent),
                ("High".to_string(), report.priority_breakdown.high),
                ("Medium".to_string(), report.priority_breakdown.medium),
                ("Low".to_string(), report.priority_breakdown.low),
                ("None".to_string(), report.priority_breakdown.none),
            ];

            let chart = BarChart::new("Priority Distribution", &priority_data);
            chart.render(chart_area, buf);
        }
    }
}
