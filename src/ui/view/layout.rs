//! Layout rendering for the view module.
//!
//! Contains header, content area, and main content rendering functions.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{Model, ViewId};
use crate::config::Theme;

use crate::ui::components::{
    Burndown, Calendar, Dashboard, Eisenhower, FocusView, Forecast, GitTodos, HabitsView, Heatmap,
    Kanban, Network, ReportsView, Sidebar, TaskList, Timeline, WeeklyPlanner,
};

/// Renders the application header
pub(super) fn render_header(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " TaskFlow ",
            Style::default().fg(theme.colors.accent.to_color()),
        ),
        Span::raw("- Project Management TUI"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color())),
    );

    frame.render_widget(title, area);
}

/// Renders the main content area (sidebar + main content)
pub(super) fn render_content(model: &Model, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    // Clear layout cache at start of render
    model.layout_cache.clear();

    // Focus mode takes over the entire content area
    if model.focus_mode {
        let focus_view = FocusView::new(model, theme);
        frame.render_widget(focus_view, area);
        return;
    }

    if model.show_sidebar {
        // Split into sidebar and main content
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25), // Sidebar
                Constraint::Min(0),     // Main content
            ])
            .split(area);

        // Cache sidebar area for mouse hit-testing
        model.layout_cache.set_sidebar_area(chunks[0]);

        // Render sidebar
        frame.render_widget(Sidebar::new(model, theme), chunks[0]);

        // Cache main area and render content
        model.layout_cache.set_main_area(chunks[1]);
        render_main_content(model, frame, chunks[1], theme);
    } else {
        // No sidebar, full width content
        model.layout_cache.set_main_area(area);
        render_main_content(model, frame, area, theme);
    }
}

/// Renders the main content view based on current view type
pub(super) fn render_main_content(model: &Model, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    match model.current_view {
        ViewId::Calendar => {
            // Cache calendar area for mouse click detection
            // The calendar grid typically has a 2-row header (month title + weekday headers)
            model.layout_cache.set_calendar_area(area);
            let calendar = Calendar::new(model, theme);
            frame.render_widget(calendar, area);
        }
        ViewId::Dashboard => {
            let dashboard = Dashboard::new(model, theme);
            frame.render_widget(dashboard, area);
        }
        ViewId::Reports => {
            // Cache reports tabs area - inner area after border, 3 rows height for tabs
            // The reports view has a 1-row border, then 3 rows for tabs
            let inner_x = area.x + 1;
            let inner_y = area.y + 1;
            let inner_width = area.width.saturating_sub(2);
            let tabs_area = Rect::new(inner_x, inner_y, inner_width, 3);
            model.layout_cache.set_reports_tabs_area(tabs_area);

            // Cache individual tab positions for precise click detection
            // Tab labels: "Overview", "Velocity", "Tags", "Time", "Focus", "Insights", "Estimation"
            // Divider: " | " (3 chars)
            const TAB_WIDTHS: [u16; 7] = [8, 8, 4, 4, 5, 8, 10]; // Character widths
            const DIVIDER_WIDTH: u16 = 3;
            let mut x_pos = inner_x;
            for (i, &width) in TAB_WIDTHS.iter().enumerate() {
                let tab_rect = Rect::new(x_pos, inner_y, width, 3);
                model.layout_cache.set_reports_tab_rect(i, tab_rect);
                x_pos += width;
                if i < 6 {
                    x_pos += DIVIDER_WIDTH; // Add divider width except after last tab
                }
            }

            let reports = ReportsView::new(model, model.report_panel, theme);
            frame.render_widget(reports, area);
        }
        ViewId::Habits => {
            let habits = HabitsView::new(model, theme);
            frame.render_widget(habits, area);
        }
        ViewId::Kanban => {
            // Cache kanban column areas - divide into 4 equal columns
            let column_width = area.width / 4;
            for i in 0..4 {
                let col_area = Rect {
                    x: area.x + (i as u16 * column_width),
                    y: area.y,
                    width: column_width,
                    height: area.height,
                };
                model.layout_cache.set_kanban_column(i, col_area);
            }
            let kanban = Kanban::new(model, theme);
            frame.render_widget(kanban, area);
        }
        ViewId::Eisenhower => {
            // Cache eisenhower quadrant areas - 2x2 grid
            let half_width = area.width / 2;
            let half_height = area.height / 2;
            // Top-left (0), Top-right (1), Bottom-left (2), Bottom-right (3)
            model
                .layout_cache
                .set_eisenhower_quadrant(0, Rect::new(area.x, area.y, half_width, half_height));
            model.layout_cache.set_eisenhower_quadrant(
                1,
                Rect::new(area.x + half_width, area.y, half_width, half_height),
            );
            model.layout_cache.set_eisenhower_quadrant(
                2,
                Rect::new(area.x, area.y + half_height, half_width, half_height),
            );
            model.layout_cache.set_eisenhower_quadrant(
                3,
                Rect::new(
                    area.x + half_width,
                    area.y + half_height,
                    half_width,
                    half_height,
                ),
            );
            let eisenhower = Eisenhower::new(model, theme);
            frame.render_widget(eisenhower, area);
        }
        ViewId::WeeklyPlanner => {
            // Cache weekly planner day areas - 7 columns
            let day_width = area.width / 7;
            for i in 0..7 {
                let day_area = Rect {
                    x: area.x + (i as u16 * day_width),
                    y: area.y,
                    width: day_width,
                    height: area.height,
                };
                model.layout_cache.set_weekly_planner_day(i, day_area);
            }
            let planner = WeeklyPlanner::new(model, theme);
            frame.render_widget(planner, area);
        }
        ViewId::Timeline => {
            let timeline = Timeline::new(model, theme);
            frame.render_widget(timeline, area);
        }
        ViewId::Heatmap => {
            let heatmap = Heatmap::new(model, theme);
            frame.render_widget(heatmap, area);
        }
        ViewId::Forecast => {
            let forecast = Forecast::new(model, theme);
            frame.render_widget(forecast, area);
        }
        ViewId::Network => {
            let network = Network::new(model, theme, model.view_selection.network_task_index);
            frame.render_widget(network, area);
        }
        ViewId::Burndown => {
            let burndown = Burndown::new(model, theme);
            frame.render_widget(burndown, area);
        }
        ViewId::GitTodos => {
            let git_todos = GitTodos::new(model, theme);
            frame.render_widget(git_todos, area);
        }
        _ => {
            // Cache task list area with header offset (border + title row = 2)
            // scroll_offset is typically 0 unless we implement virtual scrolling
            model.layout_cache.set_task_list_area(
                area,
                2,
                model
                    .selected_index
                    .saturating_sub(area.height as usize / 2),
            );
            let task_list = TaskList::new(model, theme);
            frame.render_widget(task_list, area);
        }
    }
}
