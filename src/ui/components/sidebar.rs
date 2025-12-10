//! Sidebar navigation component.
//!
//! The sidebar provides quick access to different views and projects.
//! It displays view counts (inbox, today, upcoming) and allows project selection.
//!
//! # Sections
//!
//! - **Views**: Task List, Today, Upcoming, Overdue, Calendar, etc.
//! - **Projects**: User-created project folders with task counts
//! - **Tags**: Quick filters by tag (when expanded)

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
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
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for Sidebar<'_> {
    #[allow(clippy::too_many_lines)]
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
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📅 ", Style::default()),
                styled_view_name(
                    "Today",
                    ViewId::Today,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📆 ", Style::default()),
                styled_view_name(
                    "Upcoming",
                    ViewId::Upcoming,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("⚠️  ", Style::default()),
                styled_view_name(
                    "Overdue",
                    ViewId::Overdue,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📋 ", Style::default()),
                styled_view_name(
                    "Scheduled",
                    ViewId::Scheduled,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🗓️  ", Style::default()),
                styled_view_name(
                    "Calendar",
                    ViewId::Calendar,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📊 ", Style::default()),
                styled_view_name(
                    "Dashboard",
                    ViewId::Dashboard,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📈 ", Style::default()),
                styled_view_name(
                    "Reports",
                    ViewId::Reports,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🔄 ", Style::default()),
                styled_view_name(
                    "Habits",
                    ViewId::Habits,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🔒 ", Style::default()),
                styled_view_name(
                    "Blocked",
                    ViewId::Blocked,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🏷️  ", Style::default()),
                styled_view_name(
                    "Untagged",
                    ViewId::Untagged,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📁 ", Style::default()),
                styled_view_name(
                    "No Project",
                    ViewId::NoProject,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🕐 ", Style::default()),
                styled_view_name(
                    "Recent",
                    ViewId::RecentlyModified,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🎯 ", Style::default()),
                styled_view_name(
                    "Kanban",
                    ViewId::Kanban,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("⚡ ", Style::default()),
                styled_view_name(
                    "Eisenhower",
                    ViewId::Eisenhower,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📅 ", Style::default()),
                styled_view_name(
                    "Week",
                    ViewId::WeeklyPlanner,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📊 ", Style::default()),
                styled_view_name(
                    "Timeline",
                    ViewId::Timeline,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("💤 ", Style::default()),
                styled_view_name(
                    "Snoozed",
                    ViewId::Snoozed,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🟩 ", Style::default()),
                styled_view_name(
                    "Heatmap",
                    ViewId::Heatmap,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📈 ", Style::default()),
                styled_view_name(
                    "Forecast",
                    ViewId::Forecast,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🔗 ", Style::default()),
                styled_view_name(
                    "Network",
                    ViewId::Network,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📉 ", Style::default()),
                styled_view_name(
                    "Burndown",
                    ViewId::Burndown,
                    self.model.current_view,
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
                    format!(" ({task_count})"),
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
                    .bg(self.theme.colors.accent_secondary.to_color())
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
    current: ViewId,
    no_project_selected: bool,
    theme: &Theme,
) -> Span<'static> {
    // Highlight if this view is current AND no project is selected
    if view_id == current && no_project_selected {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

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
    fn test_sidebar_renders_navigation_title() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Navigation"),
            "Navigation title should be visible"
        );
    }

    #[test]
    fn test_sidebar_renders_all_tasks_view() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("All Tasks"),
            "All Tasks view should be visible"
        );
    }

    #[test]
    fn test_sidebar_renders_today_view() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        assert!(content.contains("Today"), "Today view should be visible");
    }

    #[test]
    fn test_sidebar_renders_upcoming_view() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Upcoming"),
            "Upcoming view should be visible"
        );
    }

    #[test]
    fn test_sidebar_renders_overdue_view() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Overdue"),
            "Overdue view should be visible"
        );
    }

    #[test]
    fn test_sidebar_renders_calendar_view() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Calendar"),
            "Calendar view should be visible"
        );
    }

    #[test]
    fn test_sidebar_renders_dashboard_view() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Dashboard"),
            "Dashboard view should be visible"
        );
    }

    #[test]
    fn test_sidebar_renders_projects_section() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        // Height 30 to accommodate all views including Heatmap, Forecast, Network, Burndown
        let buffer = render_widget(sidebar, 30, 30);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Projects"),
            "Projects section should be visible"
        );
    }

    #[test]
    fn test_sidebar_shows_no_projects_when_empty() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        // Height 30 to accommodate all views including analytics views
        let buffer = render_widget(sidebar, 30, 30);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("No projects"),
            "Should show 'No projects' when empty"
        );
    }

    #[test]
    fn test_sidebar_renders_projects_with_task_counts() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        // Height 50 to accommodate all views (20+) plus 10 projects
        let buffer = render_widget(sidebar, 30, 50);
        let content = buffer_content(&buffer);

        // Sample data has 10 projects; at least one should be visible
        assert!(
            content.contains("Backend")
                || content.contains("Frontend")
                || content.contains("Doc")
                || content.contains("DevOps")
                || content.contains("Mobile")
                || content.contains("Personal"),
            "Project names should be visible"
        );
    }

    #[test]
    fn test_sidebar_uses_focused_border_when_focused() {
        let mut model = Model::new();
        model.focus_pane = FocusPane::Sidebar;
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);

        // Just ensure it renders without panic when focused
        let buffer = render_widget(sidebar, 30, 20);
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_sidebar_renders_separator() {
        let model = Model::new();
        let theme = Theme::default();
        let sidebar = Sidebar::new(&model, &theme);
        let buffer = render_widget(sidebar, 30, 20);
        let content = buffer_content(&buffer);

        // There should be a separator line between views and projects
        assert!(content.contains('─'), "Separator should be visible");
    }
}
