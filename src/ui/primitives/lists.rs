//! Styled list configurations for consistent list appearance.
//!
//! These functions configure [`List`] widgets with standardized highlight styles.
//! Use these to ensure consistent selection highlighting across the application.

use ratatui::{
    style::{Modifier, Style},
    widgets::List,
};

use crate::config::Theme;

/// Configure standard highlight styling for a list.
///
/// Uses the theme's secondary accent color as background with bold text.
/// This is the default highlight style for most selectable lists.
///
/// # Arguments
///
/// * `list` - The list to configure
/// * `theme` - The current theme configuration
///
/// # Example
///
/// ```ignore
/// let items = vec![ListItem::new("Item 1"), ListItem::new("Item 2")];
/// let list = with_highlight_style(List::new(items), theme);
/// ```
#[must_use]
pub fn with_highlight_style<'a>(list: List<'a>, theme: &Theme) -> List<'a> {
    list.highlight_style(
        Style::default()
            .bg(theme.colors.accent_secondary.to_color())
            .fg(theme.colors.foreground.to_color())
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ")
}

/// Configure accent highlight styling for a list.
///
/// Uses the theme's primary accent color as background.
/// Use for emphasis or focused panels where you want stronger visual feedback.
///
/// # Arguments
///
/// * `list` - The list to configure
/// * `theme` - The current theme configuration
#[must_use]
pub fn with_accent_highlight<'a>(list: List<'a>, theme: &Theme) -> List<'a> {
    list.highlight_style(
        Style::default()
            .bg(theme.colors.accent.to_color())
            .fg(theme.colors.background.to_color())
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ")
}

/// Configure muted highlight styling for a list.
///
/// Uses the theme's muted color for subtle highlighting.
/// Use for secondary lists or when you want less visual emphasis.
///
/// # Arguments
///
/// * `list` - The list to configure
/// * `theme` - The current theme configuration
#[must_use]
pub fn with_muted_highlight<'a>(list: List<'a>, theme: &Theme) -> List<'a> {
    list.highlight_style(
        Style::default()
            .bg(theme.colors.muted.to_color())
            .fg(theme.colors.foreground.to_color()),
    )
    .highlight_symbol("> ")
}
