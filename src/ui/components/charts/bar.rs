//! Horizontal bar chart widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

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
                format!("{label:>max_label_len$}")
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
            let value_str = format!(" {value:>4}");
            let value_x = inner.x + inner.width - 6;
            buf.set_string(value_x, y, &value_str, Style::default().fg(Color::DarkGray));
        }
    }
}
