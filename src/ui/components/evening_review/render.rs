//! Render methods for evening review phases.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::domain::Task;

use super::{EveningReview, EveningReviewPhase};

impl EveningReview<'_> {
    /// Render the welcome phase with day summary.
    pub(crate) fn render_welcome(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let today = Self::today();

        let completed_count = self.completed_today().len();
        let incomplete_count = self.all_incomplete_today().len();
        let total_time = self.total_time_today();
        let completion_rate = self.today_completion_rate();

        // Determine greeting based on completion rate
        let (greeting, greeting_style) = if completion_rate >= 80.0 {
            (
                "Great day! You crushed it!",
                Style::default().fg(theme.colors.success.to_color()),
            )
        } else if completion_rate >= 50.0 {
            (
                "Good progress today!",
                Style::default().fg(theme.colors.accent.to_color()),
            )
        } else {
            (
                "Let's wrap up your day.",
                Style::default().fg(theme.colors.foreground.to_color()),
            )
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{} {}", today.format("%A, %B %d, %Y"), ""),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(greeting, greeting_style)),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Completed: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    completed_count.to_string(),
                    Style::default()
                        .fg(theme.colors.success.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" tasks", Style::default().fg(theme.colors.muted.to_color())),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Remaining: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    incomplete_count.to_string(),
                    if incomplete_count > 0 {
                        Style::default().fg(theme.colors.warning.to_color())
                    } else {
                        Style::default().fg(theme.colors.success.to_color())
                    },
                ),
                Span::styled(" tasks", Style::default().fg(theme.colors.muted.to_color())),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Time tracked: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format_duration(total_time),
                    Style::default().fg(theme.colors.accent.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Completion: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{completion_rate:.0}%"),
                    if completion_rate >= 80.0 {
                        Style::default().fg(theme.colors.success.to_color())
                    } else if completion_rate >= 50.0 {
                        Style::default().fg(theme.colors.accent.to_color())
                    } else {
                        Style::default().fg(theme.colors.warning.to_color())
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press [Enter] or [->] to review your day",
                Style::default().fg(theme.colors.muted.to_color()),
            )),
        ];

        let para = Paragraph::new(lines).alignment(Alignment::Center);
        para.render(area, buf);
    }

    /// Render the completed today phase.
    pub(crate) fn render_completed_today(&self, area: Rect, buf: &mut Buffer) {
        let tasks = self.completed_today();
        self.render_task_list_completed(area, buf, &tasks, "No tasks completed today");
    }

    /// Render the incomplete tasks phase with action hints.
    pub(crate) fn render_incomplete_tasks(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let tasks = self.all_incomplete_today();

        if tasks.is_empty() {
            let msg = Paragraph::new("All tasks completed! Nothing left to handle.")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.success.to_color()));
            msg.render(area, buf);
            return;
        }

        // Split area for list and action hints
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(area);

        // Render task list with selection
        self.render_task_list_selectable(chunks[0], buf, &tasks);

        // Render action hints
        let hints = Line::from(vec![
            Span::styled(
                "[r] ",
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Reschedule  ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                "[s] ",
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Snooze  ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::styled(
                "[x] ",
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Complete",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
        ])
        .alignment(Alignment::Center);

        buf.set_line(chunks[1].x, chunks[1].y + 1, &hints, chunks[1].width);
    }

    /// Render tomorrow preview phase.
    pub(crate) fn render_tomorrow_preview(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let tasks = self.tomorrow_tasks();

        if tasks.is_empty() {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No tasks scheduled for tomorrow!",
                    Style::default().fg(theme.colors.success.to_color()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Enjoy your free schedule.",
                    Style::default().fg(theme.colors.muted.to_color()),
                )),
            ];
            let para = Paragraph::new(lines).alignment(Alignment::Center);
            para.render(area, buf);
            return;
        }

        // Header
        let header_line = Line::from(vec![
            Span::styled(
                format!("{} ", tasks.len()),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "task(s) for tomorrow:",
                Style::default().fg(theme.colors.foreground.to_color()),
            ),
        ])
        .alignment(Alignment::Center);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(area);

        buf.set_line(chunks[0].x, chunks[0].y, &header_line, chunks[0].width);

        // Task list (not selectable, just preview)
        self.render_task_list_preview(chunks[1], buf, &tasks);
    }

    /// Render time review phase.
    pub(crate) fn render_time_review(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let total_time = self.total_time_today();
        let entries = self.time_entries_today();

        if entries.is_empty() {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No time tracked today",
                    Style::default().fg(theme.colors.muted.to_color()),
                )),
            ];
            let para = Paragraph::new(lines).alignment(Alignment::Center);
            para.render(area, buf);
            return;
        }

        // Calculate time by project (task's project)
        let mut by_project: std::collections::HashMap<Option<crate::domain::ProjectId>, u32> =
            std::collections::HashMap::new();

        for entry in &entries {
            let project_id = self
                .model
                .tasks
                .get(&entry.task_id)
                .and_then(|t| t.project_id);
            *by_project.entry(project_id).or_default() += entry.duration_minutes.unwrap_or(0);
        }

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Total time: ",
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
                Span::styled(
                    format_duration(total_time),
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "By Project:",
                Style::default()
                    .fg(theme.colors.foreground.to_color())
                    .add_modifier(Modifier::UNDERLINED),
            )),
        ];

        // Sort projects by time (descending)
        let mut sorted: Vec<_> = by_project.into_iter().collect();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.1));

        for (project_id, minutes) in sorted.iter().take(8) {
            let project_name = project_id
                .and_then(|pid| self.model.projects.get(&pid))
                .map_or("(No Project)", |p| p.name.as_str());

            let percentage = (f64::from(*minutes) / f64::from(total_time)) * 100.0;

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {project_name:<20}"),
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
                Span::styled(
                    format!("{:>8}", format_duration(*minutes)),
                    Style::default().fg(theme.colors.accent.to_color()),
                ),
                Span::styled(
                    format!("  ({percentage:>5.1}%)"),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
            ]));
        }

        let para = Paragraph::new(lines).alignment(Alignment::Center);
        para.render(area, buf);
    }

    /// Render the summary phase.
    pub(crate) fn render_summary(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let completed_count = self.completed_today().len();
        let incomplete_count = self.all_incomplete_today().len();
        let total_time = self.total_time_today();
        let completion_rate = self.today_completion_rate();

        // Determine encouraging message
        let message = if completion_rate >= 90.0 {
            "Outstanding work today!"
        } else if completion_rate >= 75.0 {
            "Great job today!"
        } else if completion_rate >= 50.0 {
            "Good progress! Tomorrow is a new day."
        } else {
            "Every day is a fresh start. Rest well!"
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Day Complete!",
                Style::default()
                    .fg(theme.colors.success.to_color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Tasks completed: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    completed_count.to_string(),
                    Style::default()
                        .fg(theme.colors.success.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Remaining: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    incomplete_count.to_string(),
                    if incomplete_count > 0 {
                        Style::default().fg(theme.colors.warning.to_color())
                    } else {
                        Style::default().fg(theme.colors.success.to_color())
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Time tracked: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format_duration(total_time),
                    Style::default().fg(theme.colors.accent.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Completion rate: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{completion_rate:.0}%"),
                    if completion_rate >= 80.0 {
                        Style::default().fg(theme.colors.success.to_color())
                    } else if completion_rate >= 50.0 {
                        Style::default().fg(theme.colors.accent.to_color())
                    } else {
                        Style::default().fg(theme.colors.warning.to_color())
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                message,
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "See you tomorrow!",
                Style::default().fg(theme.colors.muted.to_color()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press [Esc] to close",
                Style::default().fg(theme.colors.muted.to_color()),
            )),
        ];

        let para = Paragraph::new(lines).alignment(Alignment::Center);
        para.render(area, buf);
    }

    /// Render the footer with progress and help.
    pub(crate) fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Help text varies by phase
        let help = match self.phase {
            EveningReviewPhase::Welcome => "[->] Start  [Esc] Exit",
            EveningReviewPhase::Summary => "[<-] Back  [Esc] Exit",
            EveningReviewPhase::IncompleteTasks => {
                "[<-] Back  [->] Next  [r]eschedule  [s]nooze  [x] Done  [Esc] Exit"
            }
            _ => "[<-] Back  [->] Next  [Esc] Exit",
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Progress bar visualization
        let current = self.phase.number() as usize;
        let total = EveningReviewPhase::TOTAL_PHASES as usize;
        let progress_bar = format!(
            "{}{}",
            "●".repeat(current),
            "○".repeat(total.saturating_sub(current))
        );

        let progress = format!(
            "Step {}/{}: {}",
            self.phase.number(),
            EveningReviewPhase::TOTAL_PHASES,
            self.phase.title()
        );

        let progress_line = Line::from(vec![
            Span::styled(
                &progress_bar,
                Style::default().fg(theme.colors.accent.to_color()),
            ),
            Span::raw("  "),
            Span::styled(progress, Style::default().fg(theme.colors.muted.to_color())),
        ])
        .alignment(Alignment::Center);

        buf.set_line(chunks[0].x, chunks[0].y, &progress_line, chunks[0].width);

        let help_line = Line::from(Span::styled(
            help,
            Style::default().fg(theme.colors.muted.to_color()),
        ))
        .alignment(Alignment::Center);

        buf.set_line(chunks[1].x, chunks[1].y, &help_line, chunks[1].width);
    }

    // ========== Helper render methods ==========

    /// Render a list of completed tasks (with checkmarks).
    fn render_task_list_completed(
        &self,
        area: Rect,
        buf: &mut Buffer,
        tasks: &[&Task],
        empty_message: &str,
    ) {
        let theme = self.theme;

        if tasks.is_empty() {
            let msg = Paragraph::new(empty_message)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.muted.to_color()));
            msg.render(area, buf);
            return;
        }

        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .map(|task| {
                let time_info = if task.actual_minutes > 0 {
                    format!(" ({})", format_duration(task.actual_minutes))
                } else {
                    String::new()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        "  [x] ",
                        Style::default().fg(theme.colors.success.to_color()),
                    ),
                    Span::styled(
                        &task.title,
                        Style::default()
                            .fg(theme.colors.foreground.to_color())
                            .add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        time_info,
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items);
        Widget::render(list, area, buf);
    }

    /// Render a selectable task list (for incomplete tasks).
    fn render_task_list_selectable(&self, area: Rect, buf: &mut Buffer, tasks: &[&Task]) {
        let theme = self.theme;

        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .map(|task| {
                let status_icon = if task.due_date.is_some_and(|d| d == Self::today()) {
                    "!" // Due today
                } else {
                    "~" // Scheduled today
                };

                let due_info = task
                    .due_date
                    .map(|d| {
                        if d == Self::today() {
                            " (due)".to_string()
                        } else {
                            format!(" (due {})", d.format("%b %d"))
                        }
                    })
                    .unwrap_or_default();

                let scheduled_info = if task.scheduled_date == Some(Self::today())
                    && task.due_date != Some(Self::today())
                {
                    " (scheduled)"
                } else {
                    ""
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {status_icon} "),
                        Style::default().fg(theme.colors.warning.to_color()),
                    ),
                    Span::styled(
                        &task.title,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                    Span::styled(
                        due_info,
                        Style::default().fg(theme.colors.danger.to_color()),
                    ),
                    Span::styled(
                        scheduled_info,
                        Style::default().fg(theme.colors.accent.to_color()),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.colors.accent.to_color())
                    .fg(theme.colors.background.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        if !tasks.is_empty() {
            state.select(Some(self.selected.min(tasks.len().saturating_sub(1))));
        }
        StatefulWidget::render(list, area, buf, &mut state);
    }

    /// Render a preview task list (not selectable).
    fn render_task_list_preview(&self, area: Rect, buf: &mut Buffer, tasks: &[&Task]) {
        let theme = self.theme;

        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .map(|task| {
                let tomorrow = Self::tomorrow();
                let is_due = task.due_date == Some(tomorrow);
                let is_scheduled = task.scheduled_date == Some(tomorrow);

                let indicator = if is_due && is_scheduled {
                    "!~"
                } else if is_due {
                    "! "
                } else {
                    "~ "
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {indicator} "),
                        Style::default().fg(if is_due {
                            theme.colors.warning.to_color()
                        } else {
                            theme.colors.accent.to_color()
                        }),
                    ),
                    Span::styled(
                        &task.title,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items);
        Widget::render(list, area, buf);
    }
}

/// Format duration in minutes to human-readable string.
fn format_duration(minutes: u32) -> String {
    if minutes == 0 {
        return "0m".to_string();
    }

    let hours = minutes / 60;
    let mins = minutes % 60;

    if hours > 0 && mins > 0 {
        format!("{hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{mins}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;
    use crate::domain::{today, Project, Task, TaskStatus, TimeEntry};
    use crate::ui::test_utils::{buffer_content, render_widget};
    use chrono::{Datelike, Duration, Local, TimeZone, Utc};

    fn render_phase(model: &Model, phase: EveningReviewPhase, selected: usize) -> String {
        let theme = Theme::default();
        let widget = EveningReview::with_state(model, &theme, phase, selected);
        buffer_content(&render_widget(widget, 90, 30))
    }

    /// A finished entry that started at local noon today, so that it counts as
    /// "today" regardless of the wall clock when the test runs.
    fn entry_today(task_id: crate::domain::TaskId, minutes: u32) -> TimeEntry {
        let date = today();
        let start = Local
            .with_ymd_and_hms(date.year(), date.month(), date.day(), 12, 0, 0)
            .single()
            .expect("valid local time")
            .with_timezone(&Utc);
        let mut entry = TimeEntry::start(task_id);
        entry.started_at = start;
        entry.ended_at = Some(start + Duration::minutes(i64::from(minutes)));
        entry.duration_minutes = Some(minutes);
        entry
    }

    #[test]
    fn test_format_duration_variants() {
        assert_eq!(format_duration(0), "0m");
        assert_eq!(format_duration(25), "25m");
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(120), "2h");
        assert_eq!(format_duration(95), "1h 35m");
    }

    #[test]
    fn test_time_review_without_entries_shows_placeholder() {
        let model = Model::new();
        let content = render_phase(&model, EveningReviewPhase::TimeReview, 0);
        assert!(content.contains("No time tracked today"));
        assert!(!content.contains("By Project"));
    }

    #[test]
    fn test_time_review_groups_time_by_project() {
        let mut model = Model::new();
        let project = Project::new("Client work");
        let project_id = project.id;
        model.projects.insert(project_id, project);

        let mut in_project = Task::new("Billable");
        in_project.project_id = Some(project_id);
        let no_project = Task::new("Admin");
        let in_project_id = in_project.id;
        let no_project_id = no_project.id;
        model.tasks.insert(in_project_id, in_project);
        model.tasks.insert(no_project_id, no_project);

        for entry in [
            entry_today(in_project_id, 60),
            entry_today(in_project_id, 30),
            entry_today(no_project_id, 30),
        ] {
            model.time_entries.insert(entry.id, entry);
        }

        let content = render_phase(&model, EveningReviewPhase::TimeReview, 0);

        assert!(content.contains("Total time: 2h"));
        assert!(content.contains("By Project:"));
        assert!(content.contains("Client work"));
        assert!(content.contains("1h 30m"));
        assert!(content.contains("( 75.0%)"));
        assert!(content.contains("(No Project)"));
        assert!(content.contains("( 25.0%)"));
    }

    #[test]
    fn test_time_review_entry_for_deleted_task_counts_as_no_project() {
        let mut model = Model::new();
        let orphan = entry_today(crate::domain::TaskId::new(), 45);
        model.time_entries.insert(orphan.id, orphan);

        let content = render_phase(&model, EveningReviewPhase::TimeReview, 0);

        assert!(content.contains("Total time: 45m"));
        assert!(content.contains("(No Project)"));
        assert!(content.contains("(100.0%)"));
    }

    #[test]
    fn test_time_review_shows_at_most_eight_projects() {
        let mut model = Model::new();
        for i in 0..10 {
            let project = Project::new(format!("Project {i:02}"));
            let mut task = Task::new(format!("Task {i}"));
            task.project_id = Some(project.id);
            let entry = entry_today(task.id, 10 * (i + 1));
            model.projects.insert(project.id, project);
            model.tasks.insert(task.id, task);
            model.time_entries.insert(entry.id, entry);
        }

        let content = render_phase(&model, EveningReviewPhase::TimeReview, 0);

        // Largest first; the two smallest (Project 00 and 01) are cut off
        assert!(content.contains("Project 09"));
        assert!(content.contains("Project 02"));
        assert!(!content.contains("Project 01"));
        assert!(!content.contains("Project 00"));
    }

    #[test]
    fn test_incomplete_tasks_list_marks_due_scheduled_and_other_dates() {
        let mut model = Model::new();
        let mut due_today = Task::new("Due today task");
        due_today.due_date = Some(today());
        let mut scheduled = Task::new("Scheduled task");
        scheduled.scheduled_date = Some(today());
        let mut scheduled_due_later = Task::new("Scheduled due later");
        scheduled_due_later.scheduled_date = Some(today());
        scheduled_due_later.due_date = Some(today() + Duration::days(4));
        model.tasks.insert(due_today.id, due_today);
        model.tasks.insert(scheduled.id, scheduled);
        model
            .tasks
            .insert(scheduled_due_later.id, scheduled_due_later);

        let content = render_phase(&model, EveningReviewPhase::IncompleteTasks, 1);

        assert!(content.contains("! Due today task (due)"));
        assert!(content.contains("~ Scheduled task (scheduled)"));
        let later = (today() + Duration::days(4)).format("%b %d").to_string();
        assert!(content.contains(&format!("~ Scheduled due later (due {later}) (scheduled)")));
        assert!(content.contains("> "), "selection marker rendered");
        assert!(content.contains("[r] Reschedule"));
        assert!(content.contains("[s] Snooze"));
        assert!(content.contains("[x] Complete"));
    }

    #[test]
    fn test_incomplete_tasks_selection_clamps_to_last_item() {
        let mut model = Model::new();
        let mut due_today = Task::new("Only one");
        due_today.due_date = Some(today());
        model.tasks.insert(due_today.id, due_today);

        let content = render_phase(&model, EveningReviewPhase::IncompleteTasks, 9);
        assert!(content.contains("> "));
        assert!(content.contains("Only one"));
    }

    #[test]
    fn test_incomplete_tasks_phase_when_everything_is_done() {
        let model = Model::new();
        let content = render_phase(&model, EveningReviewPhase::IncompleteTasks, 0);
        assert!(content.contains("All tasks completed! Nothing left to handle."));
    }

    #[test]
    fn test_completed_today_lists_tasks_with_time() {
        let mut model = Model::new();
        let mut done = Task::new("Finished with time");
        done.status = TaskStatus::Done;
        done.completed_at = Some(Utc::now());
        done.actual_minutes = 75;
        let mut quick = Task::new("Finished quickly");
        quick.status = TaskStatus::Done;
        quick.completed_at = Some(Utc::now());
        model.tasks.insert(done.id, done);
        model.tasks.insert(quick.id, quick);

        let content = render_phase(&model, EveningReviewPhase::CompletedToday, 0);

        assert!(content.contains("[x] Finished with time (1h 15m)"));
        assert!(content.contains("[x] Finished quickly"));
    }

    #[test]
    fn test_completed_today_empty_message() {
        let model = Model::new();
        let content = render_phase(&model, EveningReviewPhase::CompletedToday, 0);
        assert!(content.contains("No tasks completed today"));
    }

    #[test]
    fn test_tomorrow_preview_indicators() {
        let mut model = Model::new();
        let tomorrow = today() + Duration::days(1);
        let mut due = Task::new("Due tomorrow");
        due.due_date = Some(tomorrow);
        let mut scheduled = Task::new("Scheduled tomorrow");
        scheduled.scheduled_date = Some(tomorrow);
        let mut both = Task::new("Both tomorrow");
        both.due_date = Some(tomorrow);
        both.scheduled_date = Some(tomorrow);
        model.tasks.insert(due.id, due);
        model.tasks.insert(scheduled.id, scheduled);
        model.tasks.insert(both.id, both);

        let content = render_phase(&model, EveningReviewPhase::TomorrowPreview, 0);

        assert!(content.contains("3 task(s) for tomorrow:"));
        assert!(content.contains("!  Due tomorrow"));
        assert!(content.contains("~  Scheduled tomorrow"));
        assert!(content.contains("!~ Both tomorrow"));
    }

    #[test]
    fn test_tomorrow_preview_empty_message() {
        let model = Model::new();
        let content = render_phase(&model, EveningReviewPhase::TomorrowPreview, 0);
        assert!(content.contains("No tasks scheduled for tomorrow!"));
        assert!(content.contains("Enjoy your free schedule."));
    }

    #[test]
    fn test_welcome_and_summary_reflect_completion_rate() {
        let mut model = Model::new();
        let mut done = Task::new("Done");
        done.status = TaskStatus::Done;
        done.completed_at = Some(Utc::now());
        model.tasks.insert(done.id, done);

        let welcome = render_phase(&model, EveningReviewPhase::Welcome, 0);
        assert!(welcome.contains("Great day! You crushed it!"));
        assert!(welcome.contains("Completed: 1 tasks"));
        assert!(welcome.contains("Completion: 100%"));

        let summary = render_phase(&model, EveningReviewPhase::Summary, 0);
        assert!(summary.contains("Day Complete!"));
        assert!(summary.contains("Outstanding work today!"));
        assert!(summary.contains("Press [Esc] to close"));

        // Add an open task to drop the rate to 50%
        let mut open = Task::new("Open");
        open.due_date = Some(today());
        model.tasks.insert(open.id, open);
        let welcome = render_phase(&model, EveningReviewPhase::Welcome, 0);
        assert!(welcome.contains("Good progress today!"));
        let summary = render_phase(&model, EveningReviewPhase::Summary, 0);
        assert!(summary.contains("Good progress! Tomorrow is a new day."));

        // Two more open tasks: 25%
        for i in 0..2 {
            let mut open = Task::new(format!("Open {i}"));
            open.due_date = Some(today());
            model.tasks.insert(open.id, open);
        }
        let welcome = render_phase(&model, EveningReviewPhase::Welcome, 0);
        assert!(welcome.contains("Let's wrap up your day."));
        let summary = render_phase(&model, EveningReviewPhase::Summary, 0);
        assert!(summary.contains("Every day is a fresh start. Rest well!"));
    }

    #[test]
    fn test_footer_help_varies_by_phase() {
        let model = Model::new();
        let welcome = render_phase(&model, EveningReviewPhase::Welcome, 0);
        assert!(welcome.contains("[->] Start  [Esc] Exit"));
        assert!(welcome.contains("Step 1/6"));

        let incomplete = render_phase(&model, EveningReviewPhase::IncompleteTasks, 0);
        assert!(incomplete.contains("[r]eschedule  [s]nooze  [x] Done"));

        let summary = render_phase(&model, EveningReviewPhase::Summary, 0);
        assert!(summary.contains("[<-] Back  [Esc] Exit"));
        assert!(summary.contains("Step 6/6"));
    }
}
