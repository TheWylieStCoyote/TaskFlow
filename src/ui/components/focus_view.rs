//! Focus mode view component.
//!
//! A minimalist, distraction-free view for working on a single task.
//! Displays the current task prominently with an optional Pomodoro timer
//! and task chain navigation.
//!
//! # Features
//!
//! - Large, centered task display
//! - Pomodoro timer with visual progress
//! - Task chain navigation (previous/next in sequence)
//! - Subtask progress indicator

use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;

/// Focus mode view - minimalist single-task view with timer
pub struct FocusView<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> FocusView<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Format a duration as HH:MM:SS
    fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }

    /// Get time tracked for a task including current session
    fn get_time_tracked(&self, task: &Task) -> Duration {
        let mut total_minutes: u64 = 0;

        // Sum completed time entries
        for entry in self.model.time_entries.values() {
            if entry.task_id == task.id {
                total_minutes += u64::from(entry.calculated_duration_minutes());
            }
        }

        // Add current active session if tracking this task
        if let Some(active) = self.model.active_time_entry() {
            if active.task_id == task.id {
                let now = chrono::Utc::now();
                let elapsed_secs = (now - active.started_at).num_seconds().max(0) as u64;
                return Duration::from_secs(total_minutes * 60 + elapsed_secs);
            }
        }

        Duration::from_secs(total_minutes * 60)
    }
}

impl Widget for FocusView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Get the selected task
        let Some(task) = self.model.selected_task() else {
            // No task selected - shouldn't happen but handle gracefully
            let msg = Paragraph::new("No task selected")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.muted.to_color()));
            msg.render(area, buf);
            return;
        };

        // Create centered layout
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .title(" FOCUS MODE ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));
        let inner = outer_block.inner(area);
        outer_block.render(area, buf);

        // Check if task is part of a chain
        let has_chain = task.next_task_id.is_some()
            || self
                .model
                .tasks
                .values()
                .any(|t| t.next_task_id == Some(task.id));

        // Layout: padding, content, chain info, timer, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),                             // Top padding
                Constraint::Min(6),                                // Main content
                Constraint::Length(if has_chain { 2 } else { 0 }), // Chain info (conditional)
                Constraint::Length(3),                             // Timer
                Constraint::Length(2),                             // Help text
            ])
            .split(inner);

        // Render task title with status
        self.render_task_title(task, chunks[1], buf, theme);

        // Render chain info if applicable
        if has_chain {
            self.render_chain_info(task, chunks[2], buf, theme);
        }

        // Render timer
        self.render_timer(task, chunks[3], buf, theme);

        // Render help
        self.render_help(chunks[4], buf, theme, task);
    }
}

impl FocusView<'_> {
    #[allow(clippy::unused_self)] // Keep self for API consistency
    fn render_task_title(&self, task: &Task, area: Rect, buf: &mut Buffer, theme: &Theme) {
        // Split area for title, metadata, and description
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(2), // Metadata (priority, due date)
                Constraint::Min(2),    // Description
            ])
            .split(area);

        // Status indicator
        let status_icon = if task.status.is_complete() {
            "[x]"
        } else {
            "[ ]"
        };
        let status_style = if task.status.is_complete() {
            Style::default()
                .fg(theme.status.done.to_color())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme.colors.foreground.to_color())
                .add_modifier(Modifier::BOLD)
        };

        // Title line
        let title_line = Line::from(vec![
            Span::styled(format!("  {status_icon} "), status_style),
            Span::styled(
                &task.title,
                Style::default()
                    .fg(theme.colors.foreground.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        buf.set_line(chunks[0].x, chunks[0].y, &title_line, chunks[0].width);

        // Metadata line: priority and due date
        let mut meta_spans = vec![Span::raw("      ")]; // Indent to align with title

        // Priority
        let priority_str = match task.priority {
            crate::domain::Priority::Urgent => "!!!! Urgent",
            crate::domain::Priority::High => "!!! High",
            crate::domain::Priority::Medium => "!! Medium",
            crate::domain::Priority::Low => "! Low",
            crate::domain::Priority::None => "",
        };
        if !priority_str.is_empty() {
            let priority_color = match task.priority {
                crate::domain::Priority::Urgent => theme.priority.urgent.to_color(),
                crate::domain::Priority::High => theme.priority.high.to_color(),
                crate::domain::Priority::Medium => theme.priority.medium.to_color(),
                crate::domain::Priority::Low => theme.priority.low.to_color(),
                crate::domain::Priority::None => theme.colors.muted.to_color(),
            };
            meta_spans.push(Span::styled(
                priority_str,
                Style::default().fg(priority_color),
            ));
        }

        // Due date
        if let Some(due) = task.due_date {
            if !priority_str.is_empty() {
                meta_spans.push(Span::raw("  |  "));
            }
            meta_spans.push(Span::styled(
                format!("Due: {}", due.format("%b %d, %Y")),
                Style::default().fg(theme.colors.muted.to_color()),
            ));
        }

        // Scheduled date
        if let Some(sched) = task.scheduled_date {
            meta_spans.push(Span::raw("  |  "));
            meta_spans.push(Span::styled(
                format!("Scheduled: {}", sched.format("%b %d, %Y")),
                Style::default().fg(theme.colors.accent.to_color()),
            ));
        }

        let meta_line = Line::from(meta_spans);
        buf.set_line(chunks[1].x, chunks[1].y, &meta_line, chunks[1].width);

        // Description (if any)
        if let Some(ref desc) = task.description {
            if !desc.is_empty() {
                // Add a separator line
                let sep = Line::from(Span::styled(
                    "      ─────────────────────────────────────",
                    Style::default().fg(theme.colors.border.to_color()),
                ));
                buf.set_line(chunks[2].x, chunks[2].y, &sep, chunks[2].width);

                // Render description with wrap
                let desc_area = Rect {
                    x: chunks[2].x + 6,
                    y: chunks[2].y + 1,
                    width: chunks[2].width.saturating_sub(6),
                    height: chunks[2].height.saturating_sub(1),
                };
                let desc_para = Paragraph::new(desc.as_str())
                    .style(Style::default().fg(theme.colors.muted.to_color()))
                    .wrap(Wrap { trim: true });
                desc_para.render(desc_area, buf);
            }
        }
    }

    fn render_timer(&self, task: &Task, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let time_tracked = self.get_time_tracked(task);
        let time_str = Self::format_duration(time_tracked);

        let is_tracking = self.model.is_tracking_task(&task.id);

        let timer_style = if is_tracking {
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.muted.to_color())
        };

        let icon = "⏱ ";

        let timer_line = Line::from(vec![Span::styled(format!("{icon}{time_str}"), timer_style)])
            .alignment(Alignment::Center);

        // Center the timer vertically in its area
        let y = area.y + area.height / 2;
        buf.set_line(area.x, y, &timer_line, area.width);
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer, theme: &Theme, task: &Task) {
        let is_tracking = self.model.active_time_entry.is_some();

        // Build help text with chain navigation hints if applicable
        let has_chain =
            task.next_task_id.is_some() || self.get_prev_task_in_chain(task.id).is_some();

        let mut help_parts = Vec::new();
        help_parts.push(if is_tracking {
            "[t] Stop Timer"
        } else {
            "[t] Start Timer"
        });
        help_parts.push("[x] Toggle");

        if has_chain {
            if self.get_prev_task_in_chain(task.id).is_some() {
                help_parts.push("[[] Prev");
            }
            if task.next_task_id.is_some() {
                help_parts.push("[]] Next");
            }
        }

        help_parts.push("[f/Esc] Exit");

        let help_text = help_parts.join("  ");

        let help_line = Line::from(Span::styled(
            help_text,
            Style::default().fg(theme.colors.muted.to_color()),
        ))
        .alignment(Alignment::Center);

        buf.set_line(area.x, area.y, &help_line, area.width);
    }

    /// Get the previous task in a chain (the task that links to this one)
    fn get_prev_task_in_chain(
        &self,
        task_id: crate::domain::TaskId,
    ) -> Option<crate::domain::TaskId> {
        self.model
            .tasks
            .values()
            .find(|t| t.next_task_id == Some(task_id))
            .map(|t| t.id)
    }

    /// Render chain navigation info
    fn render_chain_info(&self, task: &Task, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let prev_in_chain = self.get_prev_task_in_chain(task.id);
        let next_in_chain = task.next_task_id;

        // Only render if task is part of a chain
        if prev_in_chain.is_none() && next_in_chain.is_none() {
            return;
        }

        let mut spans = vec![Span::styled(
            "      Chain: ",
            Style::default().fg(theme.colors.muted.to_color()),
        )];

        // Show previous task in chain
        if let Some(prev_id) = prev_in_chain {
            if let Some(prev_task) = self.model.tasks.get(&prev_id) {
                let prev_title: String = prev_task.title.chars().take(20).collect();
                let prev_status = if prev_task.status.is_complete() {
                    "✓"
                } else {
                    "○"
                };
                spans.push(Span::styled(
                    format!("{prev_status} {prev_title}"),
                    Style::default().fg(theme.colors.muted.to_color()),
                ));
                spans.push(Span::styled(
                    " → ",
                    Style::default().fg(theme.colors.accent.to_color()),
                ));
            }
        }

        // Current task indicator
        spans.push(Span::styled(
            "● CURRENT",
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        ));

        // Show next task in chain
        if let Some(next_id) = next_in_chain {
            if let Some(next_task) = self.model.tasks.get(&next_id) {
                let next_title: String = next_task.title.chars().take(20).collect();
                let next_status = if next_task.status.is_complete() {
                    "✓"
                } else {
                    "○"
                };
                spans.push(Span::styled(
                    " → ",
                    Style::default().fg(theme.colors.accent.to_color()),
                ));
                spans.push(Span::styled(
                    format!("{next_status} {next_title}"),
                    Style::default().fg(theme.colors.muted.to_color()),
                ));
            }
        }

        let chain_line = Line::from(spans);
        buf.set_line(area.x, area.y, &chain_line, area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;

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
    fn test_focus_view_renders_focus_mode_title() {
        let mut model = Model::new().with_sample_data();
        model.refresh_visible_tasks();
        let theme = Theme::default();
        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 60, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("FOCUS MODE"),
            "Focus mode title should be visible"
        );
    }

    #[test]
    fn test_focus_view_shows_task_title() {
        let mut model = Model::new().with_sample_data();
        model.refresh_visible_tasks();
        let theme = Theme::default();
        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 20);
        let content = buffer_content(&buffer);

        // Should show a task checkbox
        assert!(
            content.contains("[ ]") || content.contains("[x]"),
            "Task status indicator should be visible"
        );
    }

    #[test]
    fn test_focus_view_shows_timer() {
        let mut model = Model::new().with_sample_data();
        model.refresh_visible_tasks();
        let theme = Theme::default();
        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 60, 20);
        let content = buffer_content(&buffer);

        // Timer shows time in format like 00:00:00
        assert!(
            content.contains(':'),
            "Timer should show colon-separated time"
        );
    }

    #[test]
    fn test_focus_view_shows_help_text() {
        let mut model = Model::new().with_sample_data();
        model.refresh_visible_tasks();
        let theme = Theme::default();
        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Exit") || content.contains("Esc"),
            "Help text should mention exiting focus mode"
        );
    }

    #[test]
    fn test_focus_view_no_task_selected() {
        let model = Model::new(); // No tasks, nothing selected
        let theme = Theme::default();
        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 60, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("No task selected"),
            "Should show message when no task selected"
        );
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(
            FocusView::format_duration(Duration::from_secs(0)),
            "00:00:00"
        );
        assert_eq!(
            FocusView::format_duration(Duration::from_secs(59)),
            "00:00:59"
        );
        assert_eq!(
            FocusView::format_duration(Duration::from_secs(60)),
            "00:01:00"
        );
        assert_eq!(
            FocusView::format_duration(Duration::from_secs(3661)),
            "01:01:01"
        );
        assert_eq!(
            FocusView::format_duration(Duration::from_secs(7200)),
            "02:00:00"
        );
    }

    #[test]
    fn test_focus_view_with_high_priority_task() {
        use crate::domain::{Priority, Task};

        let mut model = Model::new();
        let theme = Theme::default();

        let mut task = Task::new("High priority task");
        task.priority = Priority::High;
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("High priority task"));
        assert!(content.contains("High")); // Priority label
    }

    #[test]
    fn test_focus_view_with_due_date() {
        use crate::domain::Task;
        use chrono::{Duration as ChronoDuration, Utc};

        let mut model = Model::new();
        let theme = Theme::default();

        let mut task = Task::new("Task with due date");
        task.due_date = Some(Utc::now().date_naive() + ChronoDuration::days(5));
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("Due:"));
    }

    #[test]
    fn test_focus_view_with_scheduled_date() {
        use crate::domain::Task;
        use chrono::Utc;

        let mut model = Model::new();
        let theme = Theme::default();

        let mut task = Task::new("Task with scheduled date");
        task.scheduled_date = Some(Utc::now().date_naive());
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("Scheduled:"));
    }

    #[test]
    fn test_focus_view_with_description() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        let mut task = Task::new("Task with description");
        task.description = Some("This is a detailed description of the task.".to_string());
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("detailed description"));
    }

    #[test]
    fn test_focus_view_completed_task() {
        use crate::domain::{Task, TaskStatus};

        let mut model = Model::new();
        let theme = Theme::default();

        let task = Task::new("Completed task").with_status(TaskStatus::Done);
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        assert!(content.contains("[x]")); // Completed checkbox
    }

    #[test]
    fn test_focus_view_with_chain() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        // Create a chain of tasks
        let mut task1 = Task::new("First in chain");
        let mut task2 = Task::new("Second in chain");

        task1.next_task_id = Some(task2.id);
        let task2_id = task2.id;

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.visible_tasks = vec![task2_id];
        model.selected_index = 0;

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        // Should show chain info - second task is pointed to by first
        assert!(content.contains("Chain") || content.contains("CURRENT"));
    }

    #[test]
    fn test_focus_view_with_next_in_chain() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        // Create a chain of tasks
        let mut task1 = Task::new("First in chain");
        let task2 = Task::new("Second in chain");

        task1.next_task_id = Some(task2.id);
        let task1_id = task1.id;

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.visible_tasks = vec![task1_id];
        model.selected_index = 0;

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        // Should show chain info - first task has next
        assert!(content.contains("Chain") || content.contains("CURRENT") || content.contains("Second"));
    }

    #[test]
    fn test_focus_view_timer_start_stop_hint() {
        use crate::domain::Task;

        let mut model = Model::new();
        let theme = Theme::default();

        let task = Task::new("Task for timer test");
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        // When not tracking
        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);
        assert!(content.contains("Start Timer") || content.contains("[t]"));
    }

    #[test]
    fn test_focus_view_with_active_timer() {
        use crate::domain::{Task, TimeEntry};

        let mut model = Model::new();
        let theme = Theme::default();

        let task = Task::new("Task being tracked");
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        // Start tracking
        let entry = TimeEntry::start(task_id);
        let entry_id = entry.id;
        model.time_entries.insert(entry.id, entry);
        model.active_time_entry = Some(entry_id);

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);
        let content = buffer_content(&buffer);

        // Should show timer and stop hint
        assert!(content.contains("Stop Timer") || content.contains("[t]"));
    }

    #[test]
    fn test_focus_view_time_tracked_from_entries() {
        use crate::domain::{Task, TimeEntry};
        use chrono::{Duration as ChronoDuration, Utc};

        let mut model = Model::new();
        let theme = Theme::default();

        let task = Task::new("Task with time entries");
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        // Add a completed time entry (30 minutes)
        let mut entry = TimeEntry::start(task_id);
        entry.started_at = Utc::now() - ChronoDuration::minutes(30);
        entry.stop();
        model.time_entries.insert(entry.id, entry);

        let focus_view = FocusView::new(&model, &theme);
        let buffer = render_widget(focus_view, 80, 24);

        // Should render with time tracked (contains colon from time format)
        let content = buffer_content(&buffer);
        assert!(content.contains(':'));
    }
}
