use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::app::{FocusPane, Model, ViewId};
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
        let is_focused = self.model.focus_pane == FocusPane::Sidebar;

        // Navigation views
        let mut items = vec![
            ListItem::new(Line::from(vec![
                Span::styled("📋 ", Style::default()),
                styled_view_name(
                    "All Tasks",
                    ViewId::TaskList,
                    &self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📅 ", Style::default()),
                styled_view_name(
                    "Today",
                    ViewId::Today,
                    &self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📆 ", Style::default()),
                styled_view_name(
                    "Upcoming",
                    ViewId::Upcoming,
                    &self.model.current_view,
                    self.model.selected_project.is_none(),
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

            let is_selected = self.model.selected_project.as_ref() == Some(&project.id);
            let name_style = if is_selected {
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.foreground.to_color())
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled("  📁 ", Style::default()),
                Span::styled(project.name.clone(), name_style),
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

        let border_color = if is_focused {
            theme.colors.accent.to_color()
        } else {
            theme.colors.border.to_color()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Navigation ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        if is_focused {
            let mut state = ListState::default();
            state.select(Some(self.model.sidebar_selected));
            StatefulWidget::render(list, area, buf, &mut state);
        } else {
            Widget::render(list, area, buf);
        }
    }
}

fn styled_view_name(
    name: &str,
    view_id: ViewId,
    current: &ViewId,
    no_project_selected: bool,
    theme: &Theme,
) -> Span<'static> {
    // Highlight if this view is current AND no project is selected
    if view_id == *current && no_project_selected {
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
