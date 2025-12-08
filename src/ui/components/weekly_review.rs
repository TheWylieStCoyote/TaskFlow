//! Weekly review mode component.
//!
//! Provides a GTD-style weekly review workflow:
//! 1. Review completed tasks from the past week
//! 2. Review and process overdue tasks
//! 3. Review upcoming week tasks
//! 4. Check projects for stalled work
//! 5. Weekly summary and stats

use chrono::{NaiveDate, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;

/// Phases of the weekly review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeeklyReviewPhase {
    #[default]
    Welcome,
    CompletedTasks,
    OverdueTasks,
    UpcomingWeek,
    StaleProjects,
    Summary,
}

impl WeeklyReviewPhase {
    /// Get the next phase in the review
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Welcome => Self::CompletedTasks,
            Self::CompletedTasks => Self::OverdueTasks,
            Self::OverdueTasks => Self::UpcomingWeek,
            Self::UpcomingWeek => Self::StaleProjects,
            Self::StaleProjects => Self::Summary,
            Self::Summary => Self::Summary, // Stay at end
        }
    }

    /// Get the previous phase
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Welcome => Self::Welcome, // Stay at start
            Self::CompletedTasks => Self::Welcome,
            Self::OverdueTasks => Self::CompletedTasks,
            Self::UpcomingWeek => Self::OverdueTasks,
            Self::StaleProjects => Self::UpcomingWeek,
            Self::Summary => Self::StaleProjects,
        }
    }

    /// Get phase number (1-6)
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Welcome => 1,
            Self::CompletedTasks => 2,
            Self::OverdueTasks => 3,
            Self::UpcomingWeek => 4,
            Self::StaleProjects => 5,
            Self::Summary => 6,
        }
    }

    /// Get phase title
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Weekly Review",
            Self::CompletedTasks => "Completed This Week",
            Self::OverdueTasks => "Overdue Tasks",
            Self::UpcomingWeek => "Next 7 Days",
            Self::StaleProjects => "Project Check",
            Self::Summary => "Weekly Summary",
        }
    }
}

/// Weekly review view widget
pub struct WeeklyReview<'a> {
    model: &'a Model,
    theme: &'a Theme,
    phase: WeeklyReviewPhase,
    selected: usize,
}

impl<'a> WeeklyReview<'a> {
    #[must_use]
    pub const fn new(
        model: &'a Model,
        theme: &'a Theme,
        phase: WeeklyReviewPhase,
        selected: usize,
    ) -> Self {
        Self {
            model,
            theme,
            phase,
            selected,
        }
    }

    fn today() -> NaiveDate {
        Utc::now().date_naive()
    }

    fn week_ago() -> NaiveDate {
        Self::today() - chrono::Duration::days(7)
    }

    fn week_ahead() -> NaiveDate {
        Self::today() + chrono::Duration::days(7)
    }

    /// Get tasks completed in the past week
    fn completed_this_week(&self) -> Vec<&Task> {
        let week_ago = Self::week_ago();
        self.model
            .tasks
            .values()
            .filter(|t| {
                t.status.is_complete() && t.completed_at.is_some_and(|d| d.date_naive() >= week_ago)
            })
            .collect()
    }

    /// Get overdue tasks
    fn overdue_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
            .collect()
    }

    /// Get tasks due in the next 7 days
    fn upcoming_week_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        let week_ahead = Self::week_ahead();
        self.model
            .tasks
            .values()
            .filter(|t| {
                !t.status.is_complete() && t.due_date.is_some_and(|d| d >= today && d <= week_ahead)
            })
            .collect()
    }

    /// Get projects with no recent activity (stale)
    fn stale_projects(&self) -> Vec<(&crate::domain::ProjectId, &crate::domain::Project, usize)> {
        let week_ago = Self::week_ago();

        self.model
            .projects
            .iter()
            .filter_map(|(id, project)| {
                // Count incomplete tasks in this project
                let task_count = self
                    .model
                    .tasks
                    .values()
                    .filter(|t| t.project_id.as_ref() == Some(id) && !t.status.is_complete())
                    .count();

                // Check if any task was modified in the past week
                let has_recent_activity = self.model.tasks.values().any(|t| {
                    t.project_id.as_ref() == Some(id) && t.updated_at.date_naive() >= week_ago
                });

                // Stale if has tasks but no recent activity
                if task_count > 0 && !has_recent_activity {
                    Some((id, project, task_count))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Widget for WeeklyReview<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let theme = self.theme;

        // Main container
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Weekly Review ({}/6) - {} ",
                self.phase.number(),
                self.phase.title()
            ))
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Layout: content area + progress/help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Content
                Constraint::Length(3), // Progress bar + help
            ])
            .split(inner);

        // Render phase-specific content
        match self.phase {
            WeeklyReviewPhase::Welcome => self.render_welcome(chunks[0], buf),
            WeeklyReviewPhase::CompletedTasks => {
                let tasks = self.completed_this_week();
                self.render_task_list(chunks[0], buf, &tasks, "No completed tasks this week", true)
            }
            WeeklyReviewPhase::OverdueTasks => {
                let tasks = self.overdue_tasks();
                self.render_task_list(chunks[0], buf, &tasks, "No overdue tasks! 🎉", false)
            }
            WeeklyReviewPhase::UpcomingWeek => {
                let tasks = self.upcoming_week_tasks();
                self.render_task_list(chunks[0], buf, &tasks, "No tasks due this week", false)
            }
            WeeklyReviewPhase::StaleProjects => self.render_stale_projects(chunks[0], buf),
            WeeklyReviewPhase::Summary => self.render_summary(chunks[0], buf),
        }

        // Render progress and help
        self.render_footer(chunks[1], buf);
    }
}

impl WeeklyReview<'_> {
    fn render_welcome(&self, area: Rect, buf: &mut Buffer) {
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

    fn render_task_list(
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

        let items: Vec<ListItem> = tasks
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

    fn render_stale_projects(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let stale = self.stale_projects();

        if stale.is_empty() {
            let msg = Paragraph::new("All projects have recent activity! 🎉")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.success.to_color()));
            msg.render(area, buf);
            return;
        }

        let items: Vec<ListItem> = stale
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

    fn render_summary(&self, area: Rect, buf: &mut Buffer) {
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

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_navigation() {
        let phase = WeeklyReviewPhase::Welcome;
        assert_eq!(phase.next(), WeeklyReviewPhase::CompletedTasks);
        assert_eq!(phase.prev(), WeeklyReviewPhase::Welcome);

        let phase = WeeklyReviewPhase::Summary;
        assert_eq!(phase.next(), WeeklyReviewPhase::Summary);
        assert_eq!(phase.prev(), WeeklyReviewPhase::StaleProjects);
    }

    #[test]
    fn test_phase_numbers() {
        assert_eq!(WeeklyReviewPhase::Welcome.number(), 1);
        assert_eq!(WeeklyReviewPhase::CompletedTasks.number(), 2);
        assert_eq!(WeeklyReviewPhase::OverdueTasks.number(), 3);
        assert_eq!(WeeklyReviewPhase::UpcomingWeek.number(), 4);
        assert_eq!(WeeklyReviewPhase::StaleProjects.number(), 5);
        assert_eq!(WeeklyReviewPhase::Summary.number(), 6);
    }

    #[test]
    fn test_weekly_review_renders() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Welcome, 0);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        review.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }
}
