//! Modal/dialog wrappers for popup content.
//!
//! These widgets wrap content in styled modal containers that:
//! - Clear the background area
//! - Render a styled border/title
//! - Render the inner content
//!
//! Use these for consistent modal dialog appearance.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Clear, Widget},
};

use crate::config::Theme;

use super::blocks::{accent_block, danger_block, panel_block, success_block, warning_block};

/// A modal wrapper with standard panel styling.
///
/// Clears the background and renders content within a neutral-bordered panel.
pub struct PanelModal<'a, W: Widget> {
    title: &'a str,
    content: W,
    theme: &'a Theme,
}

impl<'a, W: Widget> PanelModal<'a, W> {
    /// Create a new panel modal.
    ///
    /// # Arguments
    ///
    /// * `title` - The modal title
    /// * `content` - The inner widget to render
    /// * `theme` - The current theme configuration
    #[must_use]
    pub const fn new(title: &'a str, content: W, theme: &'a Theme) -> Self {
        Self {
            title,
            content,
            theme,
        }
    }
}

impl<W: Widget> Widget for PanelModal<'_, W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = panel_block(self.title, self.theme);
        let inner = block.inner(area);
        block.render(area, buf);
        self.content.render(inner, buf);
    }
}

/// A modal wrapper with accent styling.
///
/// Clears the background and renders content within an accent-colored border.
/// Use for important dialogs or focused content.
pub struct AccentModal<'a, W: Widget> {
    title: &'a str,
    content: W,
    theme: &'a Theme,
}

impl<'a, W: Widget> AccentModal<'a, W> {
    /// Create a new accent modal.
    #[must_use]
    pub const fn new(title: &'a str, content: W, theme: &'a Theme) -> Self {
        Self {
            title,
            content,
            theme,
        }
    }
}

impl<W: Widget> Widget for AccentModal<'_, W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = accent_block(self.title, self.theme);
        let inner = block.inner(area);
        block.render(area, buf);
        self.content.render(inner, buf);
    }
}

/// A modal wrapper with warning styling.
///
/// Clears the background and renders content within a warning-colored border.
/// Use for caution dialogs or non-critical alerts.
pub struct WarningModal<'a, W: Widget> {
    title: &'a str,
    content: W,
    theme: &'a Theme,
}

impl<'a, W: Widget> WarningModal<'a, W> {
    /// Create a new warning modal.
    #[must_use]
    pub const fn new(title: &'a str, content: W, theme: &'a Theme) -> Self {
        Self {
            title,
            content,
            theme,
        }
    }
}

impl<W: Widget> Widget for WarningModal<'_, W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = warning_block(self.title, self.theme);
        let inner = block.inner(area);
        block.render(area, buf);
        self.content.render(inner, buf);
    }
}

/// A modal wrapper with danger styling.
///
/// Clears the background and renders content within a danger-colored border.
/// Use for error dialogs, destructive confirmations, or critical alerts.
pub struct DangerModal<'a, W: Widget> {
    title: &'a str,
    content: W,
    theme: &'a Theme,
}

impl<'a, W: Widget> DangerModal<'a, W> {
    /// Create a new danger modal.
    #[must_use]
    pub const fn new(title: &'a str, content: W, theme: &'a Theme) -> Self {
        Self {
            title,
            content,
            theme,
        }
    }
}

impl<W: Widget> Widget for DangerModal<'_, W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = danger_block(self.title, self.theme);
        let inner = block.inner(area);
        block.render(area, buf);
        self.content.render(inner, buf);
    }
}

/// A modal wrapper with success styling.
///
/// Clears the background and renders content within a success-colored border.
/// Use for success messages and positive confirmations.
pub struct SuccessModal<'a, W: Widget> {
    title: &'a str,
    content: W,
    theme: &'a Theme,
}

impl<'a, W: Widget> SuccessModal<'a, W> {
    /// Create a new success modal.
    #[must_use]
    pub const fn new(title: &'a str, content: W, theme: &'a Theme) -> Self {
        Self {
            title,
            content,
            theme,
        }
    }
}

impl<W: Widget> Widget for SuccessModal<'_, W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = success_block(self.title, self.theme);
        let inner = block.inner(area);
        block.render(area, buf);
        self.content.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_utils::{buffer_content, render_widget};
    use ratatui::widgets::Paragraph;

    fn content_of<W: Widget>(widget: W) -> String {
        buffer_content(&render_widget(widget, 40, 7))
    }

    #[test]
    fn test_panel_modal_renders_title_and_content() {
        let theme = Theme::default();
        let modal = PanelModal::new("Panel Title", Paragraph::new("panel body"), &theme);
        let content = content_of(modal);
        assert!(content.contains("Panel Title"));
        assert!(content.contains("panel body"));
    }

    #[test]
    fn test_accent_modal_renders_title_and_content() {
        let theme = Theme::default();
        let modal = AccentModal::new("Accent Title", Paragraph::new("accent body"), &theme);
        let content = content_of(modal);
        assert!(content.contains("Accent Title"));
        assert!(content.contains("accent body"));
    }

    #[test]
    fn test_warning_modal_renders_title_and_content() {
        let theme = Theme::default();
        let modal = WarningModal::new("Careful", Paragraph::new("warning body"), &theme);
        let content = content_of(modal);
        assert!(content.contains("Careful"));
        assert!(content.contains("warning body"));
    }

    #[test]
    fn test_danger_modal_renders_title_and_content() {
        let theme = Theme::default();
        let modal = DangerModal::new("Delete?", Paragraph::new("danger body"), &theme);
        let content = content_of(modal);
        assert!(content.contains("Delete?"));
        assert!(content.contains("danger body"));
    }

    #[test]
    fn test_success_modal_renders_title_and_content() {
        let theme = Theme::default();
        let modal = SuccessModal::new("Saved", Paragraph::new("success body"), &theme);
        let content = content_of(modal);
        assert!(content.contains("Saved"));
        assert!(content.contains("success body"));
    }

    #[test]
    fn test_modal_clears_background_before_drawing() {
        let theme = Theme::default();
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        // Pre-fill with junk that must disappear under the modal
        for y in 0..area.height {
            buf.set_string(
                0,
                y,
                "##############################",
                ratatui::style::Style::default(),
            );
        }
        PanelModal::new("T", Paragraph::new("inner"), &theme).render(area, &mut buf);
        let content = buffer_content(&buf);
        assert!(!content.contains("####"), "background was cleared");
        assert!(content.contains("inner"));
    }

    #[test]
    fn test_modal_content_is_inset_by_border() {
        let theme = Theme::default();
        let buf = render_widget(DangerModal::new("B", Paragraph::new("x"), &theme), 10, 3);
        // Content lands on the middle row, one cell in from the border
        let symbol = buf.cell((1, 1)).map(|c| c.symbol().to_string());
        assert_eq!(symbol.as_deref(), Some("x"));
    }
}
