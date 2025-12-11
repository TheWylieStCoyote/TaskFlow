//! Work log editor popup widget.
//!
//! Displays and allows editing of work log entries for a task.

mod render;

#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Clear, Widget},
};

use crate::config::Theme;
use crate::domain::{WorkLogEntry, WorkLogEntryId};

/// Mode for work log editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkLogMode {
    /// Browsing entries list
    #[default]
    Browse,
    /// Viewing a single entry's full content
    View,
    /// Adding a new entry (multi-line input)
    Add,
    /// Editing an existing entry (multi-line input)
    Edit,
    /// Confirming deletion
    ConfirmDelete,
    /// Searching/filtering entries
    Search,
}

/// Work log editor popup widget
pub struct WorkLogEditor<'a> {
    pub(crate) entries: Vec<&'a WorkLogEntry>,
    pub(crate) selected: usize,
    pub(crate) mode: WorkLogMode,
    pub(crate) edit_buffer: &'a [String],
    pub(crate) cursor_line: usize,
    pub(crate) cursor_col: usize,
    pub(crate) search_query: &'a str,
    pub(crate) theme: &'a Theme,
}

impl<'a> WorkLogEditor<'a> {
    #[must_use]
    pub fn new(
        entries: Vec<&'a WorkLogEntry>,
        selected: usize,
        mode: WorkLogMode,
        edit_buffer: &'a [String],
        cursor_line: usize,
        cursor_col: usize,
        search_query: &'a str,
        theme: &'a Theme,
    ) -> Self {
        Self {
            entries,
            selected,
            mode,
            edit_buffer,
            cursor_line,
            cursor_col,
            search_query,
            theme,
        }
    }

    /// Get the selected entry ID if any
    #[must_use]
    pub fn selected_entry_id(&self) -> Option<&WorkLogEntryId> {
        self.entries.get(self.selected).map(|e| &e.id)
    }
}

impl Widget for WorkLogEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        match self.mode {
            WorkLogMode::Browse => self.render_browse(area, buf),
            WorkLogMode::View => self.render_view(area, buf),
            WorkLogMode::Add | WorkLogMode::Edit => self.render_edit(area, buf),
            WorkLogMode::ConfirmDelete => self.render_confirm_delete(area, buf),
            WorkLogMode::Search => self.render_search(area, buf),
        }
    }
}

/// Truncate a string to a maximum length, adding ellipsis if needed.
pub(crate) fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
