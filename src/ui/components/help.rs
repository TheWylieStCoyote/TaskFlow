use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

/// Help popup widget
pub struct HelpPopup;

impl HelpPopup {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for HelpPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        let help_text = vec![
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("j/↓", Style::default().fg(Color::Cyan)),
                Span::raw("       Move down"),
            ]),
            Line::from(vec![
                Span::styled("k/↑", Style::default().fg(Color::Cyan)),
                Span::raw("       Move up"),
            ]),
            Line::from(vec![
                Span::styled("g/G", Style::default().fg(Color::Cyan)),
                Span::raw("       Go to first/last"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+u/d", Style::default().fg(Color::Cyan)),
                Span::raw("   Page up/down"),
            ]),
            Line::from(vec![
                Span::styled("h/←", Style::default().fg(Color::Cyan)),
                Span::raw("       Focus sidebar"),
            ]),
            Line::from(vec![
                Span::styled("l/→", Style::default().fg(Color::Cyan)),
                Span::raw("       Focus task list"),
            ]),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Cyan)),
                Span::raw("     Select item"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Tasks",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("a", Style::default().fg(Color::Cyan)),
                Span::raw("         Add new task"),
            ]),
            Line::from(vec![
                Span::styled("A", Style::default().fg(Color::Cyan)),
                Span::raw("         Add subtask"),
            ]),
            Line::from(vec![
                Span::styled("e", Style::default().fg(Color::Cyan)),
                Span::raw("         Edit task title"),
            ]),
            Line::from(vec![
                Span::styled("d", Style::default().fg(Color::Cyan)),
                Span::raw("         Delete task"),
            ]),
            Line::from(vec![
                Span::styled("x/Space", Style::default().fg(Color::Cyan)),
                Span::raw("   Toggle complete"),
            ]),
            Line::from(vec![
                Span::styled("p", Style::default().fg(Color::Cyan)),
                Span::raw("         Cycle priority"),
            ]),
            Line::from(vec![
                Span::styled("D", Style::default().fg(Color::Cyan)),
                Span::raw("         Edit due date"),
            ]),
            Line::from(vec![
                Span::styled("T", Style::default().fg(Color::Cyan)),
                Span::raw("         Edit tags"),
            ]),
            Line::from(vec![
                Span::styled("n", Style::default().fg(Color::Cyan)),
                Span::raw("         Edit description/notes"),
            ]),
            Line::from(vec![
                Span::styled("m", Style::default().fg(Color::Cyan)),
                Span::raw("         Move to project"),
            ]),
            Line::from(vec![
                Span::styled("t", Style::default().fg(Color::Cyan)),
                Span::raw("         Toggle time tracking"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Projects",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("P", Style::default().fg(Color::Cyan)),
                Span::raw("         Create new project"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "View & Filter",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("b", Style::default().fg(Color::Cyan)),
                Span::raw("         Toggle sidebar"),
            ]),
            Line::from(vec![
                Span::styled("c", Style::default().fg(Color::Cyan)),
                Span::raw("         Toggle show completed"),
            ]),
            Line::from(vec![
                Span::styled("/", Style::default().fg(Color::Cyan)),
                Span::raw("         Search tasks"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+l", Style::default().fg(Color::Cyan)),
                Span::raw("    Clear search"),
            ]),
            Line::from(vec![
                Span::styled("#", Style::default().fg(Color::Cyan)),
                Span::raw("         Filter by tag"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+t", Style::default().fg(Color::Cyan)),
                Span::raw("    Clear tag filter"),
            ]),
            Line::from(vec![
                Span::styled("s", Style::default().fg(Color::Cyan)),
                Span::raw("         Cycle sort field"),
            ]),
            Line::from(vec![
                Span::styled("S", Style::default().fg(Color::Cyan)),
                Span::raw("         Toggle sort order"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Multi-Select",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("v", Style::default().fg(Color::Cyan)),
                Span::raw("         Toggle multi-select mode"),
            ]),
            Line::from(vec![
                Span::styled("V", Style::default().fg(Color::Cyan)),
                Span::raw("         Select all tasks"),
            ]),
            Line::from(vec![
                Span::styled("Space", Style::default().fg(Color::Cyan)),
                Span::raw("     Toggle task selection (in multi-select)"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+v", Style::default().fg(Color::Cyan)),
                Span::raw("    Clear selection"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Dependencies & Recurrence",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("B", Style::default().fg(Color::Cyan)),
                Span::raw("         Edit dependencies (blocked by)"),
            ]),
            Line::from(vec![
                Span::styled("R", Style::default().fg(Color::Cyan)),
                Span::raw("         Set recurrence (d/w/m/y/0)"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "General",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("u/Ctrl+z", Style::default().fg(Color::Cyan)),
                Span::raw("   Undo"),
            ]),
            Line::from(vec![
                Span::styled("U/Ctrl+r", Style::default().fg(Color::Cyan)),
                Span::raw("   Redo"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+s", Style::default().fg(Color::Cyan)),
                Span::raw("    Save"),
            ]),
            Line::from(vec![
                Span::styled("?", Style::default().fg(Color::Cyan)),
                Span::raw("         Show/hide help"),
            ]),
            Line::from(vec![
                Span::styled("q/Esc", Style::default().fg(Color::Cyan)),
                Span::raw("     Quit"),
            ]),
        ];

        let paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Help ")
                    .title_alignment(Alignment::Center)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        paragraph.render(area, buf);
    }
}

/// Calculate centered rect for popup using percentages
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = (r.width - popup_width) / 2;
    let popup_y = (r.height - popup_height) / 2;

    Rect::new(r.x + popup_x, r.y + popup_y, popup_width, popup_height)
}

/// Calculate centered rect with fixed height (for input dialogs)
pub fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = height.min(r.height); // Don't exceed screen height
    let popup_x = (r.width - popup_width) / 2;
    let popup_y = (r.height.saturating_sub(popup_height)) / 2;

    Rect::new(r.x + popup_x, r.y + popup_y, popup_width, popup_height)
}
