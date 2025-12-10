//! Saved filter picker component.
//!
//! Displays a popup for selecting, managing, and creating saved filters (smart lists).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::config::Theme;
use crate::domain::SavedFilter;

/// Saved filter picker popup widget.
pub struct SavedFilterPicker<'a> {
    filters: Vec<&'a SavedFilter>,
    selected: usize,
    active_filter: Option<&'a str>,
    theme: &'a Theme,
}

impl<'a> SavedFilterPicker<'a> {
    #[must_use]
    pub fn new(
        filters: Vec<&'a SavedFilter>,
        selected: usize,
        active_filter: Option<&'a str>,
        theme: &'a Theme,
    ) -> Self {
        Self {
            filters,
            selected,
            active_filter,
            theme,
        }
    }
}

impl Widget for SavedFilterPicker<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        let theme = self.theme;

        // Build list items
        let items: Vec<ListItem<'_>> = self
            .filters
            .iter()
            .enumerate()
            .map(|(i, filter)| {
                let number = if i < 10 {
                    format!("{i}. ")
                } else {
                    "   ".to_string()
                };

                // Icon (if any)
                let icon = filter.icon.as_deref().unwrap_or("🔍");

                // Active indicator
                let is_active = self.active_filter == Some(&filter.name);
                let active_indicator = if is_active { " ✓" } else { "" };

                // Filter description - build a brief summary
                let mut filter_info = Vec::new();

                if filter.filter.search_text.is_some() {
                    filter_info.push("search");
                }
                if filter.filter.status.is_some() {
                    filter_info.push("status");
                }
                if filter.filter.priority.is_some() {
                    filter_info.push("priority");
                }
                if filter.filter.tags.is_some() {
                    filter_info.push("tags");
                }
                if filter.filter.project_id.is_some() {
                    filter_info.push("project");
                }
                if filter.filter.due_before.is_some() || filter.filter.due_after.is_some() {
                    filter_info.push("due date");
                }

                let info = if filter_info.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", filter_info.join(", "))
                };

                ListItem::new(Line::from(vec![
                    Span::styled(number, Style::default().fg(theme.colors.muted.to_color())),
                    Span::styled(format!("{icon} "), Style::default()),
                    Span::styled(
                        &filter.name,
                        if is_active {
                            Style::default()
                                .fg(theme.colors.accent.to_color())
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(info, Style::default().fg(theme.colors.muted.to_color())),
                    Span::styled(
                        active_indicator,
                        Style::default().fg(theme.colors.success.to_color()),
                    ),
                ]))
            })
            .collect();

        let empty_message = if self.filters.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "No saved filters. Press 's' to save current filter.",
                Style::default().fg(theme.colors.muted.to_color()),
            )))]
        } else {
            items
        };

        let list = List::new(empty_message)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Saved Filters (Enter=apply, s=save, d=delete, Esc=cancel) ")
                    .border_style(Style::default().fg(theme.colors.accent.to_color())),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.colors.accent_secondary.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        if !self.filters.is_empty() {
            state.select(Some(self.selected));
        }
        StatefulWidget::render(list, area, buf, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Filter, SortSpec};
    use crate::ui::test_utils::{buffer_content, render_widget};

    fn create_test_filter(name: &str, icon: Option<&str>) -> SavedFilter {
        SavedFilter {
            id: crate::domain::SavedFilterId::new(),
            name: name.to_string(),
            filter: Filter::default(),
            sort: SortSpec::default(),
            icon: icon.map(std::string::ToString::to_string),
        }
    }

    #[test]
    fn test_saved_filter_picker_renders_empty() {
        let theme = Theme::default();
        let picker = SavedFilterPicker::new(vec![], 0, None, &theme);
        let buffer = render_widget(picker, 60, 10);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("No saved filters"),
            "Should show empty message"
        );
    }

    #[test]
    fn test_saved_filter_picker_renders_filters() {
        let theme = Theme::default();
        let filter1 = create_test_filter("Work Tasks", Some("💼"));
        let filter2 = create_test_filter("High Priority", Some("🔥"));
        let filters: Vec<&SavedFilter> = vec![&filter1, &filter2];

        let picker = SavedFilterPicker::new(filters, 0, None, &theme);
        let buffer = render_widget(picker, 60, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("Work Tasks"), "Should show filter name");
        assert!(
            content.contains("High Priority"),
            "Should show second filter"
        );
    }

    #[test]
    fn test_saved_filter_picker_shows_active_indicator() {
        let theme = Theme::default();
        let filter1 = create_test_filter("Active Filter", None);
        let filters: Vec<&SavedFilter> = vec![&filter1];

        let picker = SavedFilterPicker::new(filters, 0, Some("Active Filter"), &theme);
        let buffer = render_widget(picker, 60, 10);
        let content = buffer_content(&buffer);

        assert!(content.contains("✓"), "Should show active indicator");
    }

    #[test]
    fn test_saved_filter_picker_shows_title() {
        let theme = Theme::default();
        let picker = SavedFilterPicker::new(vec![], 0, None, &theme);
        let buffer = render_widget(picker, 80, 10);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Saved Filters"),
            "Should show title in border"
        );
    }

    #[test]
    fn test_saved_filter_picker_shows_instructions() {
        let theme = Theme::default();
        let picker = SavedFilterPicker::new(vec![], 0, None, &theme);
        let buffer = render_widget(picker, 80, 10);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Enter") || content.contains("apply"),
            "Should show apply instruction"
        );
    }

    #[test]
    fn test_saved_filter_picker_with_filter_criteria() {
        let theme = Theme::default();
        let mut filter = create_test_filter("Complex Filter", None);
        filter.filter.search_text = Some("test".to_string());
        filter.filter.tags = Some(vec!["work".to_string()]);

        let filters: Vec<&SavedFilter> = vec![&filter];
        let picker = SavedFilterPicker::new(filters, 0, None, &theme);
        let buffer = render_widget(picker, 80, 10);
        let content = buffer_content(&buffer);

        // Should show some indication of filter criteria
        assert!(
            content.contains("search") || content.contains("tags"),
            "Should show filter criteria"
        );
    }
}
