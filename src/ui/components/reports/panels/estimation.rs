//! Estimation panel rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use crate::ui::components::charts::{BarChart, ProgressGauge, StatBox};

use super::super::{format_duration, ReportsView};

impl ReportsView<'_> {
    pub(crate) fn render_estimation(&self, area: Rect, buf: &mut Buffer) {
        // Calculate estimation statistics
        let mut total_estimated: u32 = 0;
        let mut total_actual: u32 = 0;
        let mut over_count = 0;
        let mut under_count = 0;
        let mut on_target_count = 0;
        let mut accuracies: Vec<f64> = Vec::new();

        for task in self.model.tasks.values() {
            if let Some(est) = task.estimated_minutes {
                total_estimated = total_estimated.saturating_add(est);
                total_actual = total_actual.saturating_add(task.actual_minutes);

                if let Some(variance) = task.time_variance() {
                    match variance.cmp(&0) {
                        std::cmp::Ordering::Greater => over_count += 1,
                        std::cmp::Ordering::Less => under_count += 1,
                        std::cmp::Ordering::Equal => on_target_count += 1,
                    }
                }

                if let Some(accuracy) = task.estimation_accuracy() {
                    accuracies.push(accuracy);
                }
            }
        }

        let avg_accuracy = if accuracies.is_empty() {
            None
        } else {
            Some(accuracies.iter().sum::<f64>() / accuracies.len() as f64)
        };

        let tasks_with_estimates = over_count + under_count + on_target_count;

        // Split into stat boxes, gauge, and bar chart
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stat boxes
                Constraint::Length(3), // Accuracy gauge
                Constraint::Min(0),    // Breakdown chart
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

        // Accuracy
        let accuracy_str = avg_accuracy.map_or("N/A".to_string(), |a| format!("{a:.0}%"));
        let accuracy_stat = StatBox::new("Accuracy", &accuracy_str);
        accuracy_stat.render(stat_chunks[3], buf);

        // Render accuracy gauge
        if chunks[1].height > 0 {
            let accuracy_ratio = avg_accuracy.map_or(0.0, |a| {
                // Normalize: 100% accuracy = 1.0, 200% = 0.5, 50% = 0.5
                if a <= 100.0 {
                    a / 100.0
                } else {
                    100.0 / a
                }
            });
            let gauge = ProgressGauge::new("Estimation Accuracy", accuracy_ratio);
            gauge.render(chunks[1], buf);
        }

        // Render breakdown bar chart
        if chunks[2].height > 2 {
            let chart_block = Block::default()
                .title(" Estimation Breakdown ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let chart_inner = chart_block.inner(chunks[2]);
            chart_block.render(chunks[2], buf);

            if tasks_with_estimates > 0 {
                let data = vec![
                    ("Over".to_string(), over_count as u32),
                    ("Under".to_string(), under_count as u32),
                    ("On Target".to_string(), on_target_count as u32),
                ];

                let chart = BarChart::new("Breakdown", &data);
                chart.render(chart_inner, buf);
            } else {
                let msg = "No tasks with time estimates";
                buf.set_string(
                    chart_inner.x + 1,
                    chart_inner.y,
                    msg,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }
    }
}
