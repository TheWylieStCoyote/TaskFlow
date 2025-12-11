//! Burndown chart rendering.

use chrono::Datelike;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

use super::{Burndown, BurndownData};

impl Burndown<'_> {
    /// Render the ASCII burndown chart
    pub(crate) fn render_chart(&self, area: Rect, buf: &mut Buffer, data: &BurndownData) {
        if area.height < 5 || area.width < 20 {
            return;
        }

        let chart_height = area.height.saturating_sub(2) as usize;
        let chart_width = area.width.saturating_sub(8) as usize;
        let show_scope_creep = self.model.burndown_state.show_scope_creep;

        // Scale values to fit chart (use f64 for all calculations)
        let max_value = data.total.max(1.0);
        let scale = |v: f64| -> usize { ((v / max_value) * chart_height as f64).round() as usize };

        // Draw Y-axis with appropriate labels
        for row in 0..chart_height {
            let value = max_value - (row as f64 * max_value / chart_height.max(1) as f64);
            let label = if max_value >= 100.0 {
                format!("{value:>4.0}│")
            } else {
                format!("{value:>4.1}│")
            };
            buf.set_string(
                area.x,
                area.y + row as u16,
                &label,
                Style::default().fg(self.theme.colors.muted.to_color()),
            );
        }

        // X-axis
        buf.set_string(
            area.x,
            area.y + chart_height as u16,
            "    └",
            Style::default().fg(self.theme.colors.muted.to_color()),
        );
        for x in 0..chart_width {
            buf.set_string(
                area.x + 5 + x as u16,
                area.y + chart_height as u16,
                "─",
                Style::default().fg(self.theme.colors.muted.to_color()),
            );
        }

        // Draw ideal line (from total to 0)
        let points = data.daily_points.len().min(chart_width);
        if points > 0 {
            for i in 0..points {
                let ideal_remaining = data.total - (data.total * i as f64 / points.max(1) as f64);
                let ideal_y = chart_height - scale(ideal_remaining).min(chart_height);

                let x = area.x + 5 + (i * chart_width / points) as u16;
                let y = area.y + ideal_y as u16;

                if y < area.y + chart_height as u16 {
                    buf.set_string(x, y, "·", Style::default().fg(Color::DarkGray));
                }
            }
        }

        // Draw scope creep indicators (tasks added) if enabled
        if show_scope_creep {
            for (i, point) in data.daily_points.iter().enumerate() {
                if i >= chart_width || point.added <= 0.0 {
                    continue;
                }
                // Draw a small marker at the top for days with scope additions
                let x = area.x + 5 + (i * chart_width / points.max(1)) as u16;
                buf.set_string(x, area.y, "+", Style::default().fg(Color::Magenta));
            }
        }

        // Draw actual line
        for (i, point) in data.daily_points.iter().enumerate() {
            if i >= chart_width {
                break;
            }
            let actual_y = chart_height - scale(point.remaining).min(chart_height);

            let x = area.x + 5 + (i * chart_width / points.max(1)) as u16;
            let y = area.y + actual_y as u16;

            if y < area.y + chart_height as u16 {
                let ratio = point.remaining / data.total.max(1.0);
                let color = if ratio > 0.75 {
                    Color::Red
                } else if ratio > 0.5 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                buf.set_string(x, y, "█", Style::default().fg(color));
            }
        }

        // Date labels
        if let (Some(first), Some(last)) = (data.daily_points.first(), data.daily_points.last()) {
            let start_label = format!("{}/{}", first.date.month(), first.date.day());
            let end_label = format!("{}/{}", last.date.month(), last.date.day());

            buf.set_string(
                area.x + 5,
                area.y + chart_height as u16 + 1,
                &start_label,
                Style::default().fg(self.theme.colors.muted.to_color()),
            );
            buf.set_string(
                area.x + area.width - end_label.len() as u16 - 1,
                area.y + chart_height as u16 + 1,
                &end_label,
                Style::default().fg(self.theme.colors.muted.to_color()),
            );
        }
    }
}
