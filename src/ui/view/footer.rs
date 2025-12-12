//! Footer rendering for the view module.
//!
//! The footer displays contextual status information at the bottom of the screen.
//! It adapts its content based on application state, showing the most relevant
//! information at any given time.
//!
//! # Display Priority
//!
//! The footer shows content in this priority order (highest first):
//!
//! 1. **Error message** (red) - Validation errors, operation failures
//! 2. **Status message** (accent) - Success notifications, confirmations
//! 3. **Macro recording** (red) - "[REC] Recording macro..." indicator
//! 4. **Normal footer** - Task stats, indicators, and hints
//!
//! # Normal Footer Components
//!
//! When no alerts are active, the footer displays (left to right):
//!
//! ```text
//! N tasks (M completed) | [MULTI-SELECT: X] | K overdue | L due today | 🍅 MM:SS [C/G] | hint | mode | ? help
//! ```
//!
//! | Component | Condition | Color |
//! |-----------|-----------|-------|
//! | Task count | Always | Muted |
//! | Multi-select | `model.multi_select.mode` | Accent, bold |
//! | Overdue count | > 0 | Danger, bold |
//! | Due today count | > 0 | Warning, bold |
//! | Pomodoro timer | Active session | Accent (work) / Success (break) |
//! | View hint | View-specific | Accent |
//! | Show mode | Always | Muted |
//! | Help | Always | Muted |
//!
//! # View-Specific Hints
//!
//! Each view can provide navigation hints via [`get_view_hint`]:
//!
//! - **Kanban**: "h/l: columns | j/k: tasks"
//! - **Eisenhower**: "h/l/j/k: quadrants"
//! - **Calendar**: "h/l: months | Enter: day tasks"
//! - **Focus mode**: "[/]: chain | t: timer | f: exit"

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{Model, ViewId};
use crate::config::Theme;

/// Renders the footer status bar.
///
/// The footer occupies a single row at the bottom of the screen and displays
/// contextual information based on application state. See module documentation
/// for the complete display priority order and component breakdown.
///
/// # Arguments
///
/// * `model` - Application state containing alerts, stats, and view info
/// * `frame` - Ratatui frame for rendering
/// * `area` - The footer rectangle (typically 1 row tall)
/// * `theme` - Color theme for styling
pub(super) fn render_footer(model: &Model, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    // Show error message if available (in red, higher priority than status)
    if let Some(ref msg) = model.alerts.error_message {
        let footer =
            Paragraph::new(msg.clone()).style(Style::default().fg(theme.colors.danger.to_color()));
        frame.render_widget(footer, area);
        return;
    }

    // Show status message if available, otherwise show normal footer
    if let Some(ref msg) = model.alerts.status_message {
        let footer =
            Paragraph::new(msg.clone()).style(Style::default().fg(theme.colors.accent.to_color()));
        frame.render_widget(footer, area);
        return;
    }

    if model.macro_state.is_recording() {
        let footer = Paragraph::new(" [REC] Recording macro... Press Ctrl+Q then 0-9 to save ")
            .style(Style::default().fg(theme.colors.danger.to_color()));
        frame.render_widget(footer, area);
        return;
    }

    // Use cached counts for performance
    let task_count = model.visible_tasks.len();
    let completed = model.footer_stats.completed_count;
    let overdue = model.footer_stats.overdue_count;
    let due_today = model.footer_stats.due_today_count;

    // Build footer with styled spans
    let mut spans = vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            format!("{task_count} tasks"),
            Style::default().fg(theme.colors.muted.to_color()),
        ),
        Span::styled(
            format!(" ({completed} completed)"),
            Style::default().fg(theme.colors.muted.to_color()),
        ),
    ];

    // Add multi-select mode indicator
    if model.multi_select.mode {
        let selected_count = model.multi_select.selected.len();
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            format!("[MULTI-SELECT: {selected_count}]"),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add overdue indicator (red)
    if overdue > 0 {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            format!("{overdue} overdue"),
            Style::default()
                .fg(theme.colors.danger.to_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add due today indicator (yellow)
    if due_today > 0 {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            format!("{due_today} due today"),
            Style::default()
                .fg(theme.colors.warning.to_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add Pomodoro timer display if active
    if let Some(ref session) = model.pomodoro.session {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));

        // Show phase icon and timer
        let phase_icon = session.phase.icon();
        let time_display = session.formatted_remaining();
        let pause_indicator = if session.paused { " ⏸" } else { "" };

        // Color based on phase: work = accent, break = success
        let timer_color = if session.phase.is_break() {
            theme.colors.success.to_color()
        } else {
            theme.colors.accent.to_color()
        };

        spans.push(Span::styled(
            format!(
                "{} {} [{}/{}]{}",
                phase_icon,
                time_display,
                session.cycles_completed,
                session.session_goal,
                pause_indicator
            ),
            Style::default()
                .fg(timer_color)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add view-specific navigation hints
    if let Some(hint) = get_view_hint(model) {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            hint,
            Style::default().fg(theme.colors.accent.to_color()),
        ));
    }

    // Add show mode and help
    spans.push(Span::styled(
        " | ",
        Style::default().fg(theme.colors.muted.to_color()),
    ));
    spans.push(Span::styled(
        if model.filtering.show_completed {
            "showing all"
        } else {
            "hiding completed"
        },
        Style::default().fg(theme.colors.muted.to_color()),
    ));
    spans.push(Span::styled(
        " | ? help",
        Style::default().fg(theme.colors.muted.to_color()),
    ));

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}

/// Returns view-specific navigation hints for the footer.
///
/// Each view can provide contextual hints to help users navigate.
/// Returns `None` for views that use default controls or are view-only
/// (like Heatmap, Forecast, Burndown).
///
/// # Focus Mode
///
/// When `model.focus_mode` is enabled, returns focus-specific hints
/// regardless of the underlying view.
pub(super) fn get_view_hint(model: &Model) -> Option<&'static str> {
    // Focus mode has its own hints
    if model.focus_mode {
        return Some("[/]: chain | t: timer | f: exit");
    }

    match model.current_view {
        ViewId::Kanban => Some("h/l: columns | j/k: tasks"),
        ViewId::Eisenhower => Some("h/l/j/k: quadrants"),
        ViewId::WeeklyPlanner => Some("h/l: days | j/k: tasks"),
        ViewId::Timeline => Some("h/l: scroll | </>: zoom | t: today"),
        ViewId::Network => Some("h/l/j/k: navigate"),
        ViewId::Habits => Some("n: new | Space: check-in"),
        ViewId::Goals => Some("n: new goal | N: new KR | Enter: expand"),
        ViewId::Calendar => Some("h/l: months | Enter: day tasks"),
        ViewId::Duplicates => Some("j/k: navigate | D: dismiss | M: merge"),
        ViewId::Heatmap | ViewId::Forecast | ViewId::Burndown => None, // View-only
        _ => None, // Task list and others use default controls
    }
}
