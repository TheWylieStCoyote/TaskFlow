use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// Input mode for the application
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

use crate::domain::TaskId;

/// What type of item is being created/edited
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputTarget {
    #[default]
    Task,
    Subtask(TaskId), // Creating a subtask under the given parent
    EditTask(TaskId),
    EditDueDate(TaskId),
    EditScheduledDate(TaskId),
    EditTags(TaskId),
    EditDescription(TaskId),
    Project,
    Search,
    MoveToProject(TaskId),
    FilterByTag,
    BulkMoveToProject,
    BulkSetStatus,
    EditDependencies(TaskId),
    EditRecurrence(TaskId),
}

/// Input dialog for creating/editing items
pub struct InputDialog<'a> {
    title: &'a str,
    input: &'a str,
    cursor_position: usize,
}

impl<'a> InputDialog<'a> {
    #[must_use]
    pub const fn new(title: &'a str, input: &'a str, cursor_position: usize) -> Self {
        Self {
            title,
            input,
            cursor_position,
        }
    }
}

impl Widget for InputDialog<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        // Build the input text with cursor indicator
        let display_text = if self.cursor_position < self.input.len() {
            let (before, after) = self.input.split_at(self.cursor_position);
            let (_cursor_char, rest) = after.split_at(1);
            format!("{before}▌{rest}")
        } else {
            format!("{}▌", self.input)
        };

        let paragraph = Paragraph::new(display_text)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
                    .title_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );

        paragraph.render(area, buf);
    }
}

/// Confirmation dialog
pub struct ConfirmDialog<'a> {
    title: &'a str,
    message: &'a str,
}

impl<'a> ConfirmDialog<'a> {
    #[must_use]
    pub const fn new(title: &'a str, message: &'a str) -> Self {
        Self { title, message }
    }
}

impl Widget for ConfirmDialog<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let text = format!("{}\n\n[y]es / [n]o", self.message);

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
                    .title_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    /// Helper to render a widget into a buffer
    fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
        buffer
    }

    /// Extract text content from buffer
    fn buffer_content(buffer: &Buffer) -> String {
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push(
                    buffer
                        .cell((x, y))
                        .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
                );
            }
            content.push('\n');
        }
        content
    }

    // InputMode tests
    #[test]
    fn test_input_mode_default_is_normal() {
        let mode = InputMode::default();
        assert_eq!(mode, InputMode::Normal);
    }

    #[test]
    fn test_input_mode_equality() {
        assert_eq!(InputMode::Normal, InputMode::Normal);
        assert_eq!(InputMode::Editing, InputMode::Editing);
        assert_ne!(InputMode::Normal, InputMode::Editing);
    }

    // InputTarget tests
    #[test]
    fn test_input_target_default_is_task() {
        let target = InputTarget::default();
        assert_eq!(target, InputTarget::Task);
    }

    #[test]
    fn test_input_target_variants() {
        let task_id = TaskId::new();

        // Test each variant can be created
        let _ = InputTarget::Task;
        let _ = InputTarget::Subtask(task_id.clone());
        let _ = InputTarget::EditTask(task_id.clone());
        let _ = InputTarget::EditDueDate(task_id.clone());
        let _ = InputTarget::EditTags(task_id.clone());
        let _ = InputTarget::EditDescription(task_id.clone());
        let _ = InputTarget::Project;
        let _ = InputTarget::Search;
        let _ = InputTarget::MoveToProject(task_id.clone());
        let _ = InputTarget::FilterByTag;
        let _ = InputTarget::BulkMoveToProject;
        let _ = InputTarget::BulkSetStatus;
        let _ = InputTarget::EditDependencies(task_id.clone());
        let _ = InputTarget::EditRecurrence(task_id);
    }

    // InputDialog tests
    #[test]
    fn test_input_dialog_renders_title() {
        let dialog = InputDialog::new("New Task", "", 0);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        assert!(content.contains("New Task"), "Title should be visible");
    }

    #[test]
    fn test_input_dialog_renders_input_text() {
        let dialog = InputDialog::new("Edit", "Hello World", 11);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Hello World"),
            "Input text should be visible"
        );
    }

    #[test]
    fn test_input_dialog_shows_cursor() {
        let dialog = InputDialog::new("Test", "abc", 3);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        // Cursor indicator should be present
        assert!(content.contains('▌'), "Cursor indicator should be visible");
    }

    #[test]
    fn test_input_dialog_cursor_in_middle() {
        let dialog = InputDialog::new("Test", "abcdef", 3);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        // With cursor in the middle, we should see text before and after
        assert!(
            content.contains("abc"),
            "Text before cursor should be visible"
        );
    }

    #[test]
    fn test_input_dialog_empty_input() {
        let dialog = InputDialog::new("New", "", 0);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        // Should still show cursor
        assert!(
            content.contains('▌'),
            "Cursor should be visible even with empty input"
        );
    }

    // ConfirmDialog tests
    #[test]
    fn test_confirm_dialog_renders_title() {
        let dialog = ConfirmDialog::new("Confirm Delete", "Are you sure?");
        let buffer = render_widget(dialog, 40, 8);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Confirm Delete"),
            "Title should be visible"
        );
    }

    #[test]
    fn test_confirm_dialog_renders_message() {
        let dialog = ConfirmDialog::new("Delete", "Delete this task?");
        let buffer = render_widget(dialog, 40, 8);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Delete this task"),
            "Message should be visible"
        );
    }

    #[test]
    fn test_confirm_dialog_shows_yes_no_options() {
        let dialog = ConfirmDialog::new("Confirm", "Proceed?");
        let buffer = render_widget(dialog, 40, 8);
        let content = buffer_content(&buffer);

        assert!(content.contains("[y]es"), "Yes option should be visible");
        assert!(content.contains("[n]o"), "No option should be visible");
    }
}
