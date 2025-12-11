//! Burndown chart widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

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

            let height = ((remaining / max_val) * f64::from(chart_height)).round() as u16;

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
