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
    EditTags(TaskId),
    EditDescription(TaskId),
    Project,
    Search,
    MoveToProject(TaskId),
    FilterByTag,
    BulkMoveToProject,
    BulkSetStatus,
    EditDependencies(TaskId),
}

/// Input dialog for creating/editing items
pub struct InputDialog<'a> {
    title: &'a str,
    input: &'a str,
    cursor_position: usize,
}

impl<'a> InputDialog<'a> {
    pub fn new(title: &'a str, input: &'a str, cursor_position: usize) -> Self {
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
            format!("{}▌{}", before, rest)
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
    pub fn new(title: &'a str, message: &'a str) -> Self {
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
