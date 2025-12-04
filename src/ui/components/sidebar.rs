use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

use crate::app::{Model, ViewId};
use crate::config::Theme;

/// Sidebar widget showing navigation views and projects
pub struct Sidebar<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Sidebar<'a> {
    pub fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for Sidebar<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = self.theme;

        // Navigation views
        let mut items = vec![
            ListItem::new(Line::from(vec![
                Span::styled("📋 ", Style::default()),
                styled_view_name(
                    "All Tasks",
                    ViewId::TaskList,
                    &self.model.current_view,
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📅 ", Style::default()),
                styled_view_name("Today", ViewId::Today, &self.model.current_view, theme),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📆 ", Style::default()),
                styled_view_name(
                    "Upcoming",
                    ViewId::Upcoming,
                    &self.model.current_view,
                    theme,
                ),
            ])),
            // Separator
            ListItem::new(Line::from("───────────")),
            // Projects section
            ListItem::new(Line::from(Span::styled(
                "Projects",
                Style::default().fg(theme.colors.muted.to_color()),
            ))),
        ];

        // List projects
        for project in self.model.projects.values() {
            let task_count = self
                .model
                .tasks
                .values()
                .filter(|t| t.project_id.as_ref() == Some(&project.id))
                .count();

            items.push(ListItem::new(Line::from(vec![
                Span::styled("  📁 ", Style::default()),
                Span::styled(
                    &project.name,
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
                Span::styled(
                    format!(" ({})", task_count),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
            ])));
        }

        if self.model.projects.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "  No projects",
                Style::default().fg(theme.colors.muted.to_color()),
            ))));
        }

        let list = List::new(items).block(
            Block::default()
                .title(" Navigation ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.border.to_color())),
        );

        list.render(area, buf);
    }
}

fn styled_view_name(name: &str, view_id: ViewId, current: &ViewId, theme: &Theme) -> Span<'static> {
    if view_id == *current {
        Span::styled(
            name.to_string(),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            name.to_string(),
            Style::default().fg(theme.colors.foreground.to_color()),
        )
    }
}
