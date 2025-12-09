//! Dashboard panel rendering
//!
//! This module contains the rendering logic for individual dashboard panels:
//! - Completion panel
//! - Time tracking panel
//! - Projects panel
//! - Status distribution panel
//! - Estimation panel
//! - Focus sessions panel
//! - Activity panel

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::config::Theme;
use crate::domain::{Priority, TaskStatus};

use super::stats::{format_duration, DashboardStats};
use super::Dashboard;

impl Dashboard<'_> {
    pub(crate) fn render_completion_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Completion ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let stats = DashboardStats::new(self.model);
        let rate = stats.completion_rate();
        let overdue = stats.overdue_count();

        // Overall rate
        let rate_color = if rate >= 75.0 {
            Color::Green
        } else if rate >= 50.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        let lines = [
            Line::from(vec![
                Span::styled(
                    "Overall: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{rate:.0}%"),
                    Style::default().fg(rate_color).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Overdue: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{overdue}"),
                    Style::default().fg(if overdue > 0 {
                        theme.colors.danger.to_color()
                    } else {
                        Color::Green
                    }),
                ),
            ]),
            self.priority_completion_line(Priority::Urgent, "Urgent", theme),
            self.priority_completion_line(Priority::High, "High", theme),
        ];

        for (i, line) in lines.iter().enumerate() {
            if (i as u16) < inner.height {
                buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
            }
        }
    }

    pub(crate) fn priority_completion_line(
        &self,
        priority: Priority,
        label: &str,
        theme: &Theme,
    ) -> Line<'static> {
        let stats = DashboardStats::new(self.model);
        let (completed, total) = stats.completion_by_priority(priority);
        let rate = if total > 0 {
            (completed as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Line::from(vec![
            Span::styled(
                format!("{label}: "),
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                format!("{completed}/{total} ({rate:.0}%)"),
                Style::default(),
            ),
        ])
    }

    pub(crate) fn render_time_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Time Tracking ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let stats = DashboardStats::new(self.model);
        let total_time = stats.total_time_tracked();
        let completed_tasks = self
            .model
            .tasks
            .values()
            .filter(|t| t.status.is_complete())
            .count();
        let avg_time = if completed_tasks > 0 {
            total_time / completed_tasks as u32
        } else {
            0
        };

        let active = self.model.active_time_entry.is_some();

        let lines = [
            Line::from(vec![
                Span::styled(
                    "Total: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format_duration(total_time),
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Avg/task: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(format_duration(avg_time), Style::default()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Tracking: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    if active { "● Active" } else { "○ Idle" },
                    Style::default().fg(if active {
                        Color::Green
                    } else {
                        theme.colors.muted.to_color()
                    }),
                ),
            ]),
        ];

        for (i, line) in lines.iter().enumerate() {
            if (i as u16) < inner.height {
                buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
            }
        }
    }

    pub(crate) fn render_projects_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Projects ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.model.projects.is_empty() {
            let line = Line::from(Span::styled(
                "No projects",
                Style::default().fg(theme.colors.muted.to_color()),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        // Calculate per-project stats
        for (i, project) in self.model.projects.values().enumerate() {
            if (i as u16) >= inner.height {
                break;
            }

            let project_tasks: Vec<_> = self
                .model
                .tasks
                .values()
                .filter(|t| t.project_id.as_ref() == Some(&project.id))
                .collect();

            let total = project_tasks.len();
            let completed = project_tasks
                .iter()
                .filter(|t| t.status.is_complete())
                .count();
            let rate = if total > 0 {
                (completed as f32 / total as f32) * 100.0
            } else {
                0.0
            };

            // Truncate project name if needed
            let max_name_len = (inner.width as usize).saturating_sub(12);
            let name = if project.name.len() > max_name_len {
                format!("{}…", &project.name[..max_name_len.saturating_sub(1)])
            } else {
                project.name.clone()
            };

            let rate_color = if rate >= 75.0 {
                Color::Green
            } else if rate >= 50.0 {
                Color::Yellow
            } else if rate > 0.0 {
                Color::Red
            } else {
                theme.colors.muted.to_color()
            };

            let line = Line::from(vec![
                Span::styled(name, Style::default()),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{rate:.0}%"), Style::default().fg(rate_color)),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }

    pub(crate) fn render_status_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Status Distribution ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let stats = DashboardStats::new(self.model);
        let (todo, in_progress, blocked, done, cancelled) = stats.status_counts();
        let total = self.model.tasks.len();

        let statuses = [
            ("Todo", todo, theme.status.pending.to_color()),
            (
                "In Progress",
                in_progress,
                theme.status.in_progress.to_color(),
            ),
            ("Blocked", blocked, theme.colors.danger.to_color()),
            ("Done", done, theme.status.done.to_color()),
            ("Cancelled", cancelled, theme.status.cancelled.to_color()),
        ];

        for (i, (label, count, color)) in statuses.iter().enumerate() {
            if (i as u16) >= inner.height {
                break;
            }

            // Create a simple bar
            let pct = if total > 0 {
                (*count as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            let bar_width = ((f32::from(inner.width) - 18.0) * (pct / 100.0)) as usize;
            let bar = "█".repeat(bar_width.min(20));

            let line = Line::from(vec![
                Span::styled(format!("{label:<11}"), Style::default()),
                Span::styled(format!("{count:>3} "), Style::default().fg(*color)),
                Span::styled(bar, Style::default().fg(*color)),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }

    pub(crate) fn render_activity_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" This Week ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let stats = DashboardStats::new(self.model);
        let created = stats.tasks_this_week();
        let completed = stats.completed_this_week();
        let total_tasks = self.model.tasks.len();
        let active_tasks = self
            .model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::InProgress)
            .count();

        let lines = [
            Line::from(vec![
                Span::styled(
                    "Created: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{created}"),
                    Style::default().fg(theme.colors.accent.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Completed: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(format!("{completed}"), Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Active: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{active_tasks}"),
                    Style::default().fg(theme.status.in_progress.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Total: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(format!("{total_tasks}"), Style::default()),
            ]),
        ];

        for (i, line) in lines.iter().enumerate() {
            if (i as u16) < inner.height {
                buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
            }
        }
    }

    pub(crate) fn render_estimation_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Estimation ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let stats = DashboardStats::new(self.model);
        let (total_est, total_actual, over, under, on_target, avg_accuracy) =
            stats.estimation_stats();

        // Calculate total variance
        let total_variance = i64::from(total_actual) - i64::from(total_est);
        let variance_str = if total_variance > 0 {
            format!("+{}", format_duration(total_variance as u32))
        } else if total_variance < 0 {
            format!("-{}", format_duration((-total_variance) as u32))
        } else {
            "on target".to_string()
        };
        let variance_color = if total_variance > 0 {
            theme.colors.danger.to_color()
        } else if total_variance < 0 {
            Color::Green
        } else {
            theme.colors.accent.to_color()
        };

        // Accuracy color
        let accuracy_color = avg_accuracy.map_or(theme.colors.muted.to_color(), |acc| {
            if (90.0..=110.0).contains(&acc) {
                Color::Green
            } else if (70.0..=130.0).contains(&acc) {
                Color::Yellow
            } else {
                Color::Red
            }
        });

        let lines = [
            Line::from(vec![
                Span::styled(
                    "Accuracy: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    avg_accuracy.map_or("N/A".to_string(), |a| format!("{a:.0}%")),
                    Style::default()
                        .fg(accuracy_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Variance: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(variance_str, Style::default().fg(variance_color)),
            ]),
            Line::from(vec![Span::styled(
                format!(
                    "Est: {} | Act: {}",
                    format_duration(total_est),
                    format_duration(total_actual)
                ),
                Style::default().fg(theme.colors.muted.to_color()),
            )]),
            Line::from(vec![
                Span::styled("Over: ", Style::default().fg(theme.colors.muted.to_color())),
                Span::styled(
                    format!("{over}"),
                    Style::default().fg(theme.colors.danger.to_color()),
                ),
                Span::styled(
                    " Under: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(format!("{under}"), Style::default().fg(Color::Green)),
                Span::styled(" OK: ", Style::default().fg(theme.colors.muted.to_color())),
                Span::styled(
                    format!("{on_target}"),
                    Style::default().fg(theme.colors.accent.to_color()),
                ),
            ]),
        ];

        for (i, line) in lines.iter().enumerate() {
            if (i as u16) < inner.height {
                buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
            }
        }
    }

    pub(crate) fn render_focus_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Focus Sessions ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let stats = &self.model.pomodoro_stats;
        let has_active = self.model.pomodoro_session.is_some();

        // Active session indicator
        let active_indicator = if has_active {
            Span::styled(
                "● Active",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("○ Idle", Style::default().fg(theme.colors.muted.to_color()))
        };

        let lines = [
            Line::from(vec![
                Span::styled(
                    "Today: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{} 🍅", stats.cycles_today()),
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" | ", Style::default().fg(theme.colors.muted.to_color())),
                active_indicator,
            ]),
            Line::from(vec![
                Span::styled(
                    "All time: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!(
                        "{} cycles | {}",
                        stats.total_cycles,
                        format_duration(stats.total_work_mins)
                    ),
                    Style::default(),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Streak: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{} days", stats.current_streak()),
                    Style::default().fg(if stats.current_streak() > 0 {
                        Color::Green
                    } else {
                        theme.colors.muted.to_color()
                    }),
                ),
                Span::styled(
                    format!(" (best: {})", stats.longest_streak),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
            ]),
        ];

        for (i, line) in lines.iter().enumerate() {
            if (i as u16) < inner.height {
                buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
            }
        }
    }
}
