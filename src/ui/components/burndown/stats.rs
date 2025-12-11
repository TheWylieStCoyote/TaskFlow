//! Burndown statistics rendering.

use chrono::{Datelike, Duration, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::{Burndown, BurndownData};

impl Burndown<'_> {
    /// Render progress stats
    pub(crate) fn render_stats(&self, area: Rect, buf: &mut Buffer, data: &BurndownData) {
        let progress_pct = if data.total > 0 {
            data.completed * 100 / data.total
        } else {
            0
        };

        // Progress bar
        let bar_width = 20;
        let filled = (bar_width * data.completed) / data.total.max(1);
        let empty = bar_width - filled;

        let progress_bar = format!(
            "[{}{}] {}%",
            "█".repeat(filled),
            "░".repeat(empty),
            progress_pct
        );

        let today = Local::now().date_naive();
        let days_elapsed = (today - data.start_date).num_days();
        let _days_remaining = (data.end_date - today).num_days();

        let velocity = if days_elapsed > 0 {
            data.completed as f64 / days_elapsed as f64
        } else {
            0.0
        };

        let projected_completion = if velocity > 0.0 {
            let days_needed = (data.remaining as f64 / velocity).ceil() as i64;
            Some(today + Duration::days(days_needed))
        } else {
            None
        };

        let lines = vec![
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
                    "Total tasks: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{}", data.total),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Completed: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{}", data.completed),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Remaining: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{}", data.remaining),
                    Style::default().fg(if data.remaining > 0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Velocity: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{velocity:.1} tasks/day"),
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
        ];

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
