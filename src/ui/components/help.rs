use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::config::Keybindings;

/// Help popup widget that displays keybindings dynamically
pub struct HelpPopup<'a> {
    keybindings: &'a Keybindings,
}

impl<'a> HelpPopup<'a> {
    #[must_use]
    pub const fn new(keybindings: &'a Keybindings) -> Self {
        Self { keybindings }
    }
}

impl Widget for HelpPopup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        // Get bindings grouped by category
        let grouped = self.keybindings.bindings_by_category();

        let mut help_lines: Vec<Line> = Vec::new();

        for (category, bindings) in grouped {
            // Add category header
            help_lines.push(Line::from(vec![Span::styled(
                category.display_name(),
                Style::default().add_modifier(Modifier::BOLD),
            )]));

            // Add each binding in this category
            for (key, _action, description) in bindings {
                // Format the key for display
                let display_key = format_key_for_display(&key);

                // Pad key to align descriptions
                let padded_key = format!("{:<10}", display_key);

                help_lines.push(Line::from(vec![
                    Span::styled(padded_key, Style::default().fg(Color::Cyan)),
                    Span::raw(description),
                ]));
            }

            // Add empty line between categories
            help_lines.push(Line::from(""));
        }

        // Remove trailing empty line if present
        if help_lines
            .last()
            .map(|l| l.spans.is_empty())
            .unwrap_or(false)
        {
            help_lines.pop();
        }

        let paragraph = Paragraph::new(help_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Help (press ? or Esc to close) ")
                    .title_alignment(Alignment::Center)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        paragraph.render(area, buf);
    }
}

/// Format a key string for display (make it more readable)
fn format_key_for_display(key: &str) -> String {
    // Handle special key names
    match key.to_lowercase().as_str() {
        "enter" => "Enter".to_string(),
        "esc" => "Esc".to_string(),
        "tab" => "Tab".to_string(),
        "space" => "Space".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        _ => {
            // Handle modifiers
            if let Some(rest) = key.strip_prefix("ctrl+") {
                format!("Ctrl+{}", format_key_part(rest))
            } else if let Some(rest) = key.strip_prefix("alt+") {
                format!("Alt+{}", format_key_part(rest))
            } else if let Some(rest) = key.strip_prefix("shift+") {
                format!("Shift+{}", format_key_part(rest))
            } else {
                key.to_string()
            }
        }
    }
}

/// Format a key part (after modifier)
fn format_key_part(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        _ => key.to_string(),
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
        let keybindings = Keybindings::default();
        let popup = HelpPopup::new(&keybindings);
        let buffer = render_widget(popup, 60, 30);
        let content = buffer_content(&buffer);

        assert!(content.contains("Help"), "Help title should be visible");
    }

    #[test]
    fn test_help_popup_renders_categories() {
        let keybindings = Keybindings::default();
        let popup = HelpPopup::new(&keybindings);
        let buffer = render_widget(popup, 60, 80);
        let content = buffer_content(&buffer);

        // Check for category headers
        assert!(
            content.contains("Navigation"),
            "Navigation category should be visible"
        );
        assert!(
            content.contains("Tasks"),
            "Tasks category should be visible"
        );
    }

    #[test]
    fn test_help_popup_renders_keybindings() {
        let keybindings = Keybindings::default();
        let popup = HelpPopup::new(&keybindings);
        let buffer = render_widget(popup, 60, 80);
        let content = buffer_content(&buffer);

        // Check for some expected keybindings
        assert!(
            content.contains("Move") || content.contains("up") || content.contains("down"),
            "Movement instructions should be visible"
        );
    }

    #[test]
    fn test_format_key_for_display() {
        assert_eq!(format_key_for_display("enter"), "Enter");
        assert_eq!(format_key_for_display("esc"), "Esc");
        assert_eq!(format_key_for_display("ctrl+s"), "Ctrl+s");
        assert_eq!(format_key_for_display("ctrl+up"), "Ctrl+↑");
        assert_eq!(format_key_for_display("left"), "←");
        assert_eq!(format_key_for_display("a"), "a");
    }
}
