use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::Model;

use super::components::{centered_rect, HelpPopup, TaskList};

/// Main view function - renders the entire UI based on model state
pub fn view(model: &Model, frame: &mut Frame) {
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
    render_header(frame, chunks[0]);

    // Render main content
    render_content(model, frame, chunks[1]);

    // Render footer
    render_footer(model, frame, chunks[2]);

    // Render help popup if visible
    if model.show_help {
        let popup_area = centered_rect(50, 70, area);
        frame.render_widget(HelpPopup::new(), popup_area);
    }
}

fn render_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " TaskFlow ",
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("- Project Management TUI"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(title, area);
}

fn render_content(model: &Model, frame: &mut Frame, area: Rect) {
    let task_list = TaskList::new(model);
    frame.render_widget(task_list, area);
}

fn render_footer(model: &Model, frame: &mut Frame, area: Rect) {
    let task_count = model.visible_tasks.len();
    let completed = model.tasks.values().filter(|t| t.status.is_complete()).count();

    let status = format!(
        " {} tasks ({} completed) | {} | Press ? for help ",
        task_count,
        completed,
        if model.show_completed { "showing all" } else { "hiding completed" }
    );

    let footer = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(footer, area);
}
