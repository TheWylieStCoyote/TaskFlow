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

        // Scale values to fit chart
        let max_value = data.total.max(1);
        let scale = |v: usize| -> usize {
            ((v as f64 / max_value as f64) * chart_height as f64).round() as usize
        };

        // Draw Y-axis
        for row in 0..chart_height {
            let value = max_value - (row * max_value / chart_height.max(1));
            let label = format!("{value:>4}│");
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
        let points = data.daily_completions.len().min(chart_width);
        if points > 0 {
            for i in 0..points {
                let ideal_remaining = data.total - (data.total * i / points.max(1));
                let ideal_y = chart_height - scale(ideal_remaining).min(chart_height);

                let x = area.x + 5 + (i * chart_width / points) as u16;
                let y = area.y + ideal_y as u16;

                if y < area.y + chart_height as u16 {
                    buf.set_string(x, y, "·", Style::default().fg(Color::DarkGray));
                }
            }
        }

        // Draw actual line
        for (i, &(_, remaining)) in data.daily_completions.iter().enumerate() {
            if i >= chart_width {
                break;
            }
            let actual_y = chart_height - scale(remaining).min(chart_height);

            let x = area.x + 5 + (i * chart_width / points.max(1)) as u16;
            let y = area.y + actual_y as u16;

            if y < area.y + chart_height as u16 {
                let color = if remaining > data.total * 3 / 4 {
                    Color::Red
                } else if remaining > data.total / 2 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                buf.set_string(x, y, "█", Style::default().fg(color));
            }
        }

        // Date labels
        if let (Some(first), Some(last)) = (
            data.daily_completions.first(),
            data.daily_completions.last(),
        ) {
            let start_label = format!("{}/{}", first.0.month(), first.0.day());
            let end_label = format!("{}/{}", last.0.month(), last.0.day());

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
