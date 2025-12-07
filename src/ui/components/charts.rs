//! ASCII chart widgets for terminal UI.
//!
//! This module provides chart widgets for displaying analytics data
//! in the terminal using ASCII/Unicode characters.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

/// Characters for sparkline chart.
const SPARKLINE_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A horizontal bar chart widget.
pub struct BarChart<'a> {
    /// Title for the chart
    title: &'a str,
    /// Data points as (label, value) pairs
    data: &'a [(String, u32)],
    /// Maximum value (if None, auto-calculated)
    max_value: Option<u32>,
    /// Bar color
    bar_color: Color,
    /// Label color
    label_color: Color,
}

impl<'a> BarChart<'a> {
    /// Create a new bar chart.
    #[must_use]
    pub const fn new(title: &'a str, data: &'a [(String, u32)]) -> Self {
        Self {
            title,
            data,
            max_value: None,
            bar_color: Color::Cyan,
            label_color: Color::White,
        }
    }

    /// Set the maximum value for scaling.
    #[must_use]
    pub const fn max_value(mut self, max: u32) -> Self {
        self.max_value = Some(max);
        self
    }

    /// Set the bar color.
    #[must_use]
    pub const fn bar_color(mut self, color: Color) -> Self {
        self.bar_color = color;
        self
    }

    /// Set the label color.
    #[must_use]
    pub const fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }
}

impl Widget for BarChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 3 || self.data.is_empty() {
            return;
        }

        // Render block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        block.render(area, buf);

        // Calculate max value
        let max_val = self
            .max_value
            .unwrap_or_else(|| self.data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1));

        // Find the longest label for alignment
        let max_label_len = self
            .data
            .iter()
            .map(|(label, _)| label.len())
            .max()
            .unwrap_or(0)
            .min(15); // Cap label length

        // Available width for the bar
        let bar_area_width = inner.width.saturating_sub(max_label_len as u16 + 2 + 6); // label + space + value display

        // Render each bar
        for (i, (label, value)) in self.data.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }

            let y = inner.y + i as u16;

            // Render label (right-aligned)
            let label_display: String = if label.len() > max_label_len {
                format!("{}...", &label[..max_label_len - 3])
            } else {
                format!("{:>width$}", label, width = max_label_len)
            };

            buf.set_string(
                inner.x,
                y,
                &label_display,
                Style::default().fg(self.label_color),
            );

            // Calculate bar width
            let bar_width = if max_val > 0 {
                ((u64::from(*value) * u64::from(bar_area_width)) / u64::from(max_val)) as u16
            } else {
                0
            };

            // Render bar
            let bar_x = inner.x + max_label_len as u16 + 1;
            for x in 0..bar_width {
                if bar_x + x < inner.x + inner.width - 6 {
                    buf.set_string(bar_x + x, y, "█", Style::default().fg(self.bar_color));
                }
            }

            // Render value
            let value_str = format!(" {:>4}", value);
            let value_x = inner.x + inner.width - 6;
            buf.set_string(value_x, y, &value_str, Style::default().fg(Color::DarkGray));
        }
    }
}

/// A sparkline chart (mini line chart using Unicode block characters).
pub struct Sparkline<'a> {
    /// Title for the chart
    title: &'a str,
    /// Data values
    data: &'a [f64],
    /// Line color
    line_color: Color,
}

impl<'a> Sparkline<'a> {
    /// Create a new sparkline.
    #[must_use]
    pub const fn new(title: &'a str, data: &'a [f64]) -> Self {
        Self {
            title,
            data,
            line_color: Color::Green,
        }
    }

    /// Set the line color.
    #[must_use]
    pub const fn line_color(mut self, color: Color) -> Self {
        self.line_color = color;
        self
    }
}

impl Widget for Sparkline<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 5 || area.height < 2 || self.data.is_empty() {
            return;
        }

        // Render title
        buf.set_string(
            area.x,
            area.y,
            self.title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        // Find min/max for scaling (filter out NaN values)
        let min_val = self
            .data
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        let max_val = self
            .data
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0);
        let range = (max_val - min_val).max(0.001); // Avoid division by zero

        // Render sparkline
        let sparkline_y = area.y + 1;
        let available_width = area.width.saturating_sub(2) as usize;
        let data_len = self.data.len();

        // Sample data if we have more points than width
        let step = if data_len > available_width {
            data_len / available_width
        } else {
            1
        };

        for (i, chunk) in self.data.chunks(step).enumerate() {
            if i as u16 >= area.width - 2 {
                break;
            }

            // Average the chunk
            let avg: f64 = chunk.iter().sum::<f64>() / chunk.len() as f64;
            let normalized = ((avg - min_val) / range).clamp(0.0, 1.0);
            let char_idx = (normalized * 7.0).round() as usize;
            let c = SPARKLINE_CHARS[char_idx.min(7)];

            buf.set_string(
                area.x + i as u16,
                sparkline_y,
                c.to_string(),
                Style::default().fg(self.line_color),
            );
        }
    }
}

/// A simple burndown chart widget.
pub struct BurndownChart<'a> {
    /// Title for the chart
    title: &'a str,
    /// Scope values over time
    scope: &'a [f64],
    /// Completed values over time
    completed: &'a [f64],
}

impl<'a> BurndownChart<'a> {
    /// Create a new burndown chart.
    #[must_use]
    pub const fn new(title: &'a str, scope: &'a [f64], completed: &'a [f64]) -> Self {
        Self {
            title,
            scope,
            completed,
        }
    }
}

impl Widget for BurndownChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        // Render block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.scope.is_empty() || self.completed.is_empty() {
            buf.set_string(
                inner.x + 1,
                inner.y,
                "No data",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        // Find max value for scaling (handle NaN values safely)
        let max_val = self
            .scope
            .iter()
            .chain(self.completed.iter())
            .copied()
            .filter(|v| !v.is_nan())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0)
            .max(1.0);

        let chart_height = inner.height.saturating_sub(1);
        let chart_width = inner.width.saturating_sub(2);

        // Sample data to fit width
        let data_len = self.scope.len().max(self.completed.len());
        let step = if data_len > chart_width as usize {
            data_len / chart_width as usize
        } else {
            1
        };

        // Render remaining line (scope - completed)
        for (i, chunk_idx) in (0..data_len).step_by(step).enumerate() {
            if i as u16 >= chart_width {
                break;
            }

            let scope_val = self.scope.get(chunk_idx).copied().unwrap_or(0.0);
            let completed_val = self.completed.get(chunk_idx).copied().unwrap_or(0.0);
            let remaining = (scope_val - completed_val).max(0.0);

            let height = ((remaining / max_val) * chart_height as f64).round() as u16;

            // Draw vertical bar for remaining work
            for h in 0..height {
                let y = inner.y + chart_height - h - 1;
                if y >= inner.y && y < inner.y + inner.height {
                    let c = if h == height.saturating_sub(1) {
                        '▄'
                    } else {
                        '│'
                    };
                    buf.set_string(
                        inner.x + 1 + i as u16,
                        y,
                        c.to_string(),
                        Style::default().fg(Color::Yellow),
                    );
                }
            }
        }

        // Render legend on bottom
        let legend = Line::from(vec![
            Span::styled("█ ", Style::default().fg(Color::Yellow)),
            Span::styled("Remaining  ", Style::default().fg(Color::DarkGray)),
        ]);

        buf.set_line(inner.x, inner.y + inner.height - 1, &legend, inner.width);
    }
}

/// A simple gauge/progress bar widget.
pub struct ProgressGauge<'a> {
    /// Label for the gauge
    label: &'a str,
    /// Progress value (0.0 to 1.0)
    progress: f64,
    /// Color for completed portion
    filled_color: Color,
    /// Color for remaining portion
    empty_color: Color,
}

impl<'a> ProgressGauge<'a> {
    /// Create a new progress gauge.
    #[must_use]
    pub const fn new(label: &'a str, progress: f64) -> Self {
        Self {
            label,
            progress,
            filled_color: Color::Green,
            empty_color: Color::DarkGray,
        }
    }

    /// Set the filled color.
    #[must_use]
    pub const fn filled_color(mut self, color: Color) -> Self {
        self.filled_color = color;
        self
    }

    /// Set the empty color.
    #[must_use]
    pub const fn empty_color(mut self, color: Color) -> Self {
        self.empty_color = color;
        self
    }
}

impl Widget for ProgressGauge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 1 {
            return;
        }

        let progress = self.progress.clamp(0.0, 1.0);
        let percentage = (progress * 100.0).round() as u8;

        // Calculate widths
        let label_width = (self.label.len() + 1).min(area.width as usize / 3) as u16;
        let percent_width = 5u16; // " XXX%"
        let bar_width = area.width.saturating_sub(label_width + percent_width + 2);
        let filled_width = ((progress * bar_width as f64).round() as u16).min(bar_width);

        // Render label
        let label: String = if self.label.len() > label_width as usize - 1 {
            format!("{}.", &self.label[..label_width as usize - 2])
        } else {
            self.label.to_string()
        };
        buf.set_string(area.x, area.y, &label, Style::default().fg(Color::White));

        // Render bar
        let bar_x = area.x + label_width;
        buf.set_string(bar_x, area.y, "[", Style::default().fg(Color::DarkGray));

        for i in 0..bar_width {
            let c = if i < filled_width { '█' } else { '░' };
            let color = if i < filled_width {
                self.filled_color
            } else {
                self.empty_color
            };
            buf.set_string(
                bar_x + 1 + i,
                area.y,
                c.to_string(),
                Style::default().fg(color),
            );
        }

        buf.set_string(
            bar_x + bar_width + 1,
            area.y,
            "]",
            Style::default().fg(Color::DarkGray),
        );

        // Render percentage
        let percent_str = format!(" {:>3}%", percentage);
        buf.set_string(
            bar_x + bar_width + 2,
            area.y,
            &percent_str,
            Style::default().fg(Color::Cyan),
        );
    }
}

/// A stat box showing a number with a label.
pub struct StatBox<'a> {
    /// Label for the stat
    label: &'a str,
    /// Value to display
    value: &'a str,
    /// Optional trend indicator (+/-)
    trend: Option<f64>,
}

impl<'a> StatBox<'a> {
    /// Create a new stat box.
    #[must_use]
    pub const fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            trend: None,
        }
    }

    /// Set the trend indicator.
    #[must_use]
    pub const fn trend(mut self, trend: f64) -> Self {
        self.trend = Some(trend);
        self
    }
}

impl Widget for StatBox<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 3 {
            return;
        }

        // Render border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        block.render(area, buf);

        // Render value (centered, large)
        let value_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let value_x = inner.x + (inner.width.saturating_sub(self.value.len() as u16)) / 2;
        buf.set_string(value_x, inner.y, self.value, value_style);

        // Render trend if available
        if let Some(trend) = self.trend {
            let (trend_str, trend_color) = if trend > 0.0 {
                ("↑", Color::Green)
            } else if trend < 0.0 {
                ("↓", Color::Red)
            } else {
                ("→", Color::DarkGray)
            };
            if inner.width > self.value.len() as u16 + 2 {
                buf.set_string(
                    value_x + self.value.len() as u16 + 1,
                    inner.y,
                    trend_str,
                    Style::default().fg(trend_color),
                );
            }
        }

        // Render label (centered, below value)
        if inner.height > 1 {
            let label_style = Style::default().fg(Color::DarkGray);
            let label = if self.label.len() > inner.width as usize {
                &self.label[..inner.width as usize]
            } else {
                self.label
            };
            let label_x = inner.x + (inner.width.saturating_sub(label.len() as u16)) / 2;
            buf.set_string(label_x, inner.y + 1, label, label_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_buffer(width: u16, height: u16) -> (Rect, Buffer) {
        let area = Rect::new(0, 0, width, height);
        let buffer = Buffer::empty(area);
        (area, buffer)
    }

    #[test]
    fn test_bar_chart_creation() {
        let data = vec![("Item 1".to_string(), 10), ("Item 2".to_string(), 20)];
        let _chart = BarChart::new("Test", &data).bar_color(Color::Blue);
    }

    #[test]
    fn test_bar_chart_render_empty() {
        let data: Vec<(String, u32)> = vec![];
        let chart = BarChart::new("Test", &data);
        let (area, mut buf) = test_buffer(50, 10);
        chart.render(area, &mut buf);
        // Should not panic
    }

    #[test]
    fn test_bar_chart_render_with_data() {
        let data = vec![
            ("Monday".to_string(), 5),
            ("Tuesday".to_string(), 10),
            ("Wednesday".to_string(), 8),
        ];
        let chart = BarChart::new("Weekly Tasks", &data);
        let (area, mut buf) = test_buffer(60, 10);
        chart.render(area, &mut buf);
        // Should render without panic
    }

    #[test]
    fn test_sparkline_creation() {
        let data = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let _spark = Sparkline::new("Test", &data).line_color(Color::Blue);
    }

    #[test]
    fn test_sparkline_render_empty() {
        let data: Vec<f64> = vec![];
        let spark = Sparkline::new("Empty", &data);
        let (area, mut buf) = test_buffer(30, 3);
        spark.render(area, &mut buf);
    }

    #[test]
    fn test_sparkline_render_with_data() {
        let data = vec![1.0, 3.0, 2.0, 4.0, 3.0, 5.0, 4.0, 6.0];
        let spark = Sparkline::new("Velocity", &data);
        let (area, mut buf) = test_buffer(30, 3);
        spark.render(area, &mut buf);
    }

    #[test]
    fn test_burndown_chart_render() {
        let scope = vec![10.0, 10.0, 11.0, 11.0, 11.0];
        let completed = vec![0.0, 2.0, 4.0, 6.0, 8.0];
        let chart = BurndownChart::new("Sprint Burndown", &scope, &completed);
        let (area, mut buf) = test_buffer(40, 15);
        chart.render(area, &mut buf);
    }

    #[test]
    fn test_progress_gauge_creation() {
        let _gauge = ProgressGauge::new("Progress", 0.75)
            .filled_color(Color::Blue)
            .empty_color(Color::DarkGray);
    }

    #[test]
    fn test_progress_gauge_render() {
        let gauge = ProgressGauge::new("Done", 0.65);
        let (area, mut buf) = test_buffer(40, 1);
        gauge.render(area, &mut buf);
    }

    #[test]
    fn test_progress_gauge_clamping() {
        // Test that values outside 0-1 are clamped
        let gauge_over = ProgressGauge::new("Over", 1.5);
        let (area, mut buf) = test_buffer(40, 1);
        gauge_over.render(area, &mut buf);

        let gauge_under = ProgressGauge::new("Under", -0.5);
        let (area, mut buf) = test_buffer(40, 1);
        gauge_under.render(area, &mut buf);
    }

    #[test]
    fn test_stat_box_creation() {
        let _stat = StatBox::new("Tasks", "42").trend(5.0);
    }

    #[test]
    fn test_stat_box_render() {
        let stat = StatBox::new("Completed", "156").trend(10.0);
        let (area, mut buf) = test_buffer(15, 4);
        stat.render(area, &mut buf);
    }

    #[test]
    fn test_stat_box_negative_trend() {
        let stat = StatBox::new("Overdue", "3").trend(-2.0);
        let (area, mut buf) = test_buffer(15, 4);
        stat.render(area, &mut buf);
    }
}
