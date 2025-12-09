//! Tags panel rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::analytics::AnalyticsEngine;
use crate::domain::analytics::ReportConfig;
use crate::ui::components::charts::BarChart;

use super::super::ReportsView;

impl ReportsView<'_> {
    pub(crate) fn render_tags(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(30);
        let report = engine.generate_report(&config);

        // Split into header and chart
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        // Header
        let header = Paragraph::new(Line::from(vec![
            Span::styled("Tag Statistics ", Style::default().fg(Color::White)),
            Span::styled(
                format!("({} unique tags)", report.tag_stats.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        header.render(chunks[0], buf);

        // Tag bar chart
        let tag_data: Vec<(String, u32)> = report
            .tag_stats
            .iter()
            .take(10)
            .map(|t| (t.tag.clone(), t.count))
            .collect();

        if tag_data.is_empty() {
            let msg =
                Paragraph::new("No tags found. Add tags to your tasks to see statistics here.");
            msg.render(chunks[1], buf);
        } else {
            let chart =
                BarChart::new("Top Tags by Task Count", &tag_data).bar_color(Color::Magenta);
            chart.render(chunks[1], buf);
        }
    }
}
