//! Command palette component for searching and executing commands.
//!
//! Provides a searchable list of all available actions, similar to
//! VS Code's Ctrl+Shift+P command palette.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use crate::config::{Action, Keybindings, Theme, ALL_ACTIONS};

/// Command entry for display in the palette.
pub struct CommandEntry {
    /// The action this entry represents
    pub action: Action,
    /// Human-readable description
    pub description: &'static str,
    /// Keybinding string (if any)
    pub keybinding: Option<String>,
}

/// Command palette widget.
///
/// Displays a searchable list of all available commands with their
/// keybindings. Users can filter by typing and execute by pressing Enter.
pub struct CommandPalette<'a> {
    query: &'a str,
    cursor: usize,
    selected: usize,
    keybindings: &'a Keybindings,
    theme: &'a Theme,
}

impl<'a> CommandPalette<'a> {
    /// Create a new command palette widget.
    #[must_use]
    pub const fn new(
        query: &'a str,
        cursor: usize,
        selected: usize,
        keybindings: &'a Keybindings,
        theme: &'a Theme,
    ) -> Self {
        Self {
            query,
            cursor,
            selected,
            keybindings,
            theme,
        }
    }

    /// Filter commands based on the current query.
    fn filter_commands(&self) -> Vec<CommandEntry> {
        let query_lower = self.query.to_lowercase();

        ALL_ACTIONS
            .iter()
            .filter(|action| {
                // Filter out the command palette action itself
                if matches!(action, Action::ShowCommandPalette) {
                    return false;
                }

                let description = action.description().to_lowercase();
                let action_name = format!("{action:?}").to_lowercase();

                self.query.is_empty()
                    || description.contains(&query_lower)
                    || action_name.contains(&query_lower)
            })
            .map(|action| CommandEntry {
                action: action.clone(),
                description: action.description(),
                keybinding: self.keybindings.key_for_action(action).cloned(),
            })
            .collect()
    }

    /// Get the number of filtered commands.
    #[must_use]
    pub fn filtered_count(&self) -> usize {
        self.filter_commands().len()
    }

    /// Get the selected action (if any).
    #[must_use]
    pub fn selected_action(&self) -> Option<Action> {
        self.filter_commands()
            .get(self.selected)
            .map(|c| c.action.clone())
    }
}

impl Widget for CommandPalette<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the background
        Clear.render(area, buf);

        let block = Block::default()
            .title(" Command Palette ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 3 || inner.width < 20 {
            return;
        }

        // Layout: search input + separator + command list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Search input
                Constraint::Length(1), // Separator
                Constraint::Min(1),    // Command list
            ])
            .split(inner);

        // Render search input with cursor
        let display_query = if self.cursor >= self.query.len() {
            format!("{}|", self.query)
        } else {
            let (before, after) = self.query.split_at(self.cursor);
            format!("{before}|{after}")
        };
        let search_line = Line::from(vec![
            Span::styled(
                "> ",
                Style::default().fg(self.theme.colors.accent.to_color()),
            ),
            Span::styled(
                display_query,
                Style::default().fg(self.theme.colors.foreground.to_color()),
            ),
        ]);
        Paragraph::new(search_line).render(chunks[0], buf);

        // Render separator
        let sep = "\u{2500}".repeat(inner.width as usize);
        Paragraph::new(sep)
            .style(Style::default().fg(self.theme.colors.border.to_color()))
            .render(chunks[1], buf);

        // Render command list
        let commands = self.filter_commands();
        let visible_height = chunks[2].height as usize;

        // Calculate scroll offset to keep selection visible
        let scroll_offset = if self.selected >= visible_height {
            self.selected - visible_height + 1
        } else {
            0
        };

        let items: Vec<ListItem<'_>> = commands
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(i, cmd)| {
                let is_selected = i == self.selected;

                // Build line: description + keybinding on right
                let kb_str = cmd.keybinding.as_deref().unwrap_or("");
                let available_width = (inner.width as usize).saturating_sub(4);
                let kb_width = kb_str.len();
                let desc_width = available_width.saturating_sub(kb_width + 2);

                let description = if cmd.description.len() > desc_width {
                    format!("{}...", &cmd.description[..desc_width.saturating_sub(3)])
                } else {
                    cmd.description.to_string()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{description:<desc_width$}"),
                        Style::default().fg(self.theme.colors.foreground.to_color()),
                    ),
                    Span::styled(
                        format!("  {kb_str}"),
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                ]);

                let style = if is_selected {
                    Style::default()
                        .bg(self.theme.colors.accent_secondary.to_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        if items.is_empty() {
            // Show "no matches" message
            let no_match = Line::from(Span::styled(
                "No matching commands",
                Style::default()
                    .fg(self.theme.colors.muted.to_color())
                    .add_modifier(Modifier::ITALIC),
            ));
            Paragraph::new(no_match).render(chunks[2], buf);
        } else {
            let list = List::new(items);
            Widget::render(list, chunks[2], buf);
        }
    }
}

/// Get the filtered command count for a given query.
///
/// This is used by the update handler to clamp selection.
#[must_use]
pub fn get_filtered_count(query: &str) -> usize {
    let query_lower = query.to_lowercase();

    ALL_ACTIONS
        .iter()
        .filter(|action| {
            if matches!(action, Action::ShowCommandPalette) {
                return false;
            }

            let description = action.description().to_lowercase();
            let action_name = format!("{action:?}").to_lowercase();

            query.is_empty()
                || description.contains(&query_lower)
                || action_name.contains(&query_lower)
        })
        .count()
}

/// Get the selected action for a given query and selection index.
///
/// This is used by the update handler to execute the selected command.
#[must_use]
pub fn get_selected_action(query: &str, selected: usize) -> Option<Action> {
    let query_lower = query.to_lowercase();

    ALL_ACTIONS
        .iter()
        .filter(|action| {
            if matches!(action, Action::ShowCommandPalette) {
                return false;
            }

            let description = action.description().to_lowercase();
            let action_name = format!("{action:?}").to_lowercase();

            query.is_empty()
                || description.contains(&query_lower)
                || action_name.contains(&query_lower)
        })
        .nth(selected)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_filtered_count_empty_query() {
        let count = get_filtered_count("");
        // Should return all actions except ShowCommandPalette
        assert!(count > 0);
        assert_eq!(count, ALL_ACTIONS.len() - 1);
    }

    #[test]
    fn test_get_filtered_count_with_query() {
        let count = get_filtered_count("task");
        // Should return actions containing "task"
        assert!(count > 0);
        assert!(count < ALL_ACTIONS.len());
    }

    #[test]
    fn test_get_selected_action_empty_query() {
        let action = get_selected_action("", 0);
        assert!(action.is_some());
    }

    #[test]
    fn test_get_selected_action_with_query() {
        let action = get_selected_action("help", 0);
        // Should find ShowHelp action
        assert!(action.is_some());
        if let Some(a) = action {
            assert!(a.description().to_lowercase().contains("help"));
        }
    }

    #[test]
    fn test_get_selected_action_out_of_bounds() {
        let action = get_selected_action("", 9999);
        assert!(action.is_none());
    }
}
