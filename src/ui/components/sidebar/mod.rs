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

#[cfg(test)]
mod tests;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
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
                Span::styled("🎯 ", Style::default()),
                styled_view_name(
                    "Goals",
                    ViewId::Goals,
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
            ListItem::new(Line::from(vec![
                Span::styled("🔍 ", Style::default()),
                styled_view_name(
                    "Duplicates",
                    ViewId::Duplicates,
                    self.model.current_view,
                    self.model.selected_project.is_none(),
                    theme,
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📝 ", Style::default()),
                styled_view_name(
                    "Git TODOs",
                    ViewId::GitTodos,
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

        // Contexts section
        items.push(ListItem::new(Line::from("───────────")));
        items.push(ListItem::new(Line::from(Span::styled(
            "Contexts",
            Style::default().fg(theme.colors.muted.to_color()),
        ))));

        let contexts = self.model.all_contexts();
        for context in &contexts {
            let task_count = self
                .model
                .tasks
                .values()
                .filter(|t| t.tags.contains(context))
                .count();

            // Check if this context is currently active (filtered)
            let is_active = self
                .model
                .filtering
                .filter
                .tags
                .as_ref()
                .is_some_and(|tags| tags.len() == 1 && tags.contains(context));

            let name_style = if is_active {
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.foreground.to_color())
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled("  📍 ", Style::default()),
                Span::styled(context.clone(), name_style),
                Span::styled(
                    format!(" ({task_count})"),
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                if is_active {
                    Span::styled(" ✓", Style::default().fg(theme.colors.success.to_color()))
                } else {
                    Span::raw("")
                },
            ])));
        }

        if contexts.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "  Use @tag for contexts",
                Style::default().fg(theme.colors.muted.to_color()),
            ))));
        }

        // Saved Filters section
        items.push(ListItem::new(Line::from("───────────")));
        items.push(ListItem::new(Line::from(Span::styled(
            "Saved Filters",
            Style::default().fg(theme.colors.muted.to_color()),
        ))));

        // List saved filters (sorted by name)
        let mut filters: Vec<_> = self.model.saved_filters.values().collect();
        filters.sort_by(|a, b| a.name.cmp(&b.name));

        for filter in filters {
            let is_active = self.model.active_saved_filter.as_ref() == Some(&filter.id);
            let icon = filter.icon.as_deref().unwrap_or("🔍");
            let name_style = if is_active {
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.foreground.to_color())
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("  {icon} "), Style::default()),
                Span::styled(filter.name.clone(), name_style),
                if is_active {
                    Span::styled(" ✓", Style::default().fg(theme.colors.success.to_color()))
                } else {
                    Span::raw("")
                },
            ])));
        }

        if self.model.saved_filters.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "  Press F to add",
                Style::default().fg(theme.colors.muted.to_color()),
            ))));
        }

        let border_color = if is_focused {
            theme.colors.accent.to_color()
        } else {
            theme.colors.border.to_color()
        };

        let total_items = items.len();

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

        // Use persisted state for scroll offset
        let mut state = self.model.sidebar_list_state.borrow_mut();
        state.select(Some(self.model.sidebar_selected));
        StatefulWidget::render(list, area, buf, &mut state);

        // Render scrollbar if content exceeds viewport
        let viewport_height = area.height.saturating_sub(2) as usize; // Account for borders
        if total_items > viewport_height {
            let mut scrollbar_state = ScrollbarState::new(total_items.saturating_sub(1))
                .position(self.model.sidebar_selected);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .track_style(Style::default().fg(theme.colors.muted.to_color()))
                .thumb_style(Style::default().fg(theme.colors.accent.to_color()));

            StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
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
