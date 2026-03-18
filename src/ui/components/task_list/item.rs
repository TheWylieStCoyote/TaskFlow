//! Task and project header list item rendering.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

use crate::config::Theme;
use crate::domain::{Priority, Task, TaskStatus};

/// Context for rendering a task item
pub struct TaskItemContext<'a> {
    pub task: &'a Task,
    pub is_selected: bool,
    pub is_tracking: bool,
    pub time_spent: u32,
    pub nesting_depth: usize, // 0 for root, 1+ for subtasks
    pub is_multi_selected: bool,
    pub has_dependencies: bool,
    pub is_recurring: bool,
    pub has_chain: bool,                  // Task is linked to another task
    pub subtask_progress: (usize, usize), // (completed, total)
    pub theme: &'a Theme,
    pub git_branch: Option<&'a str>, // Linked git branch name
}

/// Render a project header as a list item
#[must_use]
pub fn project_header_to_list_item(
    name: &str,
    task_count: usize,
    theme: &Theme,
) -> ListItem<'static> {
    let header_style = Style::default()
        .fg(theme.colors.accent.to_color())
        .add_modifier(Modifier::BOLD);

    let count_style = Style::default().fg(theme.colors.muted.to_color());

    let line = Line::from(vec![
        Span::styled("── ", Style::default().fg(theme.colors.muted.to_color())),
        Span::styled(name.to_string(), header_style),
        Span::styled(format!(" ({task_count}) "), count_style),
        Span::styled("──", Style::default().fg(theme.colors.muted.to_color())),
    ]);

    ListItem::new(line)
}

/// Render a task as a list item with all indicators
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn task_to_list_item(ctx: &TaskItemContext<'_>) -> ListItem<'static> {
    let task = ctx.task;
    let theme = ctx.theme;

    // Multi-select indicator
    let select_span = if ctx.is_multi_selected {
        Span::styled("● ", Style::default().fg(theme.colors.accent.to_color()))
    } else {
        Span::raw("  ")
    };

    // Subtask indentation prefix - supports multi-level nesting
    let indent_span = if ctx.nesting_depth > 0 {
        // Add spaces for each level of nesting, then the branch character
        let spaces = "  ".repeat(ctx.nesting_depth.saturating_sub(1));
        let indent = format!("{spaces}└─ ");
        Span::styled(indent, Style::default().fg(theme.colors.muted.to_color()))
    } else {
        Span::raw("")
    };

    let status_style = match task.status {
        TaskStatus::Done => Style::default().fg(theme.status.done.to_color()),
        TaskStatus::InProgress => Style::default().fg(theme.status.in_progress.to_color()),
        TaskStatus::Blocked => Style::default().fg(theme.colors.danger.to_color()),
        TaskStatus::Cancelled => Style::default().fg(theme.status.cancelled.to_color()),
        TaskStatus::Todo => Style::default().fg(theme.status.pending.to_color()),
    };

    let priority_span = match task.priority {
        Priority::Urgent => Span::styled(
            "!!!! ",
            Style::default().fg(theme.priority.urgent.to_color()),
        ),
        Priority::High => {
            Span::styled("!!!  ", Style::default().fg(theme.priority.high.to_color()))
        }
        Priority::Medium => Span::styled(
            "!!   ",
            Style::default().fg(theme.priority.medium.to_color()),
        ),
        Priority::Low => Span::styled("!    ", Style::default().fg(theme.priority.low.to_color())),
        Priority::None => Span::raw("     "),
    };

    // Time tracking indicator
    let tracking_span = if ctx.is_tracking {
        Span::styled(
            "● ",
            Style::default()
                .fg(theme.colors.danger.to_color())
                .add_modifier(Modifier::SLOW_BLINK),
        )
    } else {
        Span::raw("  ")
    };

    let status_span = Span::styled(format!("{} ", task.status.symbol()), status_style);

    // Determine title style based on due date urgency
    let (title_style, urgency_prefix) = if task.status.is_complete() {
        (
            Style::default()
                .fg(theme.colors.muted.to_color())
                .add_modifier(Modifier::CROSSED_OUT),
            Span::raw(""),
        )
    } else if task.is_overdue() {
        // Overdue: red text, bold, slow blink, warning prefix
        (
            Style::default()
                .fg(theme.colors.danger.to_color())
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::SLOW_BLINK),
            Span::styled(
                "⚠ ",
                Style::default()
                    .fg(theme.colors.danger.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
        )
    } else if task.is_due_today() {
        // Due today: yellow text, bold, exclamation prefix
        (
            Style::default()
                .fg(theme.colors.warning.to_color())
                .add_modifier(Modifier::BOLD),
            Span::styled(
                "! ",
                Style::default()
                    .fg(theme.colors.warning.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
        )
    } else if ctx.is_selected {
        (Style::default().add_modifier(Modifier::BOLD), Span::raw(""))
    } else {
        (Style::default(), Span::raw(""))
    };

    let title_span = Span::styled(task.title.clone(), title_style);

    // Add due date if present
    let due_span = if let Some(due) = task.due_date {
        use std::cmp::Ordering;
        let today = chrono::Utc::now().date_naive();
        let style = match due.cmp(&today) {
            Ordering::Less => Style::default().fg(theme.colors.danger.to_color()), // Overdue
            Ordering::Equal => Style::default().fg(theme.colors.warning.to_color()), // Due today
            Ordering::Greater => Style::default().fg(theme.colors.muted.to_color()),
        };
        Span::styled(format!(" [{}]", due.format("%m/%d")), style)
    } else {
        Span::raw("")
    };

    // Add scheduled date if present (shows when task is planned to be worked on)
    let sched_span = if let Some(sched) = task.scheduled_date {
        Span::styled(
            format!(" 📅{}", sched.format("%m/%d")),
            Style::default().fg(theme.colors.accent.to_color()),
        )
    } else {
        Span::raw("")
    };

    // Time spent indicator
    let time_span = if ctx.time_spent > 0 {
        let hours = ctx.time_spent / 60;
        let mins = ctx.time_spent % 60;
        let time_str = if hours > 0 {
            format!(" ({hours}h {mins}m)")
        } else {
            format!(" ({mins}m)")
        };
        Span::styled(
            time_str,
            Style::default().fg(theme.colors.accent.to_color()),
        )
    } else {
        Span::raw("")
    };

    // Estimation variance indicator (only shown if task has estimate)
    let variance_span = if let Some(variance_text) = task.time_variance_display() {
        let variance = task.time_variance().unwrap_or(0);
        let style = if variance > 0 {
            // Over estimate - red
            Style::default().fg(theme.colors.danger.to_color())
        } else if variance < 0 {
            // Under estimate - green (success)
            Style::default().fg(theme.status.done.to_color())
        } else {
            // On target - accent
            Style::default().fg(theme.colors.accent.to_color())
        };
        Span::styled(format!(" [{variance_text}]"), style)
    } else {
        Span::raw("")
    };

    // Tags display
    let tags_span = if task.tags.is_empty() {
        Span::raw("")
    } else {
        let tags_str = task
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ");
        Span::styled(
            format!(" {tags_str}"),
            Style::default().fg(theme.colors.muted.to_color()),
        )
    };

    // Description indicator (shows if task has a note)
    let desc_span = if task.description.is_some() {
        Span::styled(" [+]", Style::default().fg(theme.colors.muted.to_color()))
    } else {
        Span::raw("")
    };

    // Dependency indicator (shows if task is blocked by other tasks)
    let dep_span = if ctx.has_dependencies {
        Span::styled(" [B]", Style::default().fg(theme.colors.warning.to_color()))
    } else {
        Span::raw("")
    };

    // Recurrence indicator
    let recur_span = if ctx.is_recurring {
        Span::styled(" ↻", Style::default().fg(theme.colors.accent.to_color()))
    } else {
        Span::raw("")
    };

    // Chain indicator (→) - shows task is linked to next task in sequence
    let chain_span = if ctx.has_chain {
        Span::styled(" →", Style::default().fg(theme.colors.accent.to_color()))
    } else {
        Span::raw("")
    };

    // Subtask progress indicator - shows a compact bar and count if task has subtasks
    let progress_span = if ctx.subtask_progress.1 > 0 {
        let (completed, total) = ctx.subtask_progress;
        const BAR_WIDTH: usize = 6;
        let filled = (completed * BAR_WIDTH) / total;
        let bar = format!(
            "{}{}",
            "\u{2588}".repeat(filled),
            "\u{2591}".repeat(BAR_WIDTH - filled)
        );
        let style = if completed == total {
            // All done - show in success/done color
            Style::default().fg(theme.status.done.to_color())
        } else {
            Style::default().fg(theme.colors.muted.to_color())
        };
        Span::styled(format!(" [{bar}] {completed}/{total}"), style)
    } else {
        Span::raw("")
    };

    // Git branch indicator - shows linked branch name
    let git_span = if let Some(branch) = ctx.git_branch {
        // Truncate long branch names for display
        let display_branch = if branch.len() > 20 {
            format!("{}...", &branch[..17])
        } else {
            branch.to_string()
        };
        Span::styled(
            format!(" ⎇ {display_branch}"),
            Style::default().fg(theme.colors.accent.to_color()),
        )
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        select_span,
        indent_span,
        tracking_span,
        priority_span,
        status_span,
        urgency_prefix,
        title_span,
        progress_span,
        desc_span,
        dep_span,
        recur_span,
        chain_span,
        git_span,
        due_span,
        sched_span,
        time_span,
        variance_span,
        tags_span,
    ]);

    ListItem::new(line)
}
