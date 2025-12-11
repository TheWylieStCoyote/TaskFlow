//! Estimation panel rendering for the Reports view.
//!
//! This module renders the estimation analytics panel, which helps users
//! understand their time estimation accuracy patterns. The panel displays:
//!
//! - **Stat boxes**: Total estimated time, actual time, variance, and multiplier
//! - **Insight text**: Summary of estimation tendencies (e.g., "You tend to estimate 15m over")
//! - **Accuracy gauge**: Visual representation of overall estimation accuracy
//! - **Breakdown chart**: Distribution of over/under/on-target estimates
//! - **Per-project table**: Accuracy breakdown by project with color-coded indicators
//!
//! # Color Coding
//!
//! - **Green**: On target (90-110% accuracy)
//! - **Yellow**: Moderate deviation (50-90% or 110-150%)
//! - **Red**: Significant deviation (<50% or >150%)

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::app::analytics::AnalyticsEngine;
use crate::domain::analytics::ReportConfig;
use crate::ui::components::charts::{BarChart, ProgressGauge, StatBox};

use super::super::{format_duration, ReportsView};

impl ReportsView<'_> {
    pub(crate) fn render_estimation(&self, area: Rect, buf: &mut Buffer) {
        // Generate estimation analytics from the engine
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(90);
        let analytics = engine.compute_estimation_analytics(config.start_date, config.end_date);

        // Calculate totals for display (across all tasks, not just in date range)
        let mut total_estimated: u32 = 0;
        let mut total_actual: u32 = 0;

        for task in self.model.tasks.values() {
            if let Some(est) = task.estimated_minutes {
                total_estimated = total_estimated.saturating_add(est);
                total_actual = total_actual.saturating_add(task.actual_minutes);
            }
        }

        // Split into sections: stat boxes, insight, gauge, breakdown, projects
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stat boxes
                Constraint::Length(2), // Insight text
                Constraint::Length(3), // Accuracy gauge
                Constraint::Min(4),    // Breakdown chart + projects
            ])
            .split(area);

        // Render stat boxes
        self.render_estimation_stats(chunks[0], buf, total_estimated, total_actual, &analytics);

        // Render insight text
        self.render_estimation_insight(chunks[1], buf, &analytics);

        // Render accuracy gauge
        self.render_estimation_gauge(chunks[2], buf, &analytics);

        // Render breakdown and project info
        self.render_estimation_breakdown(chunks[3], buf, &analytics);
    }

    #[allow(clippy::unused_self)]
    fn render_estimation_stats(
        &self,
        area: Rect,
        buf: &mut Buffer,
        total_estimated: u32,
        total_actual: u32,
        analytics: &crate::domain::analytics::EstimationAnalytics,
    ) {
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);

        // Estimated total
        let est_str = format_duration(total_estimated);
        let est_stat = StatBox::new("Estimated", &est_str);
        est_stat.render(stat_chunks[0], buf);

        // Actual total
        let actual_str = format_duration(total_actual);
        let actual_stat = StatBox::new("Actual", &actual_str);
        actual_stat.render(stat_chunks[1], buf);

        // Variance
        let variance = i64::from(total_actual) - i64::from(total_estimated);
        let variance_str = if variance > 0 {
            format!("+{}", format_duration(variance as u32))
        } else if variance < 0 {
            format!("-{}", format_duration((-variance) as u32))
        } else {
            "0m".to_string()
        };
        let variance_stat = StatBox::new("Variance", &variance_str);
        variance_stat.render(stat_chunks[2], buf);

        // Multiplier (from analytics)
        let multiplier_str = if analytics.suggested_multiplier > 0.0 {
            format!("{:.2}x", analytics.suggested_multiplier)
        } else {
            "N/A".to_string()
        };
        let multiplier_stat = StatBox::new("Multiplier", &multiplier_str);
        multiplier_stat.render(stat_chunks[3], buf);
    }

    #[allow(clippy::unused_self)]
    fn render_estimation_insight(
        &self,
        area: Rect,
        buf: &mut Buffer,
        analytics: &crate::domain::analytics::EstimationAnalytics,
    ) {
        if area.height == 0 {
            return;
        }

        let insight = analytics.accuracy_summary();
        let on_target_pct = analytics.on_target_percentage();

        // Build insight line with color (yellow if over-estimating, green otherwise)
        let insight_color = if analytics.avg_variance_minutes > 0 {
            Color::Yellow
        } else {
            Color::Green
        };
        let insight_style = Style::default().fg(insight_color);
        let extra_info = format!(" | {on_target_pct:.0}% on target");

        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(insight, insight_style.add_modifier(Modifier::ITALIC)),
            Span::styled(extra_info, Style::default().fg(Color::DarkGray)),
        ]);

        buf.set_line(area.x, area.y, &line, area.width);
    }

    #[allow(clippy::unused_self)]
    fn render_estimation_gauge(
        &self,
        area: Rect,
        buf: &mut Buffer,
        analytics: &crate::domain::analytics::EstimationAnalytics,
    ) {
        if area.height == 0 {
            return;
        }

        let accuracy_ratio = if analytics.suggested_multiplier > 0.0 {
            // Normalize: 1.0x = perfect (100%), higher/lower = worse
            if analytics.suggested_multiplier <= 1.0 {
                analytics.suggested_multiplier
            } else {
                1.0 / analytics.suggested_multiplier
            }
        } else {
            0.0
        };

        let gauge = ProgressGauge::new("Estimation Accuracy", accuracy_ratio);
        gauge.render(area, buf);
    }

    fn render_estimation_breakdown(
        &self,
        area: Rect,
        buf: &mut Buffer,
        analytics: &crate::domain::analytics::EstimationAnalytics,
    ) {
        if area.height < 3 {
            return;
        }

        // Split into breakdown chart and project list
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: Breakdown chart
        let chart_block = Block::default()
            .title(" Breakdown ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let chart_inner = chart_block.inner(chunks[0]);
        chart_block.render(chunks[0], buf);

        if analytics.tasks_with_estimates > 0 {
            let data = vec![
                ("Over".to_string(), analytics.over_count),
                ("Under".to_string(), analytics.under_count),
                ("On Target".to_string(), analytics.on_target_count),
            ];

            let chart = BarChart::new("Breakdown", &data);
            chart.render(chart_inner, buf);
        } else {
            buf.set_string(
                chart_inner.x + 1,
                chart_inner.y,
                "No tasks with estimates",
                Style::default().fg(Color::DarkGray),
            );
        }

        // Right: Per-project accuracy
        let project_block = Block::default()
            .title(" By Project ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let project_inner = project_block.inner(chunks[1]);
        project_block.render(chunks[1], buf);

        if analytics.by_project.is_empty() {
            buf.set_string(
                project_inner.x + 1,
                project_inner.y,
                "No project data",
                Style::default().fg(Color::DarkGray),
            );
        } else {
            // Show top projects by task count
            let max_rows = project_inner.height as usize;
            for (i, (project_id, accuracy, count)) in
                analytics.by_project.iter().take(max_rows).enumerate()
            {
                let name = project_id
                    .and_then(|pid| self.model.projects.get(&pid))
                    .map_or("(No Project)", |p| p.name.as_str());

                // Truncate name if needed
                let name_display: String = name.chars().take(15).collect();

                let accuracy_color = if *accuracy <= 110.0 && *accuracy >= 90.0 {
                    Color::Green
                } else if *accuracy > 150.0 || *accuracy < 50.0 {
                    Color::Red
                } else {
                    Color::Yellow
                };

                let line = Line::from(vec![
                    Span::raw(format!("{name_display:<15}")),
                    Span::styled(
                        format!(" {accuracy:>5.0}%"),
                        Style::default().fg(accuracy_color),
                    ),
                    Span::styled(format!(" ({count})"), Style::default().fg(Color::DarkGray)),
                ]);

                buf.set_line(
                    project_inner.x,
                    project_inner.y + i as u16,
                    &line,
                    project_inner.width,
                );
            }
        }
    }
}
