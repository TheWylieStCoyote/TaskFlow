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
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

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
