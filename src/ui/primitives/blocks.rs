//! Styled block builders for consistent panel appearance.
//!
//! These functions create pre-styled [`Block`] widgets that follow the application's
//! visual conventions. Use these instead of manually styling blocks to ensure
//! consistency across the UI.

use ratatui::{
    style::{Modifier, Style},
    widgets::{Block, Borders},
};

use crate::config::Theme;

/// Create a standard panel block with title and border.
///
/// Uses the theme's border color for a neutral appearance.
/// Suitable for most panels and containers.
///
/// # Arguments
///
/// * `title` - The panel title (displayed with surrounding spaces)
/// * `theme` - The current theme configuration
///
/// # Example
///
/// ```ignore
/// let block = panel_block("Tasks", theme);
/// let inner = block.inner(area);
/// block.render(area, buf);
/// ```
#[must_use]
pub fn panel_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(theme.colors.border.to_color()))
}

/// Create an accent-bordered block for focused/highlighted panels.
///
/// Uses the theme's accent color for the border and title,
/// making the panel stand out. Use for active/focused elements.
///
/// # Arguments
///
/// * `title` - The panel title (displayed with surrounding spaces)
/// * `theme` - The current theme configuration
#[must_use]
pub fn accent_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.colors.accent.to_color()))
}

/// Create a warning-styled block for alerts and caution dialogs.
///
/// Uses the theme's warning color (typically yellow/orange) for the
/// border and title. Use for non-critical alerts.
///
/// # Arguments
///
/// * `title` - The panel title (displayed with surrounding spaces)
/// * `theme` - The current theme configuration
#[must_use]
pub fn warning_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(theme.colors.warning.to_color())
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.colors.warning.to_color()))
}

/// Create a danger-styled block for errors and critical alerts.
///
/// Uses the theme's danger color (typically red) for the border and title.
/// Use for error dialogs, destructive action confirmations, etc.
///
/// # Arguments
///
/// * `title` - The panel title (displayed with surrounding spaces)
/// * `theme` - The current theme configuration
#[must_use]
pub fn danger_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(theme.colors.danger.to_color())
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.colors.danger.to_color()))
}

/// Create a success-styled block for confirmations.
///
/// Uses the theme's success color (typically green) for the border and title.
/// Use for success messages and positive confirmations.
///
/// # Arguments
///
/// * `title` - The panel title (displayed with surrounding spaces)
/// * `theme` - The current theme configuration
#[must_use]
pub fn success_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(theme.colors.success.to_color())
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.colors.success.to_color()))
}
