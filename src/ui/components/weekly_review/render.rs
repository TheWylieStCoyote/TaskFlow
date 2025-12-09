//! Rendering methods for weekly review.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::domain::Task;

use super::{WeeklyReview, WeeklyReviewPhase};

impl WeeklyReview<'_> {
    pub(crate) fn render_welcome(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let today = Self::today();

        let completed_count = self.completed_this_week().len();
        let overdue_count = self.overdue_tasks().len();
        let upcoming_count = self.upcoming_week_tasks().len();
        let stale_count = self.stale_projects().len();
        let total_incomplete: usize = self
            .model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete())
            .count();

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("📅 Week of {}", today.format("%B %d, %Y")),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Let's do a GTD-style weekly review!",
                Style::default().fg(theme.colors.foreground.to_color()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  ✅ Completed: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    completed_count.to_string(),
                    Style::default().fg(theme.colors.success.to_color()),
                ),
                Span::styled(
                    " tasks this week",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  ⚠️  Overdue: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    overdue_count.to_string(),
                    if overdue_count > 0 {
                        Style::default().fg(theme.colors.danger.to_color())
                    } else {
                        Style::default().fg(theme.colors.success.to_color())
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  📆 Next week: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    upcoming_count.to_string(),
                    Style::default().fg(theme.colors.warning.to_color()),
                ),
                Span::styled(
                    " tasks due",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
            ]),
            if stale_count > 0 {
                Line::from(vec![
                    Span::styled(
                        "  🔕 Stale projects: ",
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        stale_count.to_string(),
                        Style::default().fg(theme.colors.warning.to_color()),
                    ),
                ])
            } else {
                Line::from("")
            },
            Line::from(vec![
                Span::styled(
                    "  📋 Total open: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    total_incomplete.to_string(),
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press → or Enter to begin review",
                Style::default().fg(theme.colors.muted.to_color()),
            )),
        ];

        let para = Paragraph::new(lines).alignment(Alignment::Center);
        para.render(area, buf);
    }

    pub(crate) fn render_task_list(
        &self,
        area: Rect,
        buf: &mut Buffer,
        tasks: &[&Task],
        empty_message: &str,
        show_completed_at: bool,
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
                let status = if task.status.is_complete() {
                    "✓ "
                } else {
                    "◇ "
                };

                let status_color = if task.status.is_complete() {
                    theme.colors.success.to_color()
                } else {
                    theme.colors.foreground.to_color()
                };

                let date_info = if show_completed_at {
                    task.completed_at
                        .map(|d| format!(" ({})", d.format("%b %d")))
                        .unwrap_or_default()
                } else {
                    task.due_date
                        .map(|d| {
                            let today = Self::today();
                            if d < today {
                                let days = (today - d).num_days();
                                format!(" ({days}d overdue)")
                            } else if d == today {
                                " (today)".to_string()
                            } else {
                                format!(" ({})", d.format("%b %d"))
                            }
                        })
                        .unwrap_or_default()
                };

                let date_color =
                    if !show_completed_at && task.due_date.is_some_and(|d| d < Self::today()) {
                        theme.colors.danger.to_color()
                    } else {
                        theme.colors.muted.to_color()
                    };

                ListItem::new(Line::from(vec![
                    Span::styled(status, Style::default().fg(status_color)),
                    Span::styled(
                        &task.title,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                    Span::styled(date_info, Style::default().fg(date_color)),
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

    pub(crate) fn render_stale_projects(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let stale = self.stale_projects();

        if stale.is_empty() {
            let msg = Paragraph::new("All projects have recent activity! 🎉")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.success.to_color()));
            msg.render(area, buf);
            return;
        }

        let items: Vec<ListItem<'_>> = stale
            .iter()
            .map(|(_, project, task_count)| {
                ListItem::new(Line::from(vec![
                    Span::styled("📁 ", Style::default()),
                    Span::styled(
                        &project.name,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                    Span::styled(
                        format!(" ({task_count} tasks, no activity this week)"),
                        Style::default().fg(theme.colors.muted.to_color()),
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
        if !stale.is_empty() {
            state.select(Some(self.selected.min(stale.len().saturating_sub(1))));
        }
        StatefulWidget::render(list, area, buf, &mut state);
    }

    pub(crate) fn render_summary(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let completed_count = self.completed_this_week().len();
        let overdue_count = self.overdue_tasks().len();
        let upcoming_count = self.upcoming_week_tasks().len();
        let stale_count = self.stale_projects().len();

        // Calculate productivity metrics
        let total_tasks = self.model.tasks.len();
        let incomplete_tasks: usize = self
            .model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete())
            .count();
        let completion_rate = if total_tasks > 0 {
            (completed_count as f64 / total_tasks as f64 * 100.0) as u8
        } else {
            0
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "📊 Weekly Review Complete!",
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "This Week's Highlights:",
                Style::default()
                    .fg(theme.colors.foreground.to_color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ✅ ", Style::default()),
                Span::styled(
                    format!("{completed_count} tasks completed"),
                    Style::default().fg(theme.colors.success.to_color()),
                ),
            ]),
            if overdue_count > 0 {
                Line::from(vec![
                    Span::styled("  ⚠️  ", Style::default()),
                    Span::styled(
                        format!("{overdue_count} tasks overdue - review and reschedule"),
                        Style::default().fg(theme.colors.danger.to_color()),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled("  ✅ ", Style::default()),
                    Span::styled(
                        "No overdue tasks!",
                        Style::default().fg(theme.colors.success.to_color()),
                    ),
                ])
            },
            Line::from(vec![
                Span::styled("  📅 ", Style::default()),
                Span::styled(
                    format!("{upcoming_count} tasks due next week"),
                    Style::default().fg(theme.colors.warning.to_color()),
                ),
            ]),
            if stale_count > 0 {
                Line::from(vec![
                    Span::styled("  🔕 ", Style::default()),
                    Span::styled(
                        format!("{stale_count} stale projects need attention"),
                        Style::default().fg(theme.colors.warning.to_color()),
                    ),
                ])
            } else {
                Line::from("")
            },
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  📈 Completion rate: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{completion_rate}%"),
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  📋 Open tasks: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    incomplete_tasks.to_string(),
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Esc to exit. Have a productive week!",
                Style::default().fg(theme.colors.muted.to_color()),
            )),
        ];

        let para = Paragraph::new(lines).alignment(Alignment::Center);
        para.render(area, buf);
    }

    pub(crate) fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Help text varies by phase
        let help = match self.phase {
            WeeklyReviewPhase::Welcome => "[→/Enter] Start  [Esc] Exit",
            WeeklyReviewPhase::Summary => "[←] Back  [Esc] Exit",
            _ => "[←] Back  [→] Next  [Esc] Exit",
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Progress bar visualization
        let progress_bar = format!(
            "{}{}",
            "●".repeat(self.phase.number() as usize),
            "○".repeat(6 - self.phase.number() as usize)
        );

        let progress = format!("Step {}/6: {}", self.phase.number(), self.phase.title());

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
}
