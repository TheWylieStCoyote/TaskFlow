//! Burndown statistics rendering.

use chrono::{Datelike, Duration, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::BurndownMode;

use super::{Burndown, BurndownData};

impl Burndown<'_> {
    /// Render progress stats
    pub(crate) fn render_stats(&self, area: Rect, buf: &mut Buffer, data: &BurndownData) {
        let progress_pct = if data.total > 0.0 {
            ((data.completed / data.total) * 100.0) as usize
        } else {
            0
        };

        // Progress bar
        let bar_width = 20;
        let filled = ((bar_width as f64 * data.completed) / data.total.max(1.0)) as usize;
        let empty = bar_width - filled;

        let progress_bar = format!(
            "[{}{}] {}%",
            "█".repeat(filled),
            "░".repeat(empty),
            progress_pct
        );

        let today = Local::now().date_naive();
        let days_elapsed = data.window_days;

        let velocity = if days_elapsed > 0 {
            data.completed / days_elapsed as f64
        } else {
            0.0
        };

        let projected_completion = if velocity > 0.0 {
            let days_needed = (data.remaining / velocity).ceil() as i64;
            Some(today + Duration::days(days_needed))
        } else {
            None
        };

        // Format values based on mode
        let (total_label, total_str, completed_str, remaining_str, velocity_str) = match data.mode {
            BurndownMode::TaskCount => (
                "Total tasks: ",
                format!("{:.0}", data.total),
                format!("{:.0}", data.completed),
                format!("{:.0}", data.remaining),
                format!("{velocity:.1} tasks/day"),
            ),
            BurndownMode::TimeHours => (
                "Total hours: ",
                format!("{:.1}h", data.total),
                format!("{:.1}h", data.completed),
                format!("{:.1}h", data.remaining),
                format!("{velocity:.1} hrs/day"),
            ),
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    "Progress: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    progress_bar,
                    Style::default().fg(if progress_pct >= 75 {
                        Color::Green
                    } else if progress_pct >= 50 {
                        Color::Yellow
                    } else {
                        Color::Red
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    total_label,
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    total_str,
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Completed: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(completed_str, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Remaining: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    remaining_str,
                    Style::default().fg(if data.remaining > 0.0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    }),
                ),
            ]),
        ];

        // Add scope creep info if enabled and there's data
        if self.model.burndown_state.show_scope_creep && data.scope_added > 0.0 {
            let scope_str = match data.mode {
                BurndownMode::TaskCount => format!("+{:.0} tasks", data.scope_added),
                BurndownMode::TimeHours => format!("+{:.1}h", data.scope_added),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    "Scope added: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(scope_str, Style::default().fg(Color::Magenta)),
            ]));
        }

        lines.extend([
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Velocity: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    velocity_str,
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            if let Some(proj_date) = projected_completion {
                Line::from(vec![
                    Span::styled(
                        "Projected: ",
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        format!("{}/{}", proj_date.month(), proj_date.day()),
                        Style::default().fg(if proj_date <= data.end_date {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        "Projected: ",
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        "N/A",
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                ])
            },
            Line::from(vec![
                Span::styled(
                    "Target: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{}/{}", data.end_date.month(), data.end_date.day()),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
        ]);

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Statistics ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.colors.border.to_color())),
        );
        paragraph.render(area, buf);
    }

    /// Render project selector
    pub(crate) fn render_projects(&self, area: Rect, buf: &mut Buffer) {
        let mut lines = vec![Line::from(vec![Span::styled(
            "► All Tasks",
            Style::default()
                .fg(self.theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )])];

        for project in self.model.projects.values() {
            let task_count = self
                .model
                .tasks
                .values()
                .filter(|t| t.project_id == Some(project.id))
                .count();
            let completed = self
                .model
                .tasks
                .values()
                .filter(|t| t.project_id == Some(project.id) && t.status.is_complete())
                .count();

            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    &project.name,
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
                Span::styled(
                    format!(" ({completed}/{task_count})"),
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]));
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Projects ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.colors.border.to_color())),
        );
        paragraph.render(area, buf);
    }
}
