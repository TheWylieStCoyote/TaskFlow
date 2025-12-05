use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::Model;
use crate::config::Theme;

use super::components::{
    centered_rect, centered_rect_fixed_height, ConfirmDialog, HelpPopup, InputDialog, InputMode,
    InputTarget, Sidebar, TaskList,
};

/// Main view function - renders the entire UI based on model state
pub fn view(model: &Model, frame: &mut Frame, theme: &Theme) {
    let area = frame.area();

    // Main layout: header, content, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, chunks[0], theme);

    // Render main content
    render_content(model, frame, chunks[1], theme);

    // Render footer
    render_footer(model, frame, chunks[2], theme);

    // Render popups
    if model.show_help {
        let popup_area = centered_rect(50, 70, area);
        frame.render_widget(HelpPopup::new(), popup_area);
    }

    // Render input dialog if in editing mode
    if model.input_mode == InputMode::Editing {
        // Height: 3 rows (top border, text line, bottom border)
        let input_area = centered_rect_fixed_height(60, 3, area);
        let title = match &model.input_target {
            InputTarget::Task => "New Task",
            InputTarget::Subtask(_) => "New Subtask",
            InputTarget::EditTask(_) => "Edit Task",
            InputTarget::EditDueDate(_) => "Due Date (YYYY-MM-DD, empty to clear)",
            InputTarget::EditTags(_) => "Tags (comma-separated)",
            InputTarget::EditDescription(_) => "Description (empty to clear)",
            InputTarget::Project => "New Project",
            InputTarget::Search => "Search (Ctrl+L to clear)",
            InputTarget::MoveToProject(_) => "Move to Project (enter number)",
            InputTarget::FilterByTag => "Filter by Tag (comma-separated, Ctrl+T to clear)",
        };
        frame.render_widget(
            InputDialog::new(title, &model.input_buffer, model.cursor_position),
            input_area,
        );
    }

    // Render delete confirmation dialog
    if model.show_confirm_delete {
        // Height: 5 rows (border, message, blank, y/n prompt, border)
        let confirm_area = centered_rect_fixed_height(50, 5, area);
        let task_name = model
            .selected_task()
            .map(|t| t.title.as_str())
            .unwrap_or("this task");
        frame.render_widget(
            ConfirmDialog::new("Delete Task", &format!("Delete \"{}\"?", task_name)),
            confirm_area,
        );
    }
}

fn render_header(frame: &mut Frame, area: Rect, theme: &Theme) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " TaskFlow ",
            Style::default().fg(theme.colors.accent.to_color()),
        ),
        Span::raw("- Project Management TUI"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color())),
    );

    frame.render_widget(title, area);
}

fn render_content(model: &Model, frame: &mut Frame, area: Rect, theme: &Theme) {
    if model.show_sidebar {
        // Split into sidebar and main content
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25), // Sidebar
                Constraint::Min(0),     // Main content
            ])
            .split(area);

        // Render sidebar
        frame.render_widget(Sidebar::new(model, theme), chunks[0]);

        // Render task list in main area
        let task_list = TaskList::new(model, theme);
        frame.render_widget(task_list, chunks[1]);
    } else {
        // No sidebar, full width task list
        let task_list = TaskList::new(model, theme);
        frame.render_widget(task_list, area);
    }
}

fn render_footer(model: &Model, frame: &mut Frame, area: Rect, theme: &Theme) {
    let task_count = model.visible_tasks.len();
    let completed = model
        .tasks
        .values()
        .filter(|t| t.status.is_complete())
        .count();

    let status = format!(
        " {} tasks ({} completed) | {} | Press ? for help ",
        task_count,
        completed,
        if model.show_completed {
            "showing all"
        } else {
            "hiding completed"
        }
    );

    let footer = Paragraph::new(status).style(Style::default().fg(theme.colors.muted.to_color()));

    frame.render_widget(footer, area);
}
