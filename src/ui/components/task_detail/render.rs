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

        let mut entries = self.model.time_entries_for_task(&task.id);

        if entries.is_empty() {
            return lines;
        }

        // The model lists entries by day (chronological within a day); the
        // preview is truncated, so order it most recent first to keep the
        // latest entries visible.
        entries.sort_by_key(|e| std::cmp::Reverse(e.started_at));

        // Section header
        lines.push(Line::from(Span::styled(
            format!("── Time Entries ({}) ──", entries.len()),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )));

        // Show the 5 most recent entries
        for entry in entries.iter().take(5) {
            let duration = entry.calculated_duration_minutes();
            let date = entry
                .started_at
                .with_timezone(&chrono::Local)
                .format("%m/%d %H:%M");
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
    crate::domain::format_minutes(minutes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;
    use crate::domain::{today, Recurrence, TaskStatus, TimeEntry, WorkLogEntry};
    use crate::ui::test_utils::{buffer_content, render_widget};
    use chrono::{Duration, NaiveTime, Utc};

    /// Render the detail modal tall enough that every section is visible.
    fn render(model: &Model) -> String {
        let theme = Theme::default();
        let buffer = render_widget(TaskDetail::new(model, &theme, 0), 90, 80);
        buffer_content(&buffer)
    }

    fn model_with_task(task: Task) -> (Model, crate::domain::TaskId) {
        let mut model = Model::new();
        let id = task.id;
        model.tasks.insert(id, task);
        model.filtering.show_completed = true;
        model.refresh_visible_tasks();
        model.selected_index = model.visible_tasks.iter().position(|t| *t == id).unwrap();
        (model, id)
    }

    #[test]
    fn test_format_duration_variants() {
        assert_eq!(format_duration(0), "0m");
        assert_eq!(format_duration(45), "45m");
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(135), "2h 15m");
    }

    #[test]
    fn test_dates_section_shows_every_optional_date() {
        let mut task = Task::new("Dated task");
        task.due_date = Some(today() + Duration::days(3));
        task.scheduled_date = Some(today() + Duration::days(1));
        task.scheduled_start_time = NaiveTime::from_hms_opt(9, 0, 0);
        task.scheduled_end_time = NaiveTime::from_hms_opt(10, 30, 0);
        task.recurrence = Some(Recurrence::Daily);
        task.status = TaskStatus::Done;
        task.completed_at = Some(Utc::now());
        let (model, _) = model_with_task(task);

        let content = render(&model);

        assert!(content.contains("── Dates ──"));
        assert!(content.contains("Created:"));
        assert!(content.contains("Updated:"));
        assert!(content.contains(&format!(
            "Due: {}",
            (today() + Duration::days(3)).format("%Y-%m-%d")
        )));
        assert!(content.contains(&format!(
            "Scheduled: {}",
            (today() + Duration::days(1)).format("%Y-%m-%d")
        )));
        assert!(content.contains("Time Block: 09:00-10:30"));
        assert!(content.contains("Completed:"));
        assert!(content.contains(&format!("Recurrence: {}", Recurrence::Daily)));
    }

    #[test]
    fn test_dates_section_shows_snooze_in_snoozed_view() {
        let mut task = Task::new("Snoozed task");
        task.snooze_until = Some(today() + Duration::days(2));
        let mut model = Model::new();
        let id = task.id;
        model.tasks.insert(id, task);
        // Snoozed tasks are hidden from the default list; the Snoozed view shows them
        model.current_view = crate::app::ViewId::Snoozed;
        model.refresh_visible_tasks();
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|t| *t == id)
            .expect("snoozed view lists the task");

        let content = render(&model);

        assert!(content.contains(&format!(
            "Snoozed until: {}",
            (today() + Duration::days(2)).format("%Y-%m-%d")
        )));
    }

    #[test]
    fn test_dates_section_overdue_and_due_today_render_due_line() {
        let mut overdue = Task::new("Overdue");
        overdue.due_date = Some(today() - Duration::days(1));
        let (model, _) = model_with_task(overdue);
        assert!(render(&model).contains("Due:"));

        let mut due_today = Task::new("Due today");
        due_today.due_date = Some(today());
        let (model, _) = model_with_task(due_today);
        assert!(render(&model).contains(&format!("Due: {}", today().format("%Y-%m-%d"))));
    }

    #[test]
    fn test_dates_section_omits_absent_dates() {
        let (model, _) = model_with_task(Task::new("Bare"));
        let content = render(&model);
        assert!(!content.contains("Due:"));
        assert!(!content.contains("Scheduled:"));
        assert!(!content.contains("Snoozed until:"));
        assert!(!content.contains("Recurrence:"));
        assert!(!content.contains("Completed:"));
    }

    #[test]
    fn test_dependencies_section_lists_blockers_with_status() {
        let done_dep = Task::new("Finished blocker").with_status(TaskStatus::Done);
        let open_dep = Task::new("Open blocker");
        let mut blocked = Task::new("Blocked task");
        blocked.dependencies = vec![done_dep.id, open_dep.id, crate::domain::TaskId::new()];
        let (mut model, _) = model_with_task(blocked);
        model.tasks.insert(done_dep.id, done_dep);
        model.tasks.insert(open_dep.id, open_dep);

        let content = render(&model);

        assert!(content.contains("── Blocked By ──"));
        assert!(content.contains("Finished blocker"));
        assert!(content.contains("Open blocker"));
    }

    #[test]
    fn test_dependencies_section_absent_without_dependencies() {
        let (model, _) = model_with_task(Task::new("Independent"));
        assert!(!render(&model).contains("Blocked By"));
    }

    #[test]
    fn test_task_chain_section_shows_previous_and_next() {
        let next = Task::new("Chain next");
        let mut current = Task::new("Chain current");
        current.next_task_id = Some(next.id);
        let mut prev = Task::new("Chain previous");
        prev.next_task_id = Some(current.id);
        let (mut model, _) = model_with_task(current);
        model.tasks.insert(next.id, next);
        model.tasks.insert(prev.id, prev);

        let content = render(&model);

        assert!(content.contains("── Task Chain ──"));
        assert!(content.contains("← Previous: Chain previous"));
        assert!(content.contains("→ Next: Chain next"));
    }

    #[test]
    fn test_task_chain_section_with_only_next() {
        let next = Task::new("Only next");
        let mut current = Task::new("Head");
        current.next_task_id = Some(next.id);
        let (mut model, _) = model_with_task(current);
        model.tasks.insert(next.id, next);

        let content = render(&model);
        assert!(content.contains("→ Next: Only next"));
        assert!(!content.contains("← Previous:"));
    }

    #[test]
    fn test_task_chain_section_absent_when_unlinked() {
        let (model, _) = model_with_task(Task::new("Solo"));
        assert!(!render(&model).contains("Task Chain"));
    }

    #[test]
    fn test_time_entries_section_limits_to_five_and_marks_running() {
        let (mut model, task_id) = model_with_task(Task::new("Tracked"));
        for i in 0..7 {
            let mut entry = TimeEntry::start(task_id);
            entry.started_at = Utc::now() - Duration::hours(i + 1);
            entry.ended_at = Some(entry.started_at + Duration::minutes(30));
            entry.duration_minutes = Some(30);
            model.time_entries.insert(entry.id, entry);
        }
        let mut running = TimeEntry::start(task_id);
        running.started_at = Utc::now();
        model.time_entries.insert(running.id, running);
        model.refresh_visible_tasks();

        let content = render(&model);

        assert!(content.contains("── Time Entries (8) ──"));
        assert!(content.contains("(running)"));
        assert!(content.contains("... and 3 more"));
        assert!(content.contains(" - 30m"));
    }

    #[test]
    fn test_time_entries_section_absent_without_entries() {
        let (model, _) = model_with_task(Task::new("Untracked"));
        assert!(!render(&model).contains("Time Entries"));
    }

    #[test]
    fn test_work_logs_section_limits_to_three() {
        let (mut model, task_id) = model_with_task(Task::new("Logged"));
        for i in 0..5 {
            let mut log = WorkLogEntry::new(task_id, format!("Log entry {i}\nmore detail"));
            log.created_at = Utc::now() - Duration::minutes(i);
            model.work_logs.insert(log.id, log);
        }
        model.refresh_visible_tasks();

        let content = render(&model);

        assert!(content.contains("── Work Logs (5) ──"));
        assert!(content.contains(" - Log entry 0"));
        assert!(content.contains(" - Log entry 2"));
        assert!(!content.contains(" - Log entry 3"));
        assert!(content.contains("... and 2 more"));
    }

    #[test]
    fn test_history_section_shows_actions_and_field_changes() {
        let (mut model, task_id) = model_with_task(Task::new("Audited"));
        model.record_task_created(task_id);
        model.modify_task_with_undo(&task_id, |t| {
            t.title = "Audited (renamed)".to_string();
        });
        model.modify_task_with_undo(&task_id, |t| {
            t.toggle_complete();
        });
        let entries = model.audit_log_for_task(&task_id);
        assert_eq!(entries.len(), 3);
        let actions: Vec<String> = entries.iter().map(|e| e.action.to_string()).collect();

        let content = render(&model);

        assert!(content.contains("── History (3) ──"));
        for action in &actions {
            assert!(content.contains(action.as_str()), "missing action {action}");
        }
        assert!(content.contains("title:"));
        assert!(content.contains("Audited → Audited (renamed)"));
        assert!(!content.contains("... and"));
    }

    #[test]
    fn test_history_section_truncates_after_ten_entries() {
        let (mut model, task_id) = model_with_task(Task::new("Busy"));
        for i in 0..12 {
            model.modify_task_with_undo(&task_id, |t| {
                t.title = format!("Busy {i}");
            });
        }

        let content = render(&model);

        assert!(content.contains("── History (12) ──"));
        assert!(content.contains("... and 2 more"));
    }

    #[test]
    fn test_history_section_absent_without_entries() {
        let (model, _) = model_with_task(Task::new("Quiet"));
        assert!(!render(&model).contains("History ("));
    }

    #[test]
    fn test_subtasks_and_time_tracking_sections() {
        let mut parent = Task::new("Parent");
        parent.estimated_minutes = Some(60);
        parent.actual_minutes = 90;
        let parent_id = parent.id;
        let (mut model, _) = model_with_task(parent);
        let done_child = Task::new("Done child")
            .with_parent(parent_id)
            .with_status(TaskStatus::Done);
        let open_child = Task::new("Open child").with_parent(parent_id);
        model.tasks.insert(done_child.id, done_child);
        model.tasks.insert(open_child.id, open_child);

        let content = render(&model);

        assert!(content.contains("── Subtasks (1/2) ──"));
        assert!(content.contains("Done child"));
        assert!(content.contains("Open child"));
        assert!(content.contains("Estimated: 1h"));
        assert!(content.contains("Actual: 1h 30m"));
        assert!(content.contains("Variance:"));
    }

    #[test]
    fn test_metadata_and_description_sections() {
        let project = crate::domain::Project::new("Detail project");
        let mut task = Task::new("Described");
        task.project_id = Some(project.id);
        task.tags = vec!["one".to_string(), "two".to_string()];
        task.description = Some("Line A\nLine B".to_string());
        let (mut model, _) = model_with_task(task);
        model.projects.insert(project.id, project);

        let content = render(&model);

        assert!(content.contains("Project: Detail project"));
        assert!(content.contains("Tags: #one #two"));
        assert!(content.contains("Line A"));
        assert!(content.contains("Line B"));
        assert!(content.contains("Estimated: (not set)"));
        assert!(content.contains("Actual: (none)"));
    }
}
