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
