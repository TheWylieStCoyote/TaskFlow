//! Section rendering methods for TaskDetail.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::domain::Task;

use super::TaskDetail;

impl TaskDetail<'_> {
    /// Render the header line with title, status, and priority.
    pub(super) fn render_header(&self, task: &Task) -> Line<'static> {
        let theme = self.theme;

        let status_style = Style::default().fg(match task.status {
            crate::domain::TaskStatus::Done => theme.status.done.to_color(),
            crate::domain::TaskStatus::InProgress => theme.status.in_progress.to_color(),
            crate::domain::TaskStatus::Blocked => theme.colors.danger.to_color(),
            crate::domain::TaskStatus::Cancelled => theme.status.cancelled.to_color(),
            crate::domain::TaskStatus::Todo => theme.status.pending.to_color(),
        });

        let priority_style = Style::default().fg(match task.priority {
            crate::domain::Priority::Urgent => theme.priority.urgent.to_color(),
            crate::domain::Priority::High => theme.priority.high.to_color(),
            crate::domain::Priority::Medium => theme.priority.medium.to_color(),
            crate::domain::Priority::Low => theme.priority.low.to_color(),
            crate::domain::Priority::None => theme.colors.muted.to_color(),
        });

        Line::from(vec![
            Span::styled(
                format!("{} ", task.status.symbol()),
                status_style.add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                task.priority.symbol().to_string(),
                priority_style.add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                task.title.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ])
    }

    /// Render metadata: project, tags, ID.
    pub(super) fn render_metadata(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        // Section header
        lines.push(Line::from(Span::styled(
            "── Metadata ──",
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Project
        let project_name = task
            .project_id
            .and_then(|id| self.model.projects.get(&id))
            .map_or_else(|| "(none)".to_string(), |p| p.name.clone());
        lines.push(Line::from(vec![
            Span::styled(
                "Project: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::raw(project_name),
        ]));

        // Tags
        let tags_str = if task.tags.is_empty() {
            "(none)".to_string()
        } else {
            task.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        lines.push(Line::from(vec![
            Span::styled("Tags: ", Style::default().fg(theme.colors.muted.to_color())),
            Span::raw(tags_str),
        ]));

        // Task ID (abbreviated)
        let id_str = task.id.to_string();
        let short_id = if id_str.len() > 8 {
            &id_str[..8]
        } else {
            &id_str
        };
        lines.push(Line::from(vec![
            Span::styled("ID: ", Style::default().fg(theme.colors.muted.to_color())),
            Span::styled(
                short_id.to_string(),
                Style::default().fg(theme.colors.muted.to_color()),
            ),
        ]));

        lines
    }

    /// Render dates: created, updated, due, scheduled, completed.
    pub(super) fn render_dates(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        // Section header
        lines.push(Line::from(Span::styled(
            "── Dates ──",
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Created
        lines.push(Line::from(vec![
            Span::styled(
                "Created: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::raw(task.created_at.format("%Y-%m-%d %H:%M").to_string()),
        ]));

        // Updated
        lines.push(Line::from(vec![
            Span::styled(
                "Updated: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::raw(task.updated_at.format("%Y-%m-%d %H:%M").to_string()),
        ]));

        // Due date
        if let Some(due) = task.due_date {
            let style = if task.is_overdue() {
                Style::default().fg(theme.colors.danger.to_color())
            } else if task.is_due_today() {
                Style::default().fg(theme.colors.warning.to_color())
            } else {
                Style::default()
            };
            lines.push(Line::from(vec![
                Span::styled("Due: ", Style::default().fg(theme.colors.muted.to_color())),
                Span::styled(due.format("%Y-%m-%d").to_string(), style),
            ]));
        }

        // Scheduled date
        if let Some(sched) = task.scheduled_date {
            lines.push(Line::from(vec![
                Span::styled(
                    "Scheduled: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(sched.format("%Y-%m-%d").to_string()),
            ]));
        }

        // Scheduled time block
        if let Some(time_display) = task.scheduled_time_display() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Time Block: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(time_display),
            ]));
        }

        // Completed date
        if let Some(completed) = task.completed_at {
            lines.push(Line::from(vec![
                Span::styled(
                    "Completed: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    completed.format("%Y-%m-%d %H:%M").to_string(),
                    Style::default().fg(theme.status.done.to_color()),
                ),
            ]));
        }

        // Snooze
        if let Some(snooze) = task.snooze_until {
            lines.push(Line::from(vec![
                Span::styled(
                    "Snoozed until: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(snooze.format("%Y-%m-%d").to_string()),
            ]));
        }

        // Recurrence
        if let Some(ref recurrence) = task.recurrence {
            lines.push(Line::from(vec![
                Span::styled(
                    "Recurrence: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(recurrence.to_string()),
            ]));
        }

        lines
    }

    /// Render description section.
    pub(super) fn render_description(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        // Section header
        lines.push(Line::from(Span::styled(
            "── Description ──",
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        if let Some(ref desc) = task.description {
            for line in desc.lines() {
                lines.push(Line::from(line.to_string()));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "(no description)",
                Style::default().fg(theme.colors.muted.to_color()),
            )));
        }

        lines.push(Line::from(""));
        lines
    }

    /// Render time tracking section.
    pub(super) fn render_time_tracking(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        // Section header
        lines.push(Line::from(Span::styled(
            "── Time Tracking ──",
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Estimated
        let est_str = task
            .estimated_minutes
            .map_or_else(|| "(not set)".to_string(), format_duration);
        lines.push(Line::from(vec![
            Span::styled(
                "Estimated: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::raw(est_str),
        ]));

        // Actual
        let actual_str = if task.actual_minutes > 0 {
            format_duration(task.actual_minutes)
        } else {
            "(none)".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(
                "Actual: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::raw(actual_str),
        ]));

        // Variance
        if let Some(variance_text) = task.time_variance_display() {
            let variance = task.time_variance().unwrap_or(0);
            let style = if variance > 0 {
                Style::default().fg(theme.colors.danger.to_color())
            } else if variance < 0 {
                Style::default().fg(theme.status.done.to_color())
            } else {
                Style::default().fg(theme.colors.accent.to_color())
            };
            lines.push(Line::from(vec![
                Span::styled(
                    "Variance: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(variance_text, style),
            ]));
        }

        lines.push(Line::from(""));
        lines
    }

    /// Render subtasks section.
    pub(super) fn render_subtasks(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        // Get subtasks
        let subtasks: Vec<_> = self
            .model
            .tasks
            .values()
            .filter(|t| t.parent_task_id == Some(task.id))
            .collect();

        if subtasks.is_empty() {
            return lines;
        }

        // Section header
        let completed = subtasks.iter().filter(|t| t.status.is_complete()).count();
        let total = subtasks.len();
        lines.push(Line::from(Span::styled(
            format!("── Subtasks ({completed}/{total}) ──"),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        for subtask in subtasks {
            let status_style = if subtask.status.is_complete() {
                Style::default().fg(theme.status.done.to_color())
            } else {
                Style::default().fg(theme.status.pending.to_color())
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{} ", subtask.status.symbol()), status_style),
                Span::raw(subtask.title.clone()),
            ]));
        }

        lines.push(Line::from(""));
        lines
    }

    /// Render dependencies section.
    pub(super) fn render_dependencies(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        if task.dependencies.is_empty() {
            return lines;
        }

        // Section header
        lines.push(Line::from(Span::styled(
            "── Blocked By ──",
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        for dep_id in &task.dependencies {
            if let Some(dep_task) = self.model.tasks.get(dep_id) {
                let status_style = if dep_task.status.is_complete() {
                    Style::default().fg(theme.status.done.to_color())
                } else {
                    Style::default().fg(theme.colors.warning.to_color())
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{} ", dep_task.status.symbol()), status_style),
                    Span::raw(dep_task.title.clone()),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines
    }

    /// Render task chain section.
    pub(super) fn render_task_chain(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        // Find previous task in chain (task that links to this one)
        let prev_task = self
            .model
            .tasks
            .values()
            .find(|t| t.next_task_id == Some(task.id));

        let next_task = task.next_task_id.and_then(|id| self.model.tasks.get(&id));

        if prev_task.is_none() && next_task.is_none() {
            return lines;
        }

        // Section header
        lines.push(Line::from(Span::styled(
            "── Task Chain ──",
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        if let Some(prev) = prev_task {
            lines.push(Line::from(vec![
                Span::styled(
                    "← Previous: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(prev.title.clone()),
            ]));
        }

        if let Some(next) = next_task {
            lines.push(Line::from(vec![
                Span::styled(
                    "→ Next: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(next.title.clone()),
            ]));
        }

        lines.push(Line::from(""));
        lines
    }

    /// Render git integration section.
    /// Note: Git integration info is stored separately from tasks.
    /// This section is a placeholder for future integration.
    #[allow(clippy::unused_self)]
    pub(super) fn render_git_info(&self, _task: &Task) -> Vec<Line<'static>> {
        // Git integration info is handled by the GitTodos view
        // which scans the repository for TODO comments.
        // Tasks don't currently have direct git references stored on them.
        Vec::new()
    }

    /// Render time entries section.
    pub(super) fn render_time_entries(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        let entries = self.model.time_entries_for_task(&task.id);

        if entries.is_empty() {
            return lines;
        }

        // Section header
        lines.push(Line::from(Span::styled(
            format!("── Time Entries ({}) ──", entries.len()),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Show last 5 entries
        for entry in entries.iter().take(5) {
            let duration = entry.calculated_duration_minutes();
            let date = entry.started_at.format("%m/%d %H:%M");
            let is_running = entry.ended_at.is_none();

            let status = if is_running {
                Span::styled(
                    " (running)",
                    Style::default().fg(theme.colors.warning.to_color()),
                )
            } else {
                Span::raw("")
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{date}"),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(format!(" - {}", format_duration(duration))),
                status,
            ]));
        }

        if entries.len() > 5 {
            lines.push(Line::from(Span::styled(
                format!("  ... and {} more", entries.len() - 5),
                Style::default().fg(theme.colors.muted.to_color()),
            )));
        }

        lines.push(Line::from(""));
        lines
    }

    /// Render audit history section.
    pub(super) fn render_history(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        let entries = self.model.audit_log_for_task(&task.id);

        if entries.is_empty() {
            return lines;
        }

        lines.push(Line::from(Span::styled(
            format!("── History ({}) ──", entries.len()),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Show up to 10 most recent entries (already sorted newest-first)
        for entry in entries.iter().take(10) {
            let timestamp = entry.formatted_timestamp();
            let action = entry.action.to_string();

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    timestamp,
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw("  "),
                Span::styled(action, Style::default().fg(theme.colors.accent.to_color())),
            ]));

            // Show field changes indented below
            for change in &entry.changes {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("{}:", change.field),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::raw(format!(" {} → {}", change.old_value, change.new_value)),
                ]));
            }
        }

        if entries.len() > 10 {
            lines.push(Line::from(Span::styled(
                format!("  ... and {} more", entries.len() - 10),
                Style::default().fg(theme.colors.muted.to_color()),
            )));
        }

        lines.push(Line::from(""));
        lines
    }

    /// Render work logs section.
    pub(super) fn render_work_logs(&self, task: &Task) -> Vec<Line<'static>> {
        let theme = self.theme;
        let mut lines = Vec::new();

        let logs = self.model.work_logs_for_task(&task.id);

        if logs.is_empty() {
            return lines;
        }

        // Section header
        lines.push(Line::from(Span::styled(
            format!("── Work Logs ({}) ──", logs.len()),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Show last 3 entries (just summaries)
        for log in logs.iter().take(3) {
            let date = log.created_at.format("%m/%d %H:%M");
            let summary = log.summary();

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{date}"),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::raw(format!(" - {summary}")),
            ]));
        }

        if logs.len() > 3 {
            lines.push(Line::from(Span::styled(
                format!("  ... and {} more", logs.len() - 3),
                Style::default().fg(theme.colors.muted.to_color()),
            )));
        }

        lines
    }
}

/// Format duration in minutes as a human-readable string.
fn format_duration(minutes: u32) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}
