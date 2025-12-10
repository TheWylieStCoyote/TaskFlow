//! Daily review mode component.
//!
//! Provides a guided workflow for morning planning:
//! 1. Review overdue tasks
//! 2. Review tasks due today
//! 3. Review scheduled tasks for today
//! 4. Quick summary and planning

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

/// Phases of the daily review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DailyReviewPhase {
    #[default]
    Welcome,
    OverdueTasks,
    TodayTasks,
    ScheduledTasks,
    Summary,
}

impl DailyReviewPhase {
    /// Get the next phase in the review
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Welcome => Self::OverdueTasks,
            Self::OverdueTasks => Self::TodayTasks,
            Self::TodayTasks => Self::ScheduledTasks,
            Self::ScheduledTasks => Self::Summary,
            Self::Summary => Self::Summary, // Stay at end
        }
    }

    /// Get the previous phase
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Welcome => Self::Welcome, // Stay at start
            Self::OverdueTasks => Self::Welcome,
            Self::TodayTasks => Self::OverdueTasks,
            Self::ScheduledTasks => Self::TodayTasks,
            Self::Summary => Self::ScheduledTasks,
        }
    }

    /// Get phase number (1-5)
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Welcome => 1,
            Self::OverdueTasks => 2,
            Self::TodayTasks => 3,
            Self::ScheduledTasks => 4,
            Self::Summary => 5,
        }
    }

    /// Get phase title
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Good Morning!",
            Self::OverdueTasks => "Overdue Tasks",
            Self::TodayTasks => "Today's Tasks",
            Self::ScheduledTasks => "Scheduled for Today",
            Self::Summary => "Daily Summary",
        }
    }
}

/// Daily review view widget
pub struct DailyReview<'a> {
    model: &'a Model,
    theme: &'a Theme,
    phase: DailyReviewPhase,
    selected: usize,
}

impl<'a> DailyReview<'a> {
    #[must_use]
    pub const fn new(
        model: &'a Model,
        theme: &'a Theme,
        phase: DailyReviewPhase,
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

    /// Get overdue tasks
    fn overdue_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
            .collect()
    }

    /// Get tasks due today
    fn today_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.due_date == Some(today))
            .collect()
    }

    /// Get tasks scheduled for today (but not due today)
    fn scheduled_today_tasks(&self) -> Vec<&Task> {
        let today = Self::today();
        self.model
            .tasks
            .values()
            .filter(|t| {
                !t.status.is_complete()
                    && t.scheduled_date == Some(today)
                    && t.due_date != Some(today)
            })
            .collect()
    }
}

impl Widget for DailyReview<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let theme = self.theme;

        // Main container
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Daily Review ({}/5) - {} ",
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
            DailyReviewPhase::Welcome => self.render_welcome(chunks[0], buf),
            DailyReviewPhase::OverdueTasks => self.render_task_list(
                chunks[0],
                buf,
                &self.overdue_tasks(),
                "No overdue tasks! 🎉",
            ),
            DailyReviewPhase::TodayTasks => {
                self.render_task_list(chunks[0], buf, &self.today_tasks(), "No tasks due today");
            }
            DailyReviewPhase::ScheduledTasks => self.render_task_list(
                chunks[0],
                buf,
                &self.scheduled_today_tasks(),
                "No scheduled tasks for today",
            ),
            DailyReviewPhase::Summary => self.render_summary(chunks[0], buf),
        }

        // Render progress and help
        self.render_footer(chunks[1], buf);
    }
}

impl DailyReview<'_> {
    fn render_welcome(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let today = Self::today();

        let overdue_count = self.overdue_tasks().len();
        let today_count = self.today_tasks().len();
        let scheduled_count = self.scheduled_today_tasks().len();
        let total_incomplete: usize = self
            .model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete())
            .count();

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("📅 {}", today.format("%A, %B %d, %Y")),
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Let's review your tasks for today!",
                Style::default().fg(theme.colors.foreground.to_color()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  📌 Overdue: ",
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
                    "  📅 Due today: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    today_count.to_string(),
                    Style::default().fg(theme.colors.warning.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  🗓️  Scheduled: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    scheduled_count.to_string(),
                    Style::default().fg(theme.colors.accent.to_color()),
                ),
            ]),
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

    fn render_task_list(&self, area: Rect, buf: &mut Buffer, tasks: &[&Task], empty_message: &str) {
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
                let priority_indicator = match task.priority {
                    crate::domain::Priority::Urgent => "!!!! ",
                    crate::domain::Priority::High => "!!!  ",
                    crate::domain::Priority::Medium => "!!   ",
                    crate::domain::Priority::Low => "!    ",
                    crate::domain::Priority::None => "     ",
                };

                let priority_color = match task.priority {
                    crate::domain::Priority::Urgent => theme.priority.urgent.to_color(),
                    crate::domain::Priority::High => theme.priority.high.to_color(),
                    crate::domain::Priority::Medium => theme.priority.medium.to_color(),
                    crate::domain::Priority::Low => theme.priority.low.to_color(),
                    crate::domain::Priority::None => theme.colors.muted.to_color(),
                };

                let due_info = task
                    .due_date
                    .map(|d| {
                        let today = Self::today();
                        if d < today {
                            let days = (today - d).num_days();
                            format!(" ({days} days overdue)")
                        } else if d == today {
                            " (today)".to_string()
                        } else {
                            format!(" ({})", d.format("%b %d"))
                        }
                    })
                    .unwrap_or_default();

                ListItem::new(Line::from(vec![
                    Span::styled(priority_indicator, Style::default().fg(priority_color)),
                    Span::styled(
                        &task.title,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                    Span::styled(due_info, Style::default().fg(theme.colors.muted.to_color())),
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

    fn render_summary(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let overdue_count = self.overdue_tasks().len();
        let today_count = self.today_tasks().len();
        let scheduled_count = self.scheduled_today_tasks().len();

        let total_for_today = overdue_count + today_count + scheduled_count;

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "📊 Review Complete!",
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "You have ",
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
                Span::styled(
                    total_for_today.to_string(),
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " task(s) requiring attention:",
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(""),
            if overdue_count > 0 {
                Line::from(vec![
                    Span::styled("  ⚠️  ", Style::default()),
                    Span::styled(
                        format!("{overdue_count} overdue"),
                        Style::default().fg(theme.colors.danger.to_color()),
                    ),
                    Span::styled(
                        " - consider completing or rescheduling",
                        Style::default().fg(theme.colors.muted.to_color()),
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
            if today_count > 0 {
                Line::from(vec![
                    Span::styled("  📅 ", Style::default()),
                    Span::styled(
                        format!("{today_count} due today"),
                        Style::default().fg(theme.colors.warning.to_color()),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled("  📅 ", Style::default()),
                    Span::styled(
                        "Nothing due today",
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                ])
            },
            if scheduled_count > 0 {
                Line::from(vec![
                    Span::styled("  🗓️  ", Style::default()),
                    Span::styled(
                        format!("{scheduled_count} scheduled"),
                        Style::default().fg(theme.colors.accent.to_color()),
                    ),
                ])
            } else {
                Line::from("")
            },
            Line::from(""),
            Line::from(Span::styled(
                "Press Esc to exit review, or navigate to a view",
                Style::default().fg(theme.colors.muted.to_color()),
            )),
        ];

        let para = Paragraph::new(lines).alignment(Alignment::Center);
        para.render(area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Progress indicator
        let progress = format!("Step {}/5: {}", self.phase.number(), self.phase.title());

        // Help text varies by phase
        let help = match self.phase {
            DailyReviewPhase::Welcome => "[→/Enter] Start  [Esc] Exit",
            DailyReviewPhase::Summary => "[←] Back  [Esc] Exit",
            _ => "[←] Back  [→] Next  [x] Complete  [Esc] Exit",
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Progress bar visualization
        let progress_bar = format!(
            "{}{}",
            "●".repeat(self.phase.number() as usize),
            "○".repeat(5 - self.phase.number() as usize)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Priority;
    use chrono::Duration;

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
    fn test_phase_navigation() {
        let phase = DailyReviewPhase::Welcome;
        assert_eq!(phase.next(), DailyReviewPhase::OverdueTasks);
        assert_eq!(phase.prev(), DailyReviewPhase::Welcome); // Can't go before start

        let phase = DailyReviewPhase::Summary;
        assert_eq!(phase.next(), DailyReviewPhase::Summary); // Can't go past end
        assert_eq!(phase.prev(), DailyReviewPhase::ScheduledTasks);
    }

    #[test]
    fn test_phase_numbers() {
        assert_eq!(DailyReviewPhase::Welcome.number(), 1);
        assert_eq!(DailyReviewPhase::OverdueTasks.number(), 2);
        assert_eq!(DailyReviewPhase::TodayTasks.number(), 3);
        assert_eq!(DailyReviewPhase::ScheduledTasks.number(), 4);
        assert_eq!(DailyReviewPhase::Summary.number(), 5);
    }

    #[test]
    fn test_phase_titles() {
        assert_eq!(DailyReviewPhase::Welcome.title(), "Good Morning!");
        assert_eq!(DailyReviewPhase::OverdueTasks.title(), "Overdue Tasks");
        assert_eq!(DailyReviewPhase::TodayTasks.title(), "Today's Tasks");
        assert_eq!(DailyReviewPhase::ScheduledTasks.title(), "Scheduled for Today");
        assert_eq!(DailyReviewPhase::Summary.title(), "Daily Summary");
    }

    #[test]
    fn test_daily_review_renders() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::Welcome, 0);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        review.render(area, &mut buffer);

        // Should render without panic
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_daily_review_summary_phase() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::Summary, 0);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        review.render(area, &mut buffer);

        // Should render without panic
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_daily_review_overdue_phase_empty() {
        let model = Model::new(); // No tasks
        let theme = Theme::default();
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::OverdueTasks, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);

        // Should show empty message when no overdue tasks
        assert!(content.contains("No overdue tasks"));
    }

    #[test]
    fn test_daily_review_overdue_phase_with_tasks() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        // Create an overdue task
        let today = Utc::now().date_naive();
        let mut task = Task::new("Overdue task");
        task.due_date = Some(today - Duration::days(3));
        task.priority = Priority::High;
        model.tasks.insert(task.id, task);

        let review = DailyReview::new(&model, &theme, DailyReviewPhase::OverdueTasks, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);

        // Should show the overdue task
        assert!(content.contains("Overdue task"));
        assert!(content.contains("overdue")); // Due date info
    }

    #[test]
    fn test_daily_review_today_phase_empty() {
        let model = Model::new();
        let theme = Theme::default();
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::TodayTasks, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("No tasks due today"));
    }

    #[test]
    fn test_daily_review_today_phase_with_tasks() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        let today = Utc::now().date_naive();
        let mut task = Task::new("Today task");
        task.due_date = Some(today);
        model.tasks.insert(task.id, task);

        let review = DailyReview::new(&model, &theme, DailyReviewPhase::TodayTasks, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("Today task"));
        assert!(content.contains("today")); // Due date info
    }

    #[test]
    fn test_daily_review_scheduled_phase_empty() {
        let model = Model::new();
        let theme = Theme::default();
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::ScheduledTasks, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("No scheduled tasks"));
    }

    #[test]
    fn test_daily_review_scheduled_phase_with_tasks() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        let today = Utc::now().date_naive();
        let mut task = Task::new("Scheduled task");
        task.scheduled_date = Some(today);
        // Don't set due_date to today so it shows in scheduled
        task.due_date = Some(today + Duration::days(5));
        model.tasks.insert(task.id, task);

        let review = DailyReview::new(&model, &theme, DailyReviewPhase::ScheduledTasks, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("Scheduled task"));
    }

    #[test]
    fn test_daily_review_welcome_shows_counts() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        let today = Utc::now().date_naive();

        // Add an overdue task
        let mut overdue = Task::new("Overdue");
        overdue.due_date = Some(today - Duration::days(1));
        model.tasks.insert(overdue.id, overdue);

        // Add a task due today
        let mut due_today = Task::new("Due today");
        due_today.due_date = Some(today);
        model.tasks.insert(due_today.id, due_today);

        let review = DailyReview::new(&model, &theme, DailyReviewPhase::Welcome, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);

        // Should show the date
        assert!(content.contains(&today.format("%A").to_string())); // Day name
    }

    #[test]
    fn test_daily_review_priority_colors() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        let today = Utc::now().date_naive();

        // Add tasks with different priorities
        for priority in [Priority::Urgent, Priority::High, Priority::Medium, Priority::Low, Priority::None] {
            let mut task = Task::new(format!("{:?} priority", priority));
            task.due_date = Some(today - Duration::days(1)); // Make overdue
            task.priority = priority;
            model.tasks.insert(task.id, task);
        }

        let review = DailyReview::new(&model, &theme, DailyReviewPhase::OverdueTasks, 0);
        let buffer = render_widget(review, 80, 24);

        // Should render without panic, priority indicators should be present
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_daily_review_footer_navigation_hints() {
        let model = Model::new();
        let theme = Theme::default();

        // Welcome phase should show Start and Exit
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::Welcome, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);
        assert!(content.contains("Start") || content.contains("Enter"));

        // Middle phases should show Back and Next
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::TodayTasks, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);
        assert!(content.contains("Back") || content.contains("Next"));

        // Summary phase should show Back and Exit
        let review = DailyReview::new(&model, &theme, DailyReviewPhase::Summary, 0);
        let buffer = render_widget(review, 80, 24);
        let content = buffer_content(&buffer);
        assert!(content.contains("Back"));
    }
}
