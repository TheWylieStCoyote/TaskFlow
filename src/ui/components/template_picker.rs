use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::app::TemplateManager;
use crate::config::Theme;

/// Template picker popup widget
pub struct TemplatePicker<'a> {
    templates: &'a TemplateManager,
    selected: usize,
    theme: &'a Theme,
}

impl<'a> TemplatePicker<'a> {
    pub fn new(templates: &'a TemplateManager, selected: usize, theme: &'a Theme) -> Self {
        Self {
            templates,
            selected,
            theme,
        }
    }
}

impl Widget for TemplatePicker<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        let theme = self.theme;

        // Build list items
        let items: Vec<ListItem> = self
            .templates
            .templates
            .iter()
            .enumerate()
            .map(|(i, template)| {
                let number = if i < 10 {
                    format!("{}. ", i)
                } else {
                    "   ".to_string()
                };

                let priority_indicator = match template.priority {
                    crate::domain::Priority::Urgent => "!!!!",
                    crate::domain::Priority::High => "!!! ",
                    crate::domain::Priority::Medium => "!!  ",
                    crate::domain::Priority::Low => "!   ",
                    crate::domain::Priority::None => "    ",
                };

                let tags = if template.tags.is_empty() {
                    String::new()
                } else {
                    format!(" #{}", template.tags.join(" #"))
                };

                let due_info = template
                    .due_days
                    .map(|d| {
                        if d == 0 {
                            " (today)".to_string()
                        } else if d == 1 {
                            " (tomorrow)".to_string()
                        } else {
                            format!(" (+{} days)", d)
                        }
                    })
                    .unwrap_or_default();

                ListItem::new(Line::from(vec![
                    Span::styled(number, Style::default().fg(theme.colors.muted.to_color())),
                    Span::styled(
                        priority_indicator,
                        Style::default().fg(match template.priority {
                            crate::domain::Priority::Urgent => theme.priority.urgent.to_color(),
                            crate::domain::Priority::High => theme.priority.high.to_color(),
                            crate::domain::Priority::Medium => theme.priority.medium.to_color(),
                            crate::domain::Priority::Low => theme.priority.low.to_color(),
                            crate::domain::Priority::None => theme.colors.muted.to_color(),
                        }),
                    ),
                    Span::styled(&template.name, Style::default()),
                    Span::styled(tags, Style::default().fg(theme.colors.accent.to_color())),
                    Span::styled(due_info, Style::default().fg(theme.colors.muted.to_color())),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" New Task from Template (0-9 or Enter to select, Esc to cancel) ")
                    .border_style(Style::default().fg(theme.colors.accent.to_color())),
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
