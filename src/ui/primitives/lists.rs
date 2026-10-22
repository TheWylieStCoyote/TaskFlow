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

/// First visible index for a list that keeps `selected` in view.
///
/// Call this at render time with the real viewport height so the result
/// tracks the terminal size instead of a guessed row count. The selection
/// is centred once the list overflows and the offset is clamped so the
/// last page is always full; a list that fits never scrolls.
///
/// # Arguments
///
/// * `selected` - Index of the selected item
/// * `total` - Number of items in the list
/// * `visible` - Rows available for items (`area.height`)
#[must_use]
pub fn scroll_offset(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let max_offset = total - visible;
    selected.saturating_sub(visible / 2).min(max_offset)
}

#[cfg(test)]
mod tests {
    use super::scroll_offset;

    #[test]
    fn scroll_offset_never_scrolls_a_list_that_fits() {
        assert_eq!(scroll_offset(0, 5, 10), 0);
        assert_eq!(scroll_offset(4, 5, 10), 0);
        assert_eq!(scroll_offset(9, 10, 10), 0);
    }

    #[test]
    fn scroll_offset_centres_selection_and_clamps() {
        assert_eq!(scroll_offset(0, 50, 10), 0);
        assert_eq!(scroll_offset(4, 50, 10), 0, "still in the first page");
        assert_eq!(scroll_offset(25, 50, 10), 20, "centred");
        assert_eq!(scroll_offset(49, 50, 10), 40, "clamped at the end");
        assert_eq!(
            scroll_offset(99, 50, 10),
            40,
            "out-of-range selection clamps"
        );
    }

    #[test]
    fn scroll_offset_handles_degenerate_viewports() {
        assert_eq!(
            scroll_offset(3, 10, 0),
            3,
            "zero rows: offset tracks selection"
        );
        assert_eq!(scroll_offset(3, 10, 1), 3, "one row: the selected row");
        assert_eq!(scroll_offset(0, 0, 0), 0);
    }
}
