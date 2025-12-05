use chrono::{Datelike, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Priority, TaskStatus};

/// Statistics dashboard widget
pub struct Dashboard<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Dashboard<'a> {
    pub fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Calculate completion rate as percentage
    fn completion_rate(&self) -> f32 {
        let total = self.model.tasks.len();
        if total == 0 {
            return 0.0;
        }
        let completed = self
            .model
            .tasks
            .values()
            .filter(|t| t.status.is_complete())
            .count();
        (completed as f32 / total as f32) * 100.0
    }

    /// Calculate completion rate by priority
    fn completion_by_priority(&self, priority: Priority) -> (usize, usize) {
        let tasks: Vec<_> = self
            .model
            .tasks
            .values()
            .filter(|t| t.priority == priority)
            .collect();
        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.status.is_complete()).count();
        (completed, total)
    }

    /// Get total time tracked across all tasks
    fn total_time_tracked(&self) -> u32 {
        self.model
            .time_entries
            .values()
            .map(|e| e.calculated_duration_minutes())
            .sum()
    }

    /// Get count of overdue tasks
    fn overdue_count(&self) -> usize {
        let today = Utc::now().date_naive();
        self.model
            .tasks
            .values()
            .filter(|t| {
                t.due_date
                    .map(|d| d < today && !t.status.is_complete())
                    .unwrap_or(false)
            })
            .count()
    }

    /// Count tasks by status
    fn status_counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut todo = 0;
        let mut in_progress = 0;
        let mut blocked = 0;
        let mut done = 0;
        let mut cancelled = 0;

        for task in self.model.tasks.values() {
            match task.status {
                TaskStatus::Todo => todo += 1,
                TaskStatus::InProgress => in_progress += 1,
                TaskStatus::Blocked => blocked += 1,
                TaskStatus::Done => done += 1,
                TaskStatus::Cancelled => cancelled += 1,
            }
        }

        (todo, in_progress, blocked, done, cancelled)
    }

    /// Get tasks created this week
    fn tasks_this_week(&self) -> usize {
        let today = Utc::now().date_naive();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);

        self.model
            .tasks
            .values()
            .filter(|t| t.created_at.date_naive() >= week_start)
            .count()
    }

    /// Get tasks completed this week
    fn completed_this_week(&self) -> usize {
        let today = Utc::now().date_naive();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);

        self.model
            .tasks
            .values()
            .filter(|t| {
                t.completed_at
                    .map(|d| d.date_naive() >= week_start)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Format minutes as hours and minutes
    fn format_duration(minutes: u32) -> String {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }
}

impl Widget for Dashboard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Split into 2 columns
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left column: 3 panels
        let left_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Completion
                Constraint::Length(6), // Time Tracking
                Constraint::Min(5),    // Projects
            ])
            .split(columns[0]);

        // Right column: 2 panels
        let right_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Status Distribution
                Constraint::Min(5),    // Weekly Activity
            ])
            .split(columns[1]);

        // Render each panel
        self.render_completion_panel(left_panels[0], buf, theme);
        self.render_time_panel(left_panels[1], buf, theme);
        self.render_projects_panel(left_panels[2], buf, theme);
        self.render_status_panel(right_panels[0], buf, theme);
        self.render_activity_panel(right_panels[1], buf, theme);
    }
}

impl Dashboard<'_> {
    fn render_completion_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Completion ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let rate = self.completion_rate();
        let overdue = self.overdue_count();

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
                    format!("{:.0}%", rate),
                    Style::default().fg(rate_color).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Overdue: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{}", overdue),
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

    fn priority_completion_line(
        &self,
        priority: Priority,
        label: &str,
        theme: &Theme,
    ) -> Line<'static> {
        let (completed, total) = self.completion_by_priority(priority);
        let rate = if total > 0 {
            (completed as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Line::from(vec![
            Span::styled(
                format!("{}: ", label),
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                format!("{}/{} ({:.0}%)", completed, total, rate),
                Style::default(),
            ),
        ])
    }

    fn render_time_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Time Tracking ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let total_time = self.total_time_tracked();
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
                    Self::format_duration(total_time),
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
                Span::styled(Self::format_duration(avg_time), Style::default()),
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

    fn render_projects_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
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
                Span::styled(format!("{:.0}%", rate), Style::default().fg(rate_color)),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }

    fn render_status_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Status Distribution ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let (todo, in_progress, blocked, done, cancelled) = self.status_counts();
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
            let bar_width = ((inner.width as f32 - 18.0) * (pct / 100.0)) as usize;
            let bar = "█".repeat(bar_width.min(20));

            let line = Line::from(vec![
                Span::styled(format!("{:<11}", label), Style::default()),
                Span::styled(format!("{:>3} ", count), Style::default().fg(*color)),
                Span::styled(bar, Style::default().fg(*color)),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }

    fn render_activity_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" This Week ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let created = self.tasks_this_week();
        let completed = self.completed_this_week();
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
                    format!("{}", created),
                    Style::default().fg(theme.colors.accent.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Completed: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(format!("{}", completed), Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Active: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{}", active_tasks),
                    Style::default().fg(theme.status.in_progress.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Total: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(format!("{}", total_tasks), Style::default()),
            ]),
        ];

        for (i, line) in lines.iter().enumerate() {
            if (i as u16) < inner.height {
                buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
            }
        }
    }
}
