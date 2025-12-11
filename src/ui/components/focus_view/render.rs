//! Render methods for focus view.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::config::Theme;
use crate::domain::Task;

use super::FocusView;

impl FocusView<'_> {
    #[allow(clippy::unused_self)] // Keep self for API consistency
    pub(crate) fn render_task_title(
        &self,
        task: &Task,
        area: Rect,
        buf: &mut Buffer,
        theme: &Theme,
    ) {
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

    pub(crate) fn render_timer(&self, task: &Task, area: Rect, buf: &mut Buffer, theme: &Theme) {
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

    pub(crate) fn render_help(&self, area: Rect, buf: &mut Buffer, theme: &Theme, task: &Task) {
        let is_tracking = self.model.active_time_entry.is_some();
        let is_full_screen = self.model.pomodoro.full_screen;

        // Build help text with chain navigation hints if applicable
        let has_chain =
            task.next_task_id.is_some() || self.get_prev_task_in_chain(task.id).is_some();

        let mut help_parts = Vec::new();
        help_parts.push(if is_tracking { "[t] Stop" } else { "[t] Start" });
        help_parts.push("[x] Toggle");

        if has_chain {
            if self.get_prev_task_in_chain(task.id).is_some() {
                help_parts.push("[[] Prev");
            }
            if task.next_task_id.is_some() {
                help_parts.push("[]] Next");
            }
        }

        // Full-screen toggle
        help_parts.push(if is_full_screen {
            "[F] Windowed"
        } else {
            "[F] Full"
        });

        help_parts.push("[f/Esc] Exit");

        let help_text = help_parts.join("  ");

        let help_line = Line::from(Span::styled(
            help_text,
            Style::default().fg(theme.colors.muted.to_color()),
        ))
        .alignment(Alignment::Center);

        buf.set_line(area.x, area.y, &help_line, area.width);
    }

    /// Render chain navigation info
    pub(crate) fn render_chain_info(
        &self,
        task: &Task,
        area: Rect,
        buf: &mut Buffer,
        theme: &Theme,
    ) {
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
