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
    #[must_use]
    pub const fn new(templates: &'a TemplateManager, selected: usize, theme: &'a Theme) -> Self {
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
                    format!("{i}. ")
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
                            format!(" (+{d} days)")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{TaskTemplate, TemplateManager};
    use crate::config::Theme;
    use crate::domain::Priority;

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

    fn create_test_template_manager() -> TemplateManager {
        let mut manager = TemplateManager::new();
        manager.templates.push(TaskTemplate {
            name: "Bug Fix".to_string(),
            title: "Fix bug".to_string(),
            priority: Priority::High,
            tags: vec!["bug".to_string(), "fix".to_string()],
            description: None,
            due_days: Some(1),
        });
        manager.templates.push(TaskTemplate {
            name: "Feature".to_string(),
            title: "New feature".to_string(),
            priority: Priority::Medium,
            tags: vec!["feature".to_string()],
            description: None,
            due_days: Some(7),
        });
        manager.templates.push(TaskTemplate {
            name: "Documentation".to_string(),
            title: "Update docs".to_string(),
            priority: Priority::Low,
            tags: vec!["docs".to_string()],
            description: None,
            due_days: None,
        });
        manager
    }

    #[test]
    fn test_template_picker_renders_title() {
        let manager = create_test_template_manager();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 15);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Template") || content.contains("New Task"),
            "Title should be visible"
        );
    }

    #[test]
    fn test_template_picker_renders_template_names() {
        let manager = create_test_template_manager();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 15);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Bug Fix"),
            "Bug Fix template should be visible"
        );
        assert!(
            content.contains("Feature"),
            "Feature template should be visible"
        );
        assert!(
            content.contains("Documentation"),
            "Documentation template should be visible"
        );
    }

    #[test]
    fn test_template_picker_renders_priority_indicators() {
        let manager = create_test_template_manager();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 15);
        let content = buffer_content(&buffer);

        // Should show priority indicators (!!!, !!, !)
        assert!(
            content.contains('!'),
            "Priority indicators should be visible"
        );
    }

    #[test]
    fn test_template_picker_renders_tags() {
        let manager = create_test_template_manager();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 80, 15);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("#bug") || content.contains("#feature") || content.contains("#docs"),
            "Tags should be visible"
        );
    }

    #[test]
    fn test_template_picker_renders_due_info() {
        let manager = create_test_template_manager();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 80, 15);
        let content = buffer_content(&buffer);

        // Should show due day info like "(tomorrow)" or "(+7 days)"
        assert!(
            content.contains("tomorrow") || content.contains("days"),
            "Due date info should be visible"
        );
    }

    #[test]
    fn test_template_picker_renders_number_prefix() {
        let manager = create_test_template_manager();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 15);
        let content = buffer_content(&buffer);

        // Templates should be numbered 0-9 for quick selection
        assert!(
            content.contains("0."),
            "First template should be numbered 0"
        );
        assert!(
            content.contains("1."),
            "Second template should be numbered 1"
        );
    }

    #[test]
    fn test_template_picker_with_selection() {
        let manager = create_test_template_manager();
        let theme = Theme::default();

        // Test with different selections
        for selected in 0..3 {
            let picker = TemplatePicker::new(&manager, selected, &theme);
            let buffer = render_widget(picker, 60, 15);
            // Should render without panic
            let _ = buffer_content(&buffer);
        }
    }

    #[test]
    fn test_template_picker_empty_templates() {
        let manager = TemplateManager::new();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 15);

        // Should render without panic even with empty templates
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_template_picker_renders_instructions() {
        let manager = create_test_template_manager();
        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 80, 15);
        let content = buffer_content(&buffer);

        // Should show instructions about how to select
        assert!(
            content.contains("Enter") || content.contains("select") || content.contains("Esc"),
            "Instructions should be visible"
        );
    }

    #[test]
    fn test_template_picker_urgent_priority() {
        let mut manager = TemplateManager::new();
        manager.templates.push(TaskTemplate {
            name: "Urgent Task".to_string(),
            title: "Urgent".to_string(),
            priority: Priority::Urgent,
            tags: vec![],
            description: None,
            due_days: Some(0),
        });

        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 15);
        let content = buffer_content(&buffer);

        // Urgent priority shows "!!!!"
        assert!(content.contains("!!!!"), "Urgent priority should show !!!!");
    }

    #[test]
    fn test_template_picker_today_due() {
        let mut manager = TemplateManager::new();
        manager.templates.push(TaskTemplate {
            name: "Today Task".to_string(),
            title: "Today".to_string(),
            priority: Priority::None,
            tags: vec![],
            description: None,
            due_days: Some(0),
        });

        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 15);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("today"),
            "Should show 'today' for due_days=0"
        );
    }
}
