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
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for HelpPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for HelpPopup {
    #[allow(clippy::too_many_lines)]
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
                Span::styled("S", Style::default().fg(Color::Cyan)),
                Span::raw("         Edit scheduled date"),
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
            Line::from(vec![
                Span::styled("f", Style::default().fg(Color::Cyan)),
                Span::raw("         Toggle focus mode"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+↑", Style::default().fg(Color::Cyan)),
                Span::raw("    Move task up"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+↓", Style::default().fg(Color::Cyan)),
                Span::raw("    Move task down"),
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
                Span::styled("Esc", Style::default().fg(Color::Cyan)),
                Span::raw("       Clear search (in search mode)"),
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
                "Task Chains",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Ctrl+l", Style::default().fg(Color::Cyan)),
                Span::raw("    Link to next task"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+L", Style::default().fg(Color::Cyan)),
                Span::raw("    Unlink from chain"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Calendar View",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("←/→", Style::default().fg(Color::Cyan)),
                Span::raw("       Navigate days"),
            ]),
            Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
                Span::raw("       Navigate weeks"),
            ]),
            Line::from(vec![
                Span::styled("</> ", Style::default().fg(Color::Cyan)),
                Span::raw("      Previous/Next month"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Export",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Ctrl+e", Style::default().fg(Color::Cyan)),
                Span::raw("    Export to CSV"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+i", Style::default().fg(Color::Cyan)),
                Span::raw("    Export to ICS (calendar)"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Macros",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Ctrl+q", Style::default().fg(Color::Cyan)),
                Span::raw("    Record macro (press 0-9 for slot)"),
            ]),
            Line::from(vec![
                Span::styled("@0-9", Style::default().fg(Color::Cyan)),
                Span::raw("      Play macro from slot"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Templates",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Ctrl+n", Style::default().fg(Color::Cyan)),
                Span::raw("    Create task from template"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Pomodoro Timer",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("F5", Style::default().fg(Color::Cyan)),
                Span::raw("        Start Pomodoro session"),
            ]),
            Line::from(vec![
                Span::styled("F6", Style::default().fg(Color::Cyan)),
                Span::raw("        Pause/Resume timer"),
            ]),
            Line::from(vec![
                Span::styled("F7", Style::default().fg(Color::Cyan)),
                Span::raw("        Skip current phase"),
            ]),
            Line::from(vec![
                Span::styled("F8", Style::default().fg(Color::Cyan)),
                Span::raw("        Stop Pomodoro session"),
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
#[must_use]
pub const fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = (r.width - popup_width) / 2;
    let popup_y = (r.height - popup_height) / 2;

    Rect::new(r.x + popup_x, r.y + popup_y, popup_width, popup_height)
}

/// Calculate centered rect with fixed height (for input dialogs)
#[must_use]
pub fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = height.min(r.height); // Don't exceed screen height
    let popup_x = (r.width - popup_width) / 2;
    let popup_y = (r.height.saturating_sub(popup_height)) / 2;

    Rect::new(r.x + popup_x, r.y + popup_y, popup_width, popup_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    /// Helper to render a widget into a buffer and return it
    fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
        buffer
    }

    /// Extract text content from buffer (ignoring styles)
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

    #[test]
    fn test_centered_rect_calculates_correct_position() {
        let screen = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(50, 50, screen);

        assert_eq!(popup.width, 50);
        assert_eq!(popup.height, 25);
        assert_eq!(popup.x, 25); // Centered horizontally
        assert_eq!(popup.y, 12); // Centered vertically
    }

    #[test]
    fn test_centered_rect_with_offset_parent() {
        let screen = Rect::new(10, 5, 100, 50);
        let popup = centered_rect(50, 50, screen);

        assert_eq!(popup.width, 50);
        assert_eq!(popup.height, 25);
        assert_eq!(popup.x, 35); // 10 + (100-50)/2 = 35
        assert_eq!(popup.y, 17); // 5 + (50-25)/2 = 17
    }

    #[test]
    fn test_centered_rect_fixed_height_calculates_correct_position() {
        let screen = Rect::new(0, 0, 100, 50);
        let popup = centered_rect_fixed_height(60, 10, screen);

        assert_eq!(popup.width, 60);
        assert_eq!(popup.height, 10);
        assert_eq!(popup.x, 20); // Centered horizontally
        assert_eq!(popup.y, 20); // Centered vertically
    }

    #[test]
    fn test_centered_rect_fixed_height_clamps_to_screen() {
        let screen = Rect::new(0, 0, 100, 20);
        let popup = centered_rect_fixed_height(50, 30, screen); // Request 30, screen is 20

        assert_eq!(popup.height, 20); // Should be clamped to screen height
    }

    #[test]
    fn test_help_popup_renders_title() {
        let popup = HelpPopup::new();
        let buffer = render_widget(popup, 60, 30);
        let content = buffer_content(&buffer);

        assert!(content.contains("Help"), "Help title should be visible");
    }

    #[test]
    fn test_help_popup_renders_navigation_section() {
        let popup = HelpPopup::new();
        let buffer = render_widget(popup, 60, 40);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Navigation"),
            "Navigation section header should be visible"
        );
        assert!(
            content.contains("Move down") || content.contains("j"),
            "Navigation instructions should be visible"
        );
    }

    #[test]
    fn test_help_popup_renders_keybindings() {
        let popup = HelpPopup::new();
        let buffer = render_widget(popup, 60, 50);
        let content = buffer_content(&buffer);

        // Check for various keybinding categories
        assert!(
            content.contains("Tasks") || content.contains("Add new task"),
            "Tasks section should be visible"
        );
    }

    #[test]
    fn test_help_popup_default_impl() {
        let popup1 = HelpPopup::new();
        let popup2 = HelpPopup::default();

        // Both should render the same content
        let buffer1 = render_widget(popup1, 40, 20);
        let buffer2 = render_widget(popup2, 40, 20);

        let content1 = buffer_content(&buffer1);
        let content2 = buffer_content(&buffer2);

        assert_eq!(content1, content2);
    }
}
