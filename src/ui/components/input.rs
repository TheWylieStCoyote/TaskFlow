//! Text input component and input state management.
//!
//! Provides the input field widget for task creation, editing, and search.
//! Handles different input modes (normal vs editing) and input targets
//! (task, project, tag, etc.).
//!
//! # Input Modes
//!
//! - **Normal**: Regular navigation, keypresses trigger actions
//! - **Editing**: Text input mode, keypresses insert characters
//!
//! # Input Targets
//!
//! The input field can target different entity types: tasks, subtasks,
//! projects, tags, due dates, and more.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::config::Theme;

/// Input mode for the application
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

use crate::domain::{HabitId, ProjectId, TaskId};

use crate::storage::ImportFormat;

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
    EditProject(ProjectId), // Renaming an existing project
    Search,
    MoveToProject(TaskId),
    FilterByTag,
    BulkMoveToProject,
    BulkSetStatus,
    EditDependencies(TaskId),
    EditRecurrence(TaskId),
    LinkTask(TaskId),             // Linking current task to next task in chain
    ImportFilePath(ImportFormat), // File path input for import
    SavedFilterName,              // Name for a new saved filter
    SnoozeTask(TaskId),           // Snooze date for a task
    EditEstimate(TaskId),         // Time estimate for a task (e.g., "30m", "2h", "1h30m")
    NewHabit,                     // Creating a new habit
    EditHabit(HabitId),           // Editing an existing habit's name
    QuickCapture,                 // Quick capture mode with syntax hints
}

/// Input dialog for creating/editing items
pub struct InputDialog<'a> {
    title: &'a str,
    input: &'a str,
    cursor_position: usize,
    theme: &'a Theme,
}

impl<'a> InputDialog<'a> {
    #[must_use]
    pub const fn new(
        title: &'a str,
        input: &'a str,
        cursor_position: usize,
        theme: &'a Theme,
    ) -> Self {
        Self {
            title,
            input,
            cursor_position,
            theme,
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

        let accent = self.theme.colors.accent.to_color();
        let paragraph = Paragraph::new(display_text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
                    .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent)),
            );

        paragraph.render(area, buf);
    }
}

/// Quick capture dialog with syntax hints
pub struct QuickCaptureDialog<'a> {
    input: &'a str,
    cursor_position: usize,
    theme: &'a Theme,
}

impl<'a> QuickCaptureDialog<'a> {
    #[must_use]
    pub const fn new(input: &'a str, cursor_position: usize, theme: &'a Theme) -> Self {
        Self {
            input,
            cursor_position,
            theme,
        }
    }
}

impl Widget for QuickCaptureDialog<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::text::{Line, Span};

        Clear.render(area, buf);

        let accent = self.theme.colors.accent.to_color();
        let block = Block::default()
            .title(" Quick Capture (Esc to close, Enter to add) ")
            .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(accent));

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into input line and hints
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        // Render input with cursor
        let display_text = if self.cursor_position < self.input.len() {
            let (before, after) = self.input.split_at(self.cursor_position);
            let (_cursor_char, rest) = after.split_at(1);
            format!("{before}▌{rest}")
        } else {
            format!("{}▌", self.input)
        };

        let input_line = Paragraph::new(display_text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()));
        input_line.render(chunks[0], buf);

        // Render hints using theme colors
        let hints = [
            Line::from(vec![
                Span::styled(
                    "#tag ",
                    Style::default().fg(self.theme.colors.success.to_color()),
                ),
                Span::styled(
                    "@project ",
                    Style::default().fg(self.theme.priority.high.to_color()),
                ),
                Span::styled(
                    "!priority ",
                    Style::default().fg(self.theme.colors.warning.to_color()),
                ),
                Span::styled(
                    "due:date ",
                    Style::default().fg(self.theme.colors.danger.to_color()),
                ),
                Span::styled("sched:date", Style::default().fg(accent)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Examples: ",
                    Style::default()
                        .fg(self.theme.colors.muted.to_color())
                        .add_modifier(Modifier::ITALIC),
                ),
                Span::styled(
                    "Buy milk #groceries @Home !high due:tomorrow",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]),
        ];

        for (i, line) in hints.iter().enumerate() {
            if i + 2 < chunks.len() {
                let hint_para = Paragraph::new(line.clone());
                hint_para.render(chunks[i + 2], buf);
            }
        }
    }
}

/// Confirmation dialog
pub struct ConfirmDialog<'a> {
    title: &'a str,
    message: &'a str,
    theme: &'a Theme,
}

impl<'a> ConfirmDialog<'a> {
    #[must_use]
    pub const fn new(title: &'a str, message: &'a str, theme: &'a Theme) -> Self {
        Self {
            title,
            message,
            theme,
        }
    }
}

impl Widget for ConfirmDialog<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let text = format!("{}\n\n[y]es / [n]o", self.message);
        let warning = self.theme.colors.warning.to_color();

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
                    .title_style(Style::default().fg(warning).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(warning)),
            );

        paragraph.render(area, buf);
    }
}

/// Overdue tasks alert popup shown at startup
pub struct OverdueAlert<'a> {
    count: usize,
    task_titles: Vec<String>,
    theme: &'a Theme,
}

impl<'a> OverdueAlert<'a> {
    #[must_use]
    pub fn new(count: usize, task_titles: Vec<String>, theme: &'a Theme) -> Self {
        Self {
            count,
            task_titles,
            theme,
        }
    }
}

impl Widget for OverdueAlert<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let mut lines = vec![format!(
            "You have {} overdue task{}!\n",
            self.count,
            if self.count == 1 { "" } else { "s" }
        )];

        // Show up to 5 task titles
        for (i, title) in self.task_titles.iter().take(5).enumerate() {
            lines.push(format!("  {}. {}", i + 1, title));
        }
        if self.count > 5 {
            lines.push(format!("  ... and {} more", self.count - 5));
        }

        lines.push(String::new());
        lines.push("Press any key to dismiss".to_string());

        let text = lines.join("\n");
        let danger = self.theme.colors.danger.to_color();

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(
                Block::default()
                    .title(" ⚠ Overdue Tasks ")
                    .title_style(Style::default().fg(danger).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(danger)),
            );

        paragraph.render(area, buf);
    }
}

/// Storage error alert popup shown when data cannot be loaded
pub struct StorageErrorAlert<'a> {
    error_message: &'a str,
    theme: &'a Theme,
}

impl<'a> StorageErrorAlert<'a> {
    #[must_use]
    pub fn new(error_message: &'a str, theme: &'a Theme) -> Self {
        Self {
            error_message,
            theme,
        }
    }
}

impl Widget for StorageErrorAlert<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let text = format!(
            "Could not load your task data:\n\n  {}\n\nStarting with sample data instead.\nYour existing data has not been modified.\n\nPress any key to continue",
            self.error_message
        );
        let warning = self.theme.colors.warning.to_color();

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(
                Block::default()
                    .title(" ⚠ Storage Error ")
                    .title_style(Style::default().fg(warning).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(warning)),
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

    /// Create a default theme for testing
    fn test_theme() -> Theme {
        Theme::default()
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
        let _ = InputTarget::Subtask(task_id);
        let _ = InputTarget::EditTask(task_id);
        let _ = InputTarget::EditDueDate(task_id);
        let _ = InputTarget::EditTags(task_id);
        let _ = InputTarget::EditDescription(task_id);
        let _ = InputTarget::Project;
        let _ = InputTarget::Search;
        let _ = InputTarget::MoveToProject(task_id);
        let _ = InputTarget::FilterByTag;
        let _ = InputTarget::BulkMoveToProject;
        let _ = InputTarget::BulkSetStatus;
        let _ = InputTarget::EditDependencies(task_id);
        let _ = InputTarget::EditRecurrence(task_id);
    }

    // InputDialog tests
    #[test]
    fn test_input_dialog_renders_title() {
        let theme = test_theme();
        let dialog = InputDialog::new("New Task", "", 0, &theme);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        assert!(content.contains("New Task"), "Title should be visible");
    }

    #[test]
    fn test_input_dialog_renders_input_text() {
        let theme = test_theme();
        let dialog = InputDialog::new("Edit", "Hello World", 11, &theme);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Hello World"),
            "Input text should be visible"
        );
    }

    #[test]
    fn test_input_dialog_shows_cursor() {
        let theme = test_theme();
        let dialog = InputDialog::new("Test", "abc", 3, &theme);
        let buffer = render_widget(dialog, 40, 5);
        let content = buffer_content(&buffer);

        // Cursor indicator should be present
        assert!(content.contains('▌'), "Cursor indicator should be visible");
    }

    #[test]
    fn test_input_dialog_cursor_in_middle() {
        let theme = test_theme();
        let dialog = InputDialog::new("Test", "abcdef", 3, &theme);
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
        let theme = test_theme();
        let dialog = InputDialog::new("New", "", 0, &theme);
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
        let theme = test_theme();
        let dialog = ConfirmDialog::new("Confirm Delete", "Are you sure?", &theme);
        let buffer = render_widget(dialog, 40, 8);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Confirm Delete"),
            "Title should be visible"
        );
    }

    #[test]
    fn test_confirm_dialog_renders_message() {
        let theme = test_theme();
        let dialog = ConfirmDialog::new("Delete", "Delete this task?", &theme);
        let buffer = render_widget(dialog, 40, 8);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Delete this task"),
            "Message should be visible"
        );
    }

    #[test]
    fn test_confirm_dialog_shows_yes_no_options() {
        let theme = test_theme();
        let dialog = ConfirmDialog::new("Confirm", "Proceed?", &theme);
        let buffer = render_widget(dialog, 40, 8);
        let content = buffer_content(&buffer);

        assert!(content.contains("[y]es"), "Yes option should be visible");
        assert!(content.contains("[n]o"), "No option should be visible");
    }
}
