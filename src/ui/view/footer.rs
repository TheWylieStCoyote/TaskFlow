//! Footer rendering for the view module.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{Model, ViewId};
use crate::config::Theme;

/// Renders the footer status bar
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

/// Returns view-specific navigation hints for the footer
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
