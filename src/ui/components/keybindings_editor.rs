use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::config::{Action, Keybindings, Theme};

/// Keybindings editor popup widget
pub struct KeybindingsEditor<'a> {
    keybindings: &'a Keybindings,
    selected: usize,
    capturing: bool,
    theme: &'a Theme,
    /// Conflict message to display (e.g., "Key 'j' is bound to MoveDown")
    conflict_message: Option<&'a str>,
    /// The conflicting action, if any
    conflict_action: Option<&'a Action>,
}

impl<'a> KeybindingsEditor<'a> {
    #[must_use]
    pub const fn new(
        keybindings: &'a Keybindings,
        selected: usize,
        capturing: bool,
        theme: &'a Theme,
    ) -> Self {
        Self {
            keybindings,
            selected,
            capturing,
            theme,
            conflict_message: None,
            conflict_action: None,
        }
    }

    /// Set a conflict message to display during key capture
    #[must_use]
    pub const fn with_conflict(mut self, message: &'a str, action: &'a Action) -> Self {
        self.conflict_message = Some(message);
        self.conflict_action = Some(action);
        self
    }
}

impl Widget for KeybindingsEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        let theme = self.theme;
        let bindings = self.keybindings.sorted_bindings();

        // Build list items
        let items: Vec<ListItem> = bindings
            .iter()
            .map(|(key, action)| {
                let key_display = format!("{:>12}", key);
                let action_display = format!("{:?}", action);

                ListItem::new(Line::from(vec![
                    Span::styled(
                        key_display,
                        Style::default()
                            .fg(theme.colors.accent.to_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        action_display,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                ]))
            })
            .collect();

        let title = if self.capturing {
            if let Some(msg) = self.conflict_message {
                // Show conflict warning in red
                format!(" Keybindings - {} (Enter=overwrite, Esc=cancel) ", msg)
            } else {
                " Keybindings - Press a key combination (Esc to cancel) ".to_string()
            }
        } else {
            " Keybindings (Enter=edit, r=reset, R=reset all, s=save, Esc=close) ".to_string()
        };

        let border_color = if self.conflict_message.is_some() {
            Color::Red
        } else if self.capturing {
            Color::Yellow
        } else {
            theme.colors.accent.to_color()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(self.selected));
        StatefulWidget::render(list, area, buf, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Keybindings, Theme};

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

    #[test]
    fn test_keybindings_editor_renders_title() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();
        let editor = KeybindingsEditor::new(&keybindings, 0, false, &theme);
        let buffer = render_widget(editor, 80, 20);
        let content = buffer_content(&buffer);

        assert!(content.contains("Keybindings"), "Title should be visible");
    }

    #[test]
    fn test_keybindings_editor_renders_keys() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();
        let editor = KeybindingsEditor::new(&keybindings, 0, false, &theme);
        let buffer = render_widget(editor, 80, 40);
        let content = buffer_content(&buffer);

        // Default keybindings should include common keys
        assert!(
            content.contains("j") || content.contains("k"),
            "Common navigation keys should be visible"
        );
    }

    #[test]
    fn test_keybindings_editor_renders_actions() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();
        // Use a larger buffer to see more keybindings
        let editor = KeybindingsEditor::new(&keybindings, 0, false, &theme);
        let buffer = render_widget(editor, 80, 80);
        let content = buffer_content(&buffer);

        // The sorted bindings start with special chars like #, <, >, ?, @, then letters
        // Check for any action name that should be visible within the first 80 rows
        assert!(
            content.contains("FilterByTag") // # is first in sort order
                || content.contains("CalendarPrevMonth") // < comes early
                || content.contains("ShowHelp"), // ? comes early
            "Action names should be visible"
        );
    }

    #[test]
    fn test_keybindings_editor_capturing_mode() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();
        let editor = KeybindingsEditor::new(&keybindings, 0, true, &theme);
        let buffer = render_widget(editor, 80, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Press a key"),
            "Capturing mode should show instruction"
        );
    }

    #[test]
    fn test_keybindings_editor_with_selection() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        // Test with different selections
        for selected in 0..5 {
            let editor = KeybindingsEditor::new(&keybindings, selected, false, &theme);
            let buffer = render_widget(editor, 80, 40);
            // Should render without panic
            let _ = buffer_content(&buffer);
        }
    }

    #[test]
    fn test_keybindings_editor_instructions_visible() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();
        let editor = KeybindingsEditor::new(&keybindings, 0, false, &theme);
        let buffer = render_widget(editor, 80, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Enter") || content.contains("edit") || content.contains("save"),
            "Instructions should be visible"
        );
    }

    #[test]
    fn test_keybindings_editor_conflict_display() {
        use crate::config::Action;

        let keybindings = Keybindings::default();
        let theme = Theme::default();
        let action = Action::MoveDown;
        let editor = KeybindingsEditor::new(&keybindings, 0, true, &theme)
            .with_conflict("Key bound to MoveDown", &action);
        let buffer = render_widget(editor, 100, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("overwrite") || content.contains("MoveDown"),
            "Conflict message should be visible: {}",
            content
        );
    }
}
