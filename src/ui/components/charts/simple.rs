//! Simple chart widgets: Sparkline, ProgressGauge, StatBox.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use super::SPARKLINE_CHARS;

// ============================================================================
// Sparkline
// ============================================================================

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

// ============================================================================
// ProgressGauge
// ============================================================================

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
        let filled_width = ((progress * f64::from(bar_width)).round() as u16).min(bar_width);

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
        let percent_str = format!(" {percentage:>3}%");
        buf.set_string(
            bar_x + bar_width + 2,
            area.y,
            &percent_str,
            Style::default().fg(Color::Cyan),
        );
    }
}

// ============================================================================
// StatBox
// ============================================================================

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
