//! Layout rendering for the view module.
//!
//! Contains header, content area, and main content rendering functions.
//! This module handles the top-level layout structure and delegates to
//! view-specific components for detailed rendering.
//!
//! # Layout Structure
//!
//! The application layout follows a three-tier structure:
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │ Header (render_header)                   │  <- 3 rows
//! ├────────────┬─────────────────────────────┤
//! │ Sidebar    │ Main Content                │
//! │ (optional) │ (render_main_content)       │  <- Remaining height
//! │            │                             │
//! ├────────────┴─────────────────────────────┤
//! │ Footer (render_footer in footer.rs)      │  <- 1 row
//! └──────────────────────────────────────────┘
//! ```
//!
//! # Layout Caching
//!
//! This module populates the [`LayoutCache`] for mouse event handling.
//! Each view-specific render function caches its clickable regions:
//!
//! - **Sidebar**: Left panel area (25 columns wide)
//! - **Task List**: Row positions for click-to-select
//! - **Calendar**: Day cell positions for date selection
//! - **Kanban**: Column boundaries for drag-and-drop
//! - **Eisenhower**: Quadrant boundaries
//! - **Weekly Planner**: Day column positions
//! - **Reports**: Tab positions for panel switching
//!
//! The cache is cleared at the start of each render cycle via
//! [`LayoutCache::clear()`] to ensure stale data doesn't persist.
//!
//! # Focus Mode
//!
//! When `model.focus_mode` is enabled, [`render_content`] bypasses the
//! normal sidebar + content layout and renders [`FocusView`] full-screen.
//!
//! [`LayoutCache`]: crate::app::LayoutCache
//! [`FocusView`]: crate::ui::components::FocusView

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
    Burndown, Calendar, Dashboard, Duplicates, Eisenhower, FocusView, Forecast, GitTodos,
    GoalsView, HabitsView, Heatmap, Kanban, Network, ReportsView, Sidebar, TaskList, Timeline,
    WeeklyPlanner,
};

/// Renders the application header with title and border.
///
/// The header displays "TaskFlow - Project Management TUI" in a bordered box.
/// The "TaskFlow" portion is highlighted with the accent color from the theme.
///
/// # Layout
///
/// The header occupies the full width of `area` (typically 3 rows).
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

/// Renders the main content area (sidebar + main content).
///
/// This function orchestrates the content layout based on application state:
///
/// 1. **Focus mode**: If enabled, renders [`FocusView`] full-screen
/// 2. **Sidebar visible**: Splits into 25-column sidebar + remaining content
/// 3. **Sidebar hidden**: Renders content full-width
///
/// # Layout Caching
///
/// Clears the layout cache at the start of each render, then populates:
/// - `sidebar_area`: The sidebar rectangle (if visible)
/// - `main_area`: The main content rectangle
///
/// These cached areas are used by mouse event handlers in [`handle_mouse_event`].
///
/// [`FocusView`]: crate::ui::components::FocusView
/// [`handle_mouse_event`]: crate::app::update::handle_mouse_event
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

/// Renders the main content view based on current view type.
///
/// Dispatches to the appropriate view component based on `model.current_view`:
///
/// | View | Component | Layout Caching |
/// |------|-----------|----------------|
/// | Calendar | [`Calendar`] | Day cells for click selection |
/// | Dashboard | [`Dashboard`] | None |
/// | Reports | [`ReportsView`] | Tab positions |
/// | Habits | [`HabitsView`] | None |
/// | Goals | [`GoalsView`] | None |
/// | Kanban | [`Kanban`] | Column boundaries (4 columns) |
/// | Eisenhower | [`Eisenhower`] | Quadrant boundaries (2×2 grid) |
/// | WeeklyPlanner | [`WeeklyPlanner`] | Day columns (7 columns) |
/// | Timeline | [`Timeline`] | None |
/// | Heatmap | [`Heatmap`] | None |
/// | Forecast | [`Forecast`] | None |
/// | Network | [`Network`] | None |
/// | Burndown | [`Burndown`] | None |
/// | Duplicates | [`Duplicates`] | None |
/// | GitTodos | [`GitTodos`] | None |
/// | Default | [`TaskList`] | Row positions with header offset |
///
/// [`Calendar`]: crate::ui::components::Calendar
/// [`Dashboard`]: crate::ui::components::Dashboard
/// [`ReportsView`]: crate::ui::components::ReportsView
/// [`HabitsView`]: crate::ui::components::HabitsView
/// [`GoalsView`]: crate::ui::components::GoalsView
/// [`Kanban`]: crate::ui::components::Kanban
/// [`Eisenhower`]: crate::ui::components::Eisenhower
/// [`WeeklyPlanner`]: crate::ui::components::WeeklyPlanner
/// [`Timeline`]: crate::ui::components::Timeline
/// [`Heatmap`]: crate::ui::components::Heatmap
/// [`Forecast`]: crate::ui::components::Forecast
/// [`Network`]: crate::ui::components::Network
/// [`Burndown`]: crate::ui::components::Burndown
/// [`Duplicates`]: crate::ui::components::Duplicates
/// [`GitTodos`]: crate::ui::components::GitTodos
/// [`TaskList`]: crate::ui::components::TaskList
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
        ViewId::Goals => {
            let goals = GoalsView::new(model, theme);
            frame.render_widget(goals, area);
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
        ViewId::Duplicates => {
            let duplicates = Duplicates::new(model, theme);
            frame.render_widget(duplicates, area);
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
