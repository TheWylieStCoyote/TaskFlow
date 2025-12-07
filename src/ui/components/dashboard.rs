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
use crate::domain::{Priority, TaskStatus, TimeEntry};

/// Statistics dashboard widget
pub struct Dashboard<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Dashboard<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
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
            .map(TimeEntry::calculated_duration_minutes)
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
                    .is_some_and(|d| d < today && !t.status.is_complete())
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
            .filter(|t| t.completed_at.is_some_and(|d| d.date_naive() >= week_start))
            .count()
    }

    /// Format minutes as hours and minutes
    fn format_duration(minutes: u32) -> String {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if hours > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{mins}m")
        }
    }

    /// Calculate estimation accuracy statistics
    /// Returns (total_estimated, total_actual, over_count, under_count, on_target_count, avg_accuracy)
    fn estimation_stats(&self) -> (u32, u32, usize, usize, usize, Option<f64>) {
        let mut total_estimated: u32 = 0;
        let mut total_actual: u32 = 0;
        let mut over_count = 0;
        let mut under_count = 0;
        let mut on_target_count = 0;
        let mut accuracies: Vec<f64> = Vec::new();

        for task in self.model.tasks.values() {
            if let Some(est) = task.estimated_minutes {
                total_estimated = total_estimated.saturating_add(est);
                total_actual = total_actual.saturating_add(task.actual_minutes);

                if let Some(variance) = task.time_variance() {
                    match variance.cmp(&0) {
                        std::cmp::Ordering::Greater => over_count += 1,
                        std::cmp::Ordering::Less => under_count += 1,
                        std::cmp::Ordering::Equal => on_target_count += 1,
                    }
                }

                if let Some(accuracy) = task.estimation_accuracy() {
                    accuracies.push(accuracy);
                }
            }
        }

        let avg_accuracy = if accuracies.is_empty() {
            None
        } else {
            Some(accuracies.iter().sum::<f64>() / accuracies.len() as f64)
        };

        (
            total_estimated,
            total_actual,
            over_count,
            under_count,
            on_target_count,
            avg_accuracy,
        )
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

        // Right column: 3 panels
        let right_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Status Distribution
                Constraint::Length(7), // Estimation
                Constraint::Min(5),    // Weekly Activity
            ])
            .split(columns[1]);

        // Render each panel
        self.render_completion_panel(left_panels[0], buf, theme);
        self.render_time_panel(left_panels[1], buf, theme);
        self.render_projects_panel(left_panels[2], buf, theme);
        self.render_status_panel(right_panels[0], buf, theme);
        self.render_estimation_panel(right_panels[1], buf, theme);
        self.render_activity_panel(right_panels[2], buf, theme);
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
                format!("{label}: "),
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                format!("{completed}/{total} ({rate:.0}%)"),
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
                Span::styled(format!("{rate:.0}%"), Style::default().fg(rate_color)),
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
                Span::styled(format!("{label:<11}"), Style::default()),
                Span::styled(format!("{count:>3} "), Style::default().fg(*color)),
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

    fn render_estimation_panel(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Estimation ")
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        let (total_est, total_actual, over, under, on_target, avg_accuracy) =
            self.estimation_stats();

        // Calculate total variance
        let total_variance = total_actual as i64 - total_est as i64;
        let variance_str = if total_variance > 0 {
            format!("+{}", Self::format_duration(total_variance as u32))
        } else if total_variance < 0 {
            format!("-{}", Self::format_duration((-total_variance) as u32))
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
                    Self::format_duration(total_est),
                    Self::format_duration(total_actual)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;
    use crate::domain::Task;

    /// Helper to render a widget into a buffer
    fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
        buffer
    }

    /// Extract text content from buffer
    fn buffer_content(buffer: &Buffer) -> String {
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push(
                    buffer
                        .cell((x, y))
                        .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
                );
            }
            content.push('\n');
        }
        content
    }

    #[test]
    fn test_dashboard_renders_completion_panel() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Completion"),
            "Completion panel should be visible"
        );
    }

    #[test]
    fn test_dashboard_renders_time_tracking_panel() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Time Tracking"),
            "Time Tracking panel should be visible"
        );
    }

    #[test]
    fn test_dashboard_renders_projects_panel() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Projects"),
            "Projects panel should be visible"
        );
    }

    #[test]
    fn test_dashboard_renders_status_distribution_panel() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Status Distribution"),
            "Status Distribution panel should be visible"
        );
    }

    #[test]
    fn test_dashboard_renders_this_week_panel() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("This Week"),
            "This Week panel should be visible"
        );
    }

    #[test]
    fn test_dashboard_shows_overall_completion_rate() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Overall"),
            "Overall completion rate should be visible"
        );
    }

    #[test]
    fn test_dashboard_shows_overdue_count() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Overdue"),
            "Overdue count should be visible"
        );
    }

    #[test]
    fn test_dashboard_shows_tracking_status() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        // Should show Tracking: with either Active or Idle
        assert!(
            content.contains("Tracking"),
            "Tracking status should be visible"
        );
    }

    #[test]
    fn test_dashboard_shows_status_types() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        // Status distribution should show task statuses
        assert!(
            content.contains("Todo") || content.contains("Done"),
            "Status types should be visible"
        );
    }

    #[test]
    fn test_dashboard_shows_no_projects_when_empty() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 25);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("No projects"),
            "Should show 'No projects' when empty"
        );
    }

    #[test]
    fn test_dashboard_with_sample_data() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);
        let buffer = render_widget(dashboard, 80, 30);

        // Should render without panic
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_dashboard_completion_rate_calculation() {
        let mut model = Model::new();

        // Add 4 tasks, 2 done, 2 not done
        let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
        let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
        let task3 = Task::new("Task 3").with_status(TaskStatus::Todo);
        let task4 = Task::new("Task 4").with_status(TaskStatus::Todo);

        model.tasks.insert(task1.id.clone(), task1);
        model.tasks.insert(task2.id.clone(), task2);
        model.tasks.insert(task3.id.clone(), task3);
        model.tasks.insert(task4.id.clone(), task4);

        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);

        // Completion rate should be 50%
        assert!((dashboard.completion_rate() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_dashboard_completion_rate_empty() {
        let model = Model::new();
        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);

        // No tasks = 0% completion
        assert_eq!(dashboard.completion_rate(), 0.0);
    }

    #[test]
    fn test_dashboard_status_counts() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1").with_status(TaskStatus::Todo);
        let task2 = Task::new("Task 2").with_status(TaskStatus::InProgress);
        let task3 = Task::new("Task 3").with_status(TaskStatus::Done);

        model.tasks.insert(task1.id.clone(), task1);
        model.tasks.insert(task2.id.clone(), task2);
        model.tasks.insert(task3.id.clone(), task3);

        let theme = Theme::default();
        let dashboard = Dashboard::new(&model, &theme);

        let (todo, in_progress, blocked, done, cancelled) = dashboard.status_counts();
        assert_eq!(todo, 1);
        assert_eq!(in_progress, 1);
        assert_eq!(blocked, 0);
        assert_eq!(done, 1);
        assert_eq!(cancelled, 0);
    }

    #[test]
    fn test_dashboard_format_duration() {
        assert_eq!(Dashboard::format_duration(30), "30m");
        assert_eq!(Dashboard::format_duration(60), "1h 0m");
        assert_eq!(Dashboard::format_duration(90), "1h 30m");
        assert_eq!(Dashboard::format_duration(125), "2h 5m");
    }
}
