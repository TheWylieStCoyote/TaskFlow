//! Mouse event handling.
//!
//! This module provides mouse input handling for the TaskFlow TUI,
//! mapping click coordinates to navigation messages.

use chrono::Datelike;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use taskflow::app::{FocusPane, LayoutCache, Message, Model, NavigationMessage, ViewId};

/// Handle a mouse event and return the appropriate message.
pub fn handle_mouse_event(event: MouseEvent, model: &Model) -> Message {
    let x = event.column;
    let y = event.row;

    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => handle_left_click(x, y, model),
        MouseEventKind::ScrollUp => handle_scroll_up(model),
        MouseEventKind::ScrollDown => handle_scroll_down(model),
        _ => Message::None,
    }
}

/// Handle left mouse button click.
fn handle_left_click(x: u16, y: u16, model: &Model) -> Message {
    // Check for double-click first
    let is_double = model.layout_cache.is_double_click(x, y);
    model.layout_cache.record_click(x, y);

    // Check sidebar click (if visible)
    if model.show_sidebar {
        if let Some(sidebar_area) = model.layout_cache.sidebar_area() {
            if LayoutCache::is_in_rect(x, y, sidebar_area) {
                return handle_sidebar_click(x, y, sidebar_area, model);
            }
        }
    }

    // Check main area click based on current view
    if let Some(main_area) = model.layout_cache.main_area() {
        if LayoutCache::is_in_rect(x, y, main_area) {
            return handle_main_area_click(x, y, is_double, model);
        }
    }

    Message::None
}

/// Handle click in the sidebar area.
fn handle_sidebar_click(
    _x: u16,
    y: u16,
    sidebar_area: ratatui::layout::Rect,
    model: &Model,
) -> Message {
    // Account for border (1 row at top)
    let relative_y = y.saturating_sub(sidebar_area.y + 1);
    let item_index = relative_y as usize;

    // Check if within valid sidebar items
    let total_items = model.sidebar_item_count();
    if item_index < total_items {
        // First update sidebar selection, then select the item
        Message::Navigation(NavigationMessage::SidebarSelectIndex(item_index))
    } else {
        Message::None
    }
}

/// Handle click in the main content area.
fn handle_main_area_click(x: u16, y: u16, is_double: bool, model: &Model) -> Message {
    match model.current_view {
        ViewId::Calendar => handle_calendar_click(x, y, model),
        ViewId::Kanban => handle_kanban_click(x, y, model),
        ViewId::Eisenhower => handle_eisenhower_click(x, y, model),
        ViewId::WeeklyPlanner => handle_weekly_planner_click(x, y, model),
        ViewId::Reports => handle_reports_click(x, y, model),
        _ => handle_task_list_click(y, is_double, model),
    }
}

/// Handle click in a task list view.
fn handle_task_list_click(y: u16, is_double: bool, model: &Model) -> Message {
    if let Some(task_area) = model.layout_cache.task_list_area() {
        // Calculate relative position accounting for header
        let header_offset = model.layout_cache.task_list_header_offset();
        let relative_y = y.saturating_sub(task_area.y + header_offset);

        // Account for scroll offset
        let task_index = model.layout_cache.scroll_offset() + relative_y as usize;

        if task_index < model.visible_tasks.len() {
            // Focus task list pane if not already focused
            if model.focus_pane != FocusPane::TaskList {
                return Message::Navigation(NavigationMessage::FocusTaskList);
            }

            if is_double {
                // Double-click to edit - first select, then edit
                // We need to select first, so return select and let next click trigger edit
                return Message::Navigation(NavigationMessage::Select(task_index));
            }
            return Message::Navigation(NavigationMessage::Select(task_index));
        }
    }
    Message::None
}

/// Handle click in the calendar view.
fn handle_calendar_click(x: u16, y: u16, model: &Model) -> Message {
    // Check if clicking in the calendar grid area
    if let Some(calendar_area) = model.layout_cache.calendar_area() {
        if LayoutCache::is_in_rect(x, y, calendar_area) {
            // Calendar grid: 7 columns (days), typically 6 rows (weeks)
            // Plus 2 rows for month header and day-of-week header
            let header_rows = 2u16;
            let content_y = y.saturating_sub(calendar_area.y + header_rows);
            let content_x = x.saturating_sub(calendar_area.x);

            let cell_width = calendar_area.width / 7;
            let cell_height = calendar_area.height.saturating_sub(header_rows) / 6;

            if cell_width > 0 && cell_height > 0 {
                let col = content_x / cell_width;
                let row = content_y / cell_height;

                // Calculate the day number based on the calendar layout
                // This is simplified - actual calculation depends on month start day
                let day_index = row as u32 * 7 + col as u32;

                // Get first day of month offset (0 = Monday, 6 = Sunday)
                if let Some(first_date) = chrono::NaiveDate::from_ymd_opt(
                    model.calendar_state.year,
                    model.calendar_state.month,
                    1,
                ) {
                    let first_weekday = first_date.weekday().num_days_from_monday();
                    if day_index >= first_weekday {
                        let day = day_index - first_weekday + 1;
                        let days_in_month =
                            days_in_month(model.calendar_state.year, model.calendar_state.month);
                        if day >= 1 && day <= days_in_month {
                            return Message::Navigation(NavigationMessage::CalendarSelectDay(day));
                        }
                    }
                }
            }
        }
    }

    // If not in calendar grid, check task list area
    handle_task_list_click(y, false, model)
}

/// Handle click in the Kanban view.
fn handle_kanban_click(x: u16, y: u16, model: &Model) -> Message {
    // Check which column was clicked
    for (col_idx, col_area) in model.layout_cache.kanban_columns().iter().enumerate() {
        if let Some(area) = col_area {
            if LayoutCache::is_in_rect(x, y, *area) {
                // Clicked in this column
                let current_col = model.view_selection.kanban_column;

                if col_idx != current_col {
                    // Switch to this column
                    return match col_idx {
                        0 => Message::Navigation(NavigationMessage::KanbanSelectColumn(0)),
                        1 => Message::Navigation(NavigationMessage::KanbanSelectColumn(1)),
                        2 => Message::Navigation(NavigationMessage::KanbanSelectColumn(2)),
                        3 => Message::Navigation(NavigationMessage::KanbanSelectColumn(3)),
                        _ => Message::None,
                    };
                }
                // Already in this column - could select task by y position
                // For now, just ensure focus
                return Message::Navigation(NavigationMessage::FocusTaskList);
            }
        }
    }
    Message::None
}

/// Handle click in the Eisenhower matrix view.
fn handle_eisenhower_click(x: u16, y: u16, model: &Model) -> Message {
    // Check which quadrant was clicked
    for (quad_idx, quad_area) in model.layout_cache.eisenhower_quadrants().iter().enumerate() {
        if let Some(area) = quad_area {
            if LayoutCache::is_in_rect(x, y, *area) {
                let current_quad = model.view_selection.eisenhower_quadrant;

                if quad_idx != current_quad {
                    return Message::Navigation(NavigationMessage::EisenhowerSelectQuadrant(
                        quad_idx,
                    ));
                }
                // Already in this quadrant
                return Message::Navigation(NavigationMessage::FocusTaskList);
            }
        }
    }
    Message::None
}

/// Handle click in the Weekly Planner view.
fn handle_weekly_planner_click(x: u16, y: u16, model: &Model) -> Message {
    // Check which day column was clicked
    for (day_idx, day_area) in model.layout_cache.weekly_planner_days().iter().enumerate() {
        if let Some(area) = day_area {
            if LayoutCache::is_in_rect(x, y, *area) {
                let current_day = model.view_selection.weekly_planner_day;

                if day_idx != current_day {
                    return Message::Navigation(NavigationMessage::WeeklyPlannerSelectDay(day_idx));
                }
                // Already in this day
                return Message::Navigation(NavigationMessage::FocusTaskList);
            }
        }
    }
    Message::None
}

/// Handle click in the Reports view.
fn handle_reports_click(x: u16, y: u16, model: &Model) -> Message {
    // Check if clicking on any of the individual tab rects
    for (tab_idx, tab_rect) in model.layout_cache.reports_tab_rects().iter().enumerate() {
        if let Some(rect) = tab_rect {
            if LayoutCache::is_in_rect(x, y, *rect) {
                return Message::Navigation(NavigationMessage::ReportsSelectPanel(tab_idx));
            }
        }
    }

    // If clicking in the tabs area but not on a specific tab (e.g., on divider),
    // find the closest tab based on x position
    if let Some(tabs_area) = model.layout_cache.reports_tabs_area() {
        if LayoutCache::is_in_rect(x, y, tabs_area) {
            // Calculate which tab is closest based on cumulative positions
            let relative_x = x.saturating_sub(tabs_area.x);

            // Cumulative end positions including dividers
            // Overview(8)+|(3)=11, Velocity(8)+|(3)=22, Tags(4)+|(3)=29,
            // Time(4)+|(3)=36, Tasks(5)+|(3)=44, Focus(5)+|(3)=52,
            // Insights(8)+|(3)=63, Estimation(10)=73
            const TAB_ENDS: [u16; 8] = [11, 22, 29, 36, 44, 52, 63, 73];

            let panel_idx = TAB_ENDS
                .iter()
                .position(|&end| relative_x < end)
                .unwrap_or(TAB_ENDS.len() - 1);

            return Message::Navigation(NavigationMessage::ReportsSelectPanel(panel_idx));
        }
    }
    Message::None
}

/// Handle scroll wheel up.
fn handle_scroll_up(model: &Model) -> Message {
    if model.show_help {
        return Message::Ui(taskflow::app::UiMessage::HelpScrollUp);
    }
    Message::Navigation(NavigationMessage::Up)
}

/// Handle scroll wheel down.
fn handle_scroll_down(model: &Model) -> Message {
    if model.show_help {
        return Message::Ui(taskflow::app::UiMessage::HelpScrollDown);
    }
    Message::Navigation(NavigationMessage::Down)
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Check if a year is a leap year.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::MouseEvent;

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 2), 29); // Leap year
        assert_eq!(days_in_month(2023, 2), 28); // Non-leap year
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
    }

    #[test]
    fn test_days_in_month_all_months() {
        // January, March, May, July, August, October, December have 31 days
        for month in [1, 3, 5, 7, 8, 10, 12] {
            assert_eq!(days_in_month(2024, month), 31);
        }

        // April, June, September, November have 30 days
        for month in [4, 6, 9, 11] {
            assert_eq!(days_in_month(2024, month), 30);
        }
    }

    #[test]
    fn test_scroll_up_returns_up_message() {
        let model = Model::new();
        let result = handle_scroll_up(&model);
        assert!(matches!(result, Message::Navigation(NavigationMessage::Up)));
    }

    #[test]
    fn test_scroll_down_returns_down_message() {
        let model = Model::new();
        let result = handle_scroll_down(&model);
        assert!(matches!(
            result,
            Message::Navigation(NavigationMessage::Down)
        ));
    }

    #[test]
    fn test_handle_mouse_event_scroll_up() {
        let model = Model::new();
        let event = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let result = handle_mouse_event(event, &model);
        assert!(matches!(result, Message::Navigation(NavigationMessage::Up)));
    }

    #[test]
    fn test_handle_mouse_event_scroll_down() {
        let model = Model::new();
        let event = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let result = handle_mouse_event(event, &model);
        assert!(matches!(
            result,
            Message::Navigation(NavigationMessage::Down)
        ));
    }

    #[test]
    fn test_handle_mouse_event_other_button_returns_none() {
        let model = Model::new();
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let result = handle_mouse_event(event, &model);
        assert!(matches!(result, Message::None));
    }

    #[test]
    fn test_handle_mouse_event_mouse_move_returns_none() {
        let model = Model::new();
        let event = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let result = handle_mouse_event(event, &model);
        assert!(matches!(result, Message::None));
    }

    #[test]
    fn test_left_click_no_cached_areas_returns_none() {
        let model = Model::new();
        // Without any cached layout areas, click should return None
        let result = handle_left_click(50, 50, &model);
        assert!(matches!(result, Message::None));
    }

    // ========================================================================
    // Click helpers
    // ========================================================================

    use crossterm::event::KeyModifiers;
    use ratatui::layout::Rect;
    use taskflow::domain::Task;

    fn click(x: u16, y: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Main content area used by most click tests: x 20..100, y 0..40.
    fn main_area() -> Rect {
        Rect::new(20, 0, 80, 40)
    }

    fn model_with_tasks(count: usize) -> Model {
        let mut model = Model::new();
        for i in 0..count {
            let task = Task::new(format!("Task {i}"));
            model.tasks.insert(task.id, task);
        }
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), count);
        model
    }

    /// Task list view: main area and task list area coincide, with a
    /// two-row header (border + column header) and no scrolling.
    fn task_list_model(count: usize) -> Model {
        let model = model_with_tasks(count);
        model.layout_cache.set_main_area(main_area());
        model.layout_cache.set_task_list_area(main_area(), 2, 0);
        model
    }

    // ========================================================================
    // Sidebar
    // ========================================================================

    #[test]
    fn sidebar_click_selects_item_below_border() {
        let model = Model::new();
        assert!(model.show_sidebar);
        model
            .layout_cache
            .set_sidebar_area(Rect::new(0, 0, 20, 200));

        // Row 1 is the first item (row 0 is the border)
        assert!(matches!(
            handle_mouse_event(click(5, 1), &model),
            Message::Navigation(NavigationMessage::SidebarSelectIndex(0))
        ));
        assert!(matches!(
            handle_mouse_event(click(5, 4), &model),
            Message::Navigation(NavigationMessage::SidebarSelectIndex(3))
        ));
        // Clicking the border row maps to the first item
        assert!(matches!(
            handle_mouse_event(click(5, 0), &model),
            Message::Navigation(NavigationMessage::SidebarSelectIndex(0))
        ));
    }

    #[test]
    fn sidebar_click_past_last_item_is_ignored() {
        let model = Model::new();
        model
            .layout_cache
            .set_sidebar_area(Rect::new(0, 0, 20, 200));
        let count = model.sidebar_item_count();
        let last_row = u16::try_from(count).unwrap(); // row of item `count - 1`

        assert!(matches!(
            handle_mouse_event(click(5, last_row), &model),
            Message::Navigation(NavigationMessage::SidebarSelectIndex(idx)) if idx == count - 1
        ));
        assert!(matches!(
            handle_mouse_event(click(5, last_row + 1), &model),
            Message::None
        ));
    }

    #[test]
    fn hidden_sidebar_ignores_clicks_in_its_area() {
        let mut model = Model::new();
        model.show_sidebar = false;
        model.layout_cache.set_sidebar_area(Rect::new(0, 0, 20, 40));
        assert!(matches!(
            handle_mouse_event(click(5, 1), &model),
            Message::None
        ));
    }

    #[test]
    fn sidebar_wins_over_main_area_when_both_contain_point() {
        let model = task_list_model(2);
        model.layout_cache.set_sidebar_area(Rect::new(0, 0, 40, 40));
        assert!(matches!(
            handle_mouse_event(click(25, 2), &model),
            Message::Navigation(NavigationMessage::SidebarSelectIndex(1))
        ));
    }

    // ========================================================================
    // Task list
    // ========================================================================

    #[test]
    fn task_list_click_selects_row_under_header() {
        let model = task_list_model(3);
        assert_eq!(model.focus_pane, FocusPane::TaskList);
        assert!(matches!(
            handle_mouse_event(click(30, 2), &model),
            Message::Navigation(NavigationMessage::Select(0))
        ));
        assert!(matches!(
            handle_mouse_event(click(30, 4), &model),
            Message::Navigation(NavigationMessage::Select(2))
        ));
    }

    #[test]
    fn task_list_click_beyond_tasks_is_ignored() {
        let model = task_list_model(3);
        assert!(matches!(
            handle_mouse_event(click(30, 5), &model),
            Message::None
        ));
    }

    #[test]
    fn task_list_click_accounts_for_scroll_offset() {
        let model = model_with_tasks(10);
        model.layout_cache.set_main_area(main_area());
        model.layout_cache.set_task_list_area(main_area(), 2, 5);
        assert!(matches!(
            handle_mouse_event(click(30, 3), &model),
            Message::Navigation(NavigationMessage::Select(6))
        ));
    }

    #[test]
    fn task_list_click_focuses_pane_first_when_sidebar_focused() {
        let mut model = task_list_model(3);
        model.focus_pane = FocusPane::Sidebar;
        assert!(matches!(
            handle_mouse_event(click(30, 2), &model),
            Message::Navigation(NavigationMessage::FocusTaskList)
        ));
    }

    #[test]
    fn task_list_double_click_selects_same_row() {
        let model = task_list_model(3);
        assert!(matches!(
            handle_mouse_event(click(30, 3), &model),
            Message::Navigation(NavigationMessage::Select(1))
        ));
        // A repeated click at the same spot (double-click path) still selects
        assert!(matches!(
            handle_mouse_event(click(30, 3), &model),
            Message::Navigation(NavigationMessage::Select(1))
        ));
    }

    #[test]
    fn main_area_click_without_task_list_area_is_ignored() {
        let model = model_with_tasks(3);
        model.layout_cache.set_main_area(main_area());
        assert!(matches!(
            handle_mouse_event(click(30, 2), &model),
            Message::None
        ));
    }

    #[test]
    fn click_outside_main_area_is_ignored() {
        let model = task_list_model(3);
        assert!(matches!(
            handle_mouse_event(click(5, 2), &model),
            Message::None
        ));
        assert!(matches!(
            handle_mouse_event(click(30, 45), &model),
            Message::None
        ));
    }

    // ========================================================================
    // Calendar
    // ========================================================================

    /// Calendar view pinned to March 2024 (the 1st is a Friday, so the grid
    /// starts with a four-cell Monday..Thursday offset). The grid is 70 wide
    /// and 20 tall: 10-column cells and, after the two header rows, 3-row
    /// cells. Tasks are made visible in the default view first, because the
    /// calendar view filters `visible_tasks` by the (wall-clock) selected day.
    fn calendar_model(task_count: usize) -> Model {
        let mut model = model_with_tasks(task_count);
        model.current_view = ViewId::Calendar;
        model.calendar_state.year = 2024;
        model.calendar_state.month = 3;
        model.layout_cache.set_main_area(Rect::new(20, 0, 80, 40));
        model
            .layout_cache
            .set_calendar_area(Rect::new(20, 0, 70, 20));
        model
    }

    #[test]
    fn calendar_click_selects_day_from_cell() {
        let model = calendar_model(0);
        // Row 0, column 4 (Friday) is the 1st
        assert!(matches!(
            handle_mouse_event(click(61, 2), &model),
            Message::Navigation(NavigationMessage::CalendarSelectDay(1))
        ));
        // Row 1, column 0 (Monday) is the 4th
        assert!(matches!(
            handle_mouse_event(click(20, 5), &model),
            Message::Navigation(NavigationMessage::CalendarSelectDay(4))
        ));
        // Row 4, column 6 (Sunday) is the 31st
        assert!(matches!(
            handle_mouse_event(click(89, 14), &model),
            Message::Navigation(NavigationMessage::CalendarSelectDay(31))
        ));
    }

    #[test]
    fn calendar_click_on_empty_leading_cells_is_ignored() {
        let model = calendar_model(0);
        // Row 0, column 0 precedes the 1st of the month
        assert!(matches!(
            handle_mouse_event(click(20, 2), &model),
            Message::None
        ));
    }

    #[test]
    fn calendar_click_past_end_of_month_is_ignored() {
        let model = calendar_model(0);
        // Row 5, column 6 would be day 38
        assert!(matches!(
            handle_mouse_event(click(89, 17), &model),
            Message::None
        ));
    }

    #[test]
    fn calendar_click_outside_grid_falls_back_to_task_list() {
        let model = calendar_model(1);
        // Task list sits below the grid with a one-row header
        model
            .layout_cache
            .set_task_list_area(Rect::new(20, 20, 80, 20), 1, 0);

        assert!(matches!(
            handle_mouse_event(click(30, 21), &model),
            Message::Navigation(NavigationMessage::Select(0))
        ));
        // Past the only task
        assert!(matches!(
            handle_mouse_event(click(30, 25), &model),
            Message::None
        ));
    }

    #[test]
    fn calendar_too_narrow_for_cells_falls_back_to_task_list() {
        let model = calendar_model(1);
        // Fewer than seven columns: cell width rounds to zero
        model
            .layout_cache
            .set_calendar_area(Rect::new(20, 0, 5, 20));
        model
            .layout_cache
            .set_task_list_area(Rect::new(20, 0, 80, 40), 2, 0);
        assert!(matches!(
            handle_mouse_event(click(21, 2), &model),
            Message::Navigation(NavigationMessage::Select(0))
        ));
    }

    // ========================================================================
    // Kanban
    // ========================================================================

    fn kanban_model(current_column: usize) -> Model {
        let mut model = Model::new();
        model.current_view = ViewId::Kanban;
        model.view_selection.kanban_column = current_column;
        model.layout_cache.set_main_area(Rect::new(20, 0, 80, 40));
        for i in 0..4u16 {
            model
                .layout_cache
                .set_kanban_column(usize::from(i), Rect::new(20 + i * 20, 0, 20, 30));
        }
        model
    }

    #[test]
    fn kanban_click_on_other_column_switches_to_it() {
        let model = kanban_model(0);
        assert!(matches!(
            handle_mouse_event(click(45, 5), &model),
            Message::Navigation(NavigationMessage::KanbanSelectColumn(1))
        ));
        assert!(matches!(
            handle_mouse_event(click(65, 5), &model),
            Message::Navigation(NavigationMessage::KanbanSelectColumn(2))
        ));
        assert!(matches!(
            handle_mouse_event(click(85, 5), &model),
            Message::Navigation(NavigationMessage::KanbanSelectColumn(3))
        ));

        let model = kanban_model(3);
        assert!(matches!(
            handle_mouse_event(click(25, 5), &model),
            Message::Navigation(NavigationMessage::KanbanSelectColumn(0))
        ));
    }

    #[test]
    fn kanban_click_on_current_column_focuses_task_list() {
        let model = kanban_model(2);
        assert!(matches!(
            handle_mouse_event(click(65, 5), &model),
            Message::Navigation(NavigationMessage::FocusTaskList)
        ));
    }

    #[test]
    fn kanban_click_outside_columns_is_ignored() {
        let model = kanban_model(0);
        // Inside the main area but below every column
        assert!(matches!(
            handle_mouse_event(click(45, 35), &model),
            Message::None
        ));
    }

    // ========================================================================
    // Eisenhower
    // ========================================================================

    fn eisenhower_model(current_quadrant: usize) -> Model {
        let mut model = Model::new();
        model.current_view = ViewId::Eisenhower;
        model.view_selection.eisenhower_quadrant = current_quadrant;
        model.layout_cache.set_main_area(Rect::new(20, 0, 80, 45));
        model
            .layout_cache
            .set_eisenhower_quadrant(0, Rect::new(20, 0, 40, 20));
        model
            .layout_cache
            .set_eisenhower_quadrant(1, Rect::new(60, 0, 40, 20));
        model
            .layout_cache
            .set_eisenhower_quadrant(2, Rect::new(20, 20, 40, 20));
        model
            .layout_cache
            .set_eisenhower_quadrant(3, Rect::new(60, 20, 40, 20));
        model
    }

    #[test]
    fn eisenhower_click_selects_quadrant_or_focuses() {
        let model = eisenhower_model(0);
        assert!(matches!(
            handle_mouse_event(click(70, 5), &model),
            Message::Navigation(NavigationMessage::EisenhowerSelectQuadrant(1))
        ));
        assert!(matches!(
            handle_mouse_event(click(30, 25), &model),
            Message::Navigation(NavigationMessage::EisenhowerSelectQuadrant(2))
        ));
        assert!(matches!(
            handle_mouse_event(click(70, 25), &model),
            Message::Navigation(NavigationMessage::EisenhowerSelectQuadrant(3))
        ));
        assert!(matches!(
            handle_mouse_event(click(30, 5), &model),
            Message::Navigation(NavigationMessage::FocusTaskList)
        ));
        // Inside the main area but below the quadrants
        assert!(matches!(
            handle_mouse_event(click(30, 42), &model),
            Message::None
        ));
    }

    // ========================================================================
    // Weekly planner
    // ========================================================================

    fn weekly_planner_model(current_day: usize) -> Model {
        let mut model = Model::new();
        model.current_view = ViewId::WeeklyPlanner;
        model.view_selection.weekly_planner_day = current_day;
        model.layout_cache.set_main_area(Rect::new(20, 0, 80, 45));
        for i in 0..7u16 {
            model
                .layout_cache
                .set_weekly_planner_day(usize::from(i), Rect::new(20 + i * 10, 0, 10, 40));
        }
        model
    }

    #[test]
    fn weekly_planner_click_selects_day_or_focuses() {
        let model = weekly_planner_model(0);
        assert!(matches!(
            handle_mouse_event(click(65, 5), &model),
            Message::Navigation(NavigationMessage::WeeklyPlannerSelectDay(4))
        ));
        assert!(matches!(
            handle_mouse_event(click(89, 5), &model),
            Message::Navigation(NavigationMessage::WeeklyPlannerSelectDay(6))
        ));
        assert!(matches!(
            handle_mouse_event(click(25, 5), &model),
            Message::Navigation(NavigationMessage::FocusTaskList)
        ));
        // Inside the main area but below the day columns
        assert!(matches!(
            handle_mouse_event(click(25, 42), &model),
            Message::None
        ));
    }

    // ========================================================================
    // Reports
    // ========================================================================

    fn reports_model() -> Model {
        let mut model = Model::new();
        model.current_view = ViewId::Reports;
        model.layout_cache.set_main_area(Rect::new(20, 0, 80, 40));
        model
    }

    #[test]
    fn reports_click_on_tab_rect_selects_that_panel() {
        let model = reports_model();
        for i in 0..7u16 {
            model
                .layout_cache
                .set_reports_tab_rect(usize::from(i), Rect::new(20 + i * 10, 0, 10, 1));
        }
        assert!(matches!(
            handle_mouse_event(click(20, 0), &model),
            Message::Navigation(NavigationMessage::ReportsSelectPanel(0))
        ));
        assert!(matches!(
            handle_mouse_event(click(45, 0), &model),
            Message::Navigation(NavigationMessage::ReportsSelectPanel(2))
        ));
        assert!(matches!(
            handle_mouse_event(click(89, 0), &model),
            Message::Navigation(NavigationMessage::ReportsSelectPanel(6))
        ));
        // Below the tab row, with no tabs area cached, nothing happens
        assert!(matches!(
            handle_mouse_event(click(45, 5), &model),
            Message::None
        ));
    }

    #[test]
    fn reports_click_in_tabs_area_uses_cumulative_tab_widths() {
        let model = reports_model();
        model
            .layout_cache
            .set_reports_tabs_area(Rect::new(20, 0, 80, 1));

        // (relative x, expected panel): boundaries from the tab widths
        let cases: [(u16, usize); 17] = [
            (0, 0),
            (10, 0),
            (11, 1),
            (21, 1),
            (22, 2),
            (28, 2),
            (29, 3),
            (35, 3),
            (36, 4),
            (43, 4),
            (44, 5),
            (51, 5),
            (52, 6),
            (62, 6),
            (63, 7),
            (72, 7),
            (79, 7),
        ];
        for (rel_x, expected) in cases {
            assert!(
                matches!(
                    handle_mouse_event(click(20 + rel_x, 0), &model),
                    Message::Navigation(NavigationMessage::ReportsSelectPanel(p)) if p == expected
                ),
                "relative x {rel_x} should select panel {expected}"
            );
        }

        // Inside the main area but outside the tab strip
        assert!(matches!(
            handle_mouse_event(click(45, 5), &model),
            Message::None
        ));
    }

    #[test]
    fn reports_tab_rect_wins_over_tabs_area_fallback() {
        let model = reports_model();
        model
            .layout_cache
            .set_reports_tabs_area(Rect::new(20, 0, 80, 1));
        // A rect for panel 5 placed where the fallback would pick panel 0
        model
            .layout_cache
            .set_reports_tab_rect(5, Rect::new(20, 0, 5, 1));
        assert!(matches!(
            handle_mouse_event(click(22, 0), &model),
            Message::Navigation(NavigationMessage::ReportsSelectPanel(5))
        ));
    }

    // ========================================================================
    // Scroll with help open
    // ========================================================================

    #[test]
    fn scroll_with_help_open_scrolls_help() {
        let mut model = Model::new();
        model.show_help = true;
        assert!(matches!(
            handle_scroll_up(&model),
            Message::Ui(taskflow::app::UiMessage::HelpScrollUp)
        ));
        assert!(matches!(
            handle_scroll_down(&model),
            Message::Ui(taskflow::app::UiMessage::HelpScrollDown)
        ));
    }
}
