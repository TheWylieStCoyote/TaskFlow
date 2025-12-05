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
            Line::from(""),
            Line::from(vec![
                Span::styled("j/↓", Style::default().fg(Color::Cyan)),
                Span::raw("     Move down"),
            ]),
            Line::from(vec![
                Span::styled("k/↑", Style::default().fg(Color::Cyan)),
                Span::raw("     Move up"),
            ]),
            Line::from(vec![
                Span::styled("g", Style::default().fg(Color::Cyan)),
                Span::raw("       Go to first"),
            ]),
            Line::from(vec![
                Span::styled("G", Style::default().fg(Color::Cyan)),
                Span::raw("       Go to last"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+u", Style::default().fg(Color::Cyan)),
                Span::raw("  Page up"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+d", Style::default().fg(Color::Cyan)),
                Span::raw("  Page down"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("a", Style::default().fg(Color::Cyan)),
                Span::raw("       Add new task"),
            ]),
            Line::from(vec![
                Span::styled("x/Space", Style::default().fg(Color::Cyan)),
                Span::raw(" Toggle complete"),
            ]),
            Line::from(vec![
                Span::styled("d", Style::default().fg(Color::Cyan)),
                Span::raw("       Delete task"),
            ]),
            Line::from(vec![
                Span::styled("e", Style::default().fg(Color::Cyan)),
                Span::raw("       Edit task title"),
            ]),
            Line::from(vec![
                Span::styled("p", Style::default().fg(Color::Cyan)),
                Span::raw("       Cycle priority"),
            ]),
            Line::from(vec![
                Span::styled("m", Style::default().fg(Color::Cyan)),
                Span::raw("       Move to project"),
            ]),
            Line::from(vec![
                Span::styled("t", Style::default().fg(Color::Cyan)),
                Span::raw("       Toggle time tracking"),
            ]),
            Line::from(vec![
                Span::styled("c", Style::default().fg(Color::Cyan)),
                Span::raw("       Toggle show completed"),
            ]),
            Line::from(vec![
                Span::styled("b", Style::default().fg(Color::Cyan)),
                Span::raw("       Toggle sidebar"),
            ]),
            Line::from(vec![
                Span::styled("/", Style::default().fg(Color::Cyan)),
                Span::raw("       Search tasks"),
            ]),
            Line::from(vec![
                Span::styled("#", Style::default().fg(Color::Cyan)),
                Span::raw("       Filter by tag"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+t", Style::default().fg(Color::Cyan)),
                Span::raw("  Clear tag filter"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "General",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("u/Ctrl+z", Style::default().fg(Color::Cyan)),
                Span::raw(" Undo last action"),
            ]),
            Line::from(vec![
                Span::styled("U/Ctrl+r", Style::default().fg(Color::Cyan)),
                Span::raw(" Redo last action"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+s", Style::default().fg(Color::Cyan)),
                Span::raw("   Save"),
            ]),
            Line::from(vec![
                Span::styled("?", Style::default().fg(Color::Cyan)),
                Span::raw("        Show/hide help"),
            ]),
            Line::from(vec![
                Span::styled("q/Esc", Style::default().fg(Color::Cyan)),
                Span::raw("    Quit"),
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

/// Calculate centered rect for popup
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = (r.width - popup_width) / 2;
    let popup_y = (r.height - popup_height) / 2;

    Rect::new(r.x + popup_x, r.y + popup_y, popup_width, popup_height)
}
