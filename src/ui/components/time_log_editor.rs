//! Time log editor component.
//!
//! A modal popup for viewing and editing time entries associated with a task.
//! Users can browse entries, edit start/end times, and delete entries.
//!
//! # Modes
//!
//! - **Browse**: Navigate through time entries
//! - **EditStart**: Edit the start time of selected entry
//! - **EditEnd**: Edit the end time of selected entry
//! - **ConfirmDelete**: Confirm deletion of an entry

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Widget},
};

use crate::config::Theme;
use crate::domain::{TimeEntry, TimeEntryId};

/// Mode for time log editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeLogMode {
    #[default]
    Browse,
    EditStart,
    EditEnd,
    ConfirmDelete,
}

/// Time log editor popup widget
pub struct TimeLogEditor<'a> {
    entries: Vec<&'a TimeEntry>,
    selected: usize,
    mode: TimeLogMode,
    edit_buffer: &'a str,
    theme: &'a Theme,
}

impl<'a> TimeLogEditor<'a> {
    #[must_use]
    pub fn new(
        entries: Vec<&'a TimeEntry>,
        selected: usize,
        mode: TimeLogMode,
        edit_buffer: &'a str,
        theme: &'a Theme,
    ) -> Self {
        Self {
            entries,
            selected,
            mode,
            edit_buffer,
            theme,
        }
    }

    /// Get the selected entry ID if any
    #[must_use]
    pub fn selected_entry_id(&self) -> Option<&TimeEntryId> {
        self.entries.get(self.selected).map(|e| &e.id)
    }
}

impl Widget for TimeLogEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        let theme = self.theme;

        // Build list items
        let items: Vec<ListItem<'_>> = self
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = idx == self.selected;
                let is_running = entry.is_running();

                // Format: [date] [start] - [end] [duration] [status]
                let date = entry.started_at.format("%Y-%m-%d").to_string();
                let start_time = entry.started_at.format("%H:%M").to_string();
                let end_time = entry
                    .ended_at
                    .map_or_else(|| "running".to_string(), |t| t.format("%H:%M").to_string());
                let duration = entry.formatted_duration();

                let status_indicator = if is_running { "●" } else { " " };

                let mut spans = vec![
                    Span::styled(
                        status_indicator,
                        Style::default()
                            .fg(if is_running {
                                Color::Red
                            } else {
                                theme.colors.muted.to_color()
                            })
                            .add_modifier(if is_running {
                                Modifier::SLOW_BLINK
                            } else {
                                Modifier::empty()
                            }),
                    ),
                    Span::raw(" "),
                    Span::styled(date, Style::default().fg(theme.colors.muted.to_color())),
                    Span::raw("  "),
                ];

                // Highlight start time if editing
                if is_selected && self.mode == TimeLogMode::EditStart {
                    spans.push(Span::styled(
                        format!("[{}]", self.edit_buffer),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::styled(
                        start_time,
                        Style::default()
                            .fg(theme.colors.accent.to_color())
                            .add_modifier(Modifier::BOLD),
                    ));
                }

                spans.push(Span::styled(
                    " - ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ));

                // Highlight end time if editing
                if is_selected && self.mode == TimeLogMode::EditEnd {
                    spans.push(Span::styled(
                        format!("[{}]", self.edit_buffer),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::styled(
                        end_time,
                        Style::default().fg(if is_running {
                            Color::Red
                        } else {
                            theme.colors.accent.to_color()
                        }),
                    ));
                }

                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    format!("({duration})"),
                    Style::default().fg(theme.colors.foreground.to_color()),
                ));

                let style = if is_selected {
                    Style::default()
                        .bg(theme.colors.accent_secondary.to_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(style)
            })
            .collect();

        let title = match self.mode {
            TimeLogMode::Browse => {
                " Time Log (s=edit start, e=edit end, d=delete, a=add, Esc=close) "
            }
            TimeLogMode::EditStart => " Edit Start Time (HH:MM, Enter=save, Esc=cancel) ",
            TimeLogMode::EditEnd => " Edit End Time (HH:MM, Enter=save, Esc=cancel) ",
            TimeLogMode::ConfirmDelete => " Delete this entry? (y=yes, n=no) ",
        };

        let border_color = match self.mode {
            TimeLogMode::Browse => theme.colors.accent.to_color(),
            TimeLogMode::EditStart | TimeLogMode::EditEnd => Color::Yellow,
            TimeLogMode::ConfirmDelete => Color::Red,
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let list = if items.is_empty() {
            List::new(vec![ListItem::new(Line::from(Span::styled(
                "  No time entries yet. Press 'a' to add one.",
                Style::default().fg(theme.colors.muted.to_color()),
            )))])
        } else {
            List::new(items)
        };

        let list = list.block(block);
        list.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use crate::domain::TaskId;
    use crate::ui::test_utils::{buffer_content, render_widget};

    #[test]
    fn test_time_log_editor_empty() {
        let theme = Theme::default();
        let editor = TimeLogEditor::new(vec![], 0, TimeLogMode::Browse, "", &theme);
        let buffer = render_widget(editor, 60, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Time Log"));
        assert!(content.contains("No time entries"));
    }

    #[test]
    fn test_time_log_editor_with_entry() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.stop();
        entry.duration_minutes = Some(30);

        let editor = TimeLogEditor::new(vec![&entry], 0, TimeLogMode::Browse, "", &theme);
        let buffer = render_widget(editor, 80, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Time Log"));
        assert!(content.contains("30m"));
    }

    #[test]
    fn test_time_log_editor_running_entry() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let entry = TimeEntry::start(task_id);

        let editor = TimeLogEditor::new(vec![&entry], 0, TimeLogMode::Browse, "", &theme);
        let buffer = render_widget(editor, 80, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("running"));
    }

    #[test]
    fn test_time_log_editor_edit_mode() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.stop();

        let editor = TimeLogEditor::new(vec![&entry], 0, TimeLogMode::EditStart, "10:30", &theme);
        let buffer = render_widget(editor, 80, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Edit Start Time"));
    }

    #[test]
    fn test_time_log_editor_delete_confirm() {
        let theme = Theme::default();
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.stop();

        let editor = TimeLogEditor::new(vec![&entry], 0, TimeLogMode::ConfirmDelete, "", &theme);
        let buffer = render_widget(editor, 80, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Delete"));
    }
}
