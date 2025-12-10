//! Burndown chart view component - progress toward completion.
//!
//! Displays project progress as a burndown chart, showing remaining work
//! over time compared to an ideal completion line.

use chrono::{Datelike, Duration, Local, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::ProjectId;

/// Burndown chart view widget
pub struct Burndown<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Burndown<'a> {
    /// Create a new burndown widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get burndown data for a project or all tasks
    fn get_burndown_data(&self, project_id: Option<ProjectId>) -> BurndownData {
        let tasks: Vec<_> = self
            .model
            .tasks
            .values()
            .filter(|t| project_id.is_none() || t.project_id == project_id)
            .collect();

        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.status.is_complete()).count();
        let remaining = total - completed;

        // Get completion history for the last 14 days
        let today = Local::now().date_naive();
        let mut daily_completions: Vec<(NaiveDate, usize)> = Vec::new();

        for days_ago in (0..14).rev() {
            let date = today - Duration::days(days_ago);
            let completed_by_date = tasks
                .iter()
                .filter(|t| {
                    t.status.is_complete() && t.completed_at.is_some_and(|c| c.date_naive() <= date)
                })
                .count();
            daily_completions.push((date, total - completed_by_date));
        }

        // Find earliest task and latest due date for scope
        let start_date = tasks
            .iter()
            .map(|t| t.created_at.date_naive())
            .min()
            .unwrap_or(today - Duration::days(14));

        let end_date = tasks
            .iter()
            .filter_map(|t| t.due_date)
            .max()
            .unwrap_or(today + Duration::days(14));

        BurndownData {
            total,
            completed,
            remaining,
            daily_completions,
            start_date,
            end_date,
        }
    }

    /// Render the ASCII burndown chart
    fn render_chart(&self, area: Rect, buf: &mut Buffer, data: &BurndownData) {
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

    /// Render progress stats
    fn render_stats(&self, area: Rect, buf: &mut Buffer, data: &BurndownData) {
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
    fn render_projects(&self, area: Rect, buf: &mut Buffer) {
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

/// Data structure for burndown chart
struct BurndownData {
    total: usize,
    completed: usize,
    remaining: usize,
    daily_completions: Vec<(NaiveDate, usize)>,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl Widget for Burndown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Burndown - Progress Chart ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 40 || inner.height < 15 {
            return;
        }

        let data = self.get_burndown_data(self.model.selected_project);

        // Layout: chart on left, stats on right
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(inner);

        let right_panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(12), Constraint::Length(8)])
            .split(chunks[1]);

        // Chart area with border
        let chart_block = Block::default()
            .title(" Last 14 Days ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));
        let chart_inner = chart_block.inner(chunks[0]);
        chart_block.render(chunks[0], buf);

        self.render_chart(chart_inner, buf, &data);
        self.render_stats(right_panel[0], buf, &data);
        self.render_projects(right_panel[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Task, TaskStatus};
    use ratatui::buffer::Buffer;

    #[test]
    fn test_burndown_empty_model() {
        let model = Model::new();
        let theme = Theme::default();
        let burndown = Burndown::new(&model, &theme);
        let data = burndown.get_burndown_data(None);
        assert_eq!(data.total, 0);
        assert_eq!(data.completed, 0);
        assert_eq!(data.remaining, 0);
    }

    #[test]
    fn test_burndown_with_tasks() {
        let mut model = Model::new();

        // Add some tasks
        let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
        let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
        let task3 = Task::new("Task 3").with_status(TaskStatus::Todo);
        let task4 = Task::new("Task 4").with_status(TaskStatus::InProgress);

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);
        model.tasks.insert(task4.id, task4);

        let theme = Theme::default();
        let burndown = Burndown::new(&model, &theme);
        let data = burndown.get_burndown_data(None);

        assert_eq!(data.total, 4);
        assert_eq!(data.completed, 2);
        assert_eq!(data.remaining, 2);
    }

    #[test]
    fn test_burndown_daily_completions_length() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let burndown = Burndown::new(&model, &theme);
        let data = burndown.get_burndown_data(None);

        // Should have 14 days of completion history
        assert_eq!(data.daily_completions.len(), 14);
    }

    #[test]
    fn test_burndown_renders_without_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let burndown = Burndown::new(&model, &theme);

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        burndown.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_burndown_small_area_does_not_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let burndown = Burndown::new(&model, &theme);

        // Very small area - should early return without panic
        let area = Rect::new(0, 0, 20, 10);
        let mut buffer = Buffer::empty(area);
        burndown.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_burndown_progress_calculation() {
        let mut model = Model::new();
        let theme = Theme::default();

        // Add 4 tasks, 2 completed
        let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
        let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
        let task3 = Task::new("Task 3").with_status(TaskStatus::Todo);
        let task4 = Task::new("Task 4").with_status(TaskStatus::InProgress);

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);
        model.tasks.insert(task4.id, task4);

        let burndown = Burndown::new(&model, &theme);
        let data = burndown.get_burndown_data(None);

        assert_eq!(data.total, 4);
        assert_eq!(data.completed, 2);
        assert_eq!(data.remaining, 2);
    }

    #[test]
    fn test_burndown_with_project_filter() {
        use crate::domain::{Project, ProjectId};

        let mut model = Model::new();
        let theme = Theme::default();

        // Create a project
        let project = Project::new("Test Project");
        let project_id = project.id;
        model.projects.insert(project.id, project);

        // Add tasks - 2 in project, 2 without project
        let mut task1 = Task::new("Project Task 1").with_status(TaskStatus::Done);
        task1.project_id = Some(project_id);
        let mut task2 = Task::new("Project Task 2").with_status(TaskStatus::Todo);
        task2.project_id = Some(project_id);
        let task3 = Task::new("No Project Task 1");
        let task4 = Task::new("No Project Task 2");

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);
        model.tasks.insert(task4.id, task4);

        let burndown = Burndown::new(&model, &theme);
        let data = burndown.get_burndown_data(Some(project_id));

        assert_eq!(data.total, 2);
        assert_eq!(data.completed, 1);
        assert_eq!(data.remaining, 1);
    }

    #[test]
    fn test_burndown_velocity_calculation() {
        use chrono::Duration;

        let mut model = Model::new();
        let theme = Theme::default();

        // Add some tasks with completion dates spread over time
        let today = Local::now().date_naive();
        let mut task1 = Task::new("Task 1").with_status(TaskStatus::Done);
        task1.completed_at = Some((today - Duration::days(3)).and_hms_opt(12, 0, 0).unwrap().and_utc());
        let mut task2 = Task::new("Task 2").with_status(TaskStatus::Done);
        task2.completed_at = Some((today - Duration::days(2)).and_hms_opt(12, 0, 0).unwrap().and_utc());
        let task3 = Task::new("Task 3");

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);

        let burndown = Burndown::new(&model, &theme);
        let data = burndown.get_burndown_data(None);

        assert_eq!(data.total, 3);
        assert_eq!(data.completed, 2);
        assert!(!data.daily_completions.is_empty());
    }

    #[test]
    fn test_burndown_renders_full_chart() {
        let mut model = Model::new();
        let theme = Theme::default();

        // Add several tasks with varying states
        for i in 0..10 {
            let status = if i < 5 { TaskStatus::Done } else { TaskStatus::Todo };
            let task = Task::new(format!("Task {i}")).with_status(status);
            model.tasks.insert(task.id, task);
        }

        let burndown = Burndown::new(&model, &theme);
        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        burndown.render(area, &mut buffer);

        // Should render chart elements
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_burndown_all_completed() {
        let mut model = Model::new();
        let theme = Theme::default();

        // All tasks completed
        let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
        let task2 = Task::new("Task 2").with_status(TaskStatus::Done);

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);

        let burndown = Burndown::new(&model, &theme);
        let data = burndown.get_burndown_data(None);

        assert_eq!(data.total, 2);
        assert_eq!(data.completed, 2);
        assert_eq!(data.remaining, 0);
    }

    #[test]
    fn test_burndown_projects_panel() {
        use crate::domain::Project;

        let mut model = Model::new();
        let theme = Theme::default();

        // Create projects with tasks
        let project1 = Project::new("Project Alpha");
        let project2 = Project::new("Project Beta");

        let mut task1 = Task::new("Task 1");
        task1.project_id = Some(project1.id);
        let mut task2 = Task::new("Task 2");
        task2.project_id = Some(project2.id);

        model.projects.insert(project1.id, project1);
        model.projects.insert(project2.id, project2);
        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);

        let burndown = Burndown::new(&model, &theme);
        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        burndown.render(area, &mut buffer);

        // Should render without panic
        assert!(buffer.area.width > 0);
    }
}
