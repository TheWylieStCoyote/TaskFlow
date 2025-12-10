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
            // Time(4)+|(3)=36, Focus(5)+|(3)=44, Insights(8)+|(3)=55, Estimation(10)=65
            const TAB_ENDS: [u16; 7] = [11, 22, 29, 36, 44, 55, 65];

            let panel_idx = TAB_ENDS
                .iter()
                .position(|&end| relative_x < end)
                .unwrap_or(6);

            return Message::Navigation(NavigationMessage::ReportsSelectPanel(panel_idx));
        }
    }
    Message::None
}

/// Handle scroll wheel up.
fn handle_scroll_up(_model: &Model) -> Message {
    // Scroll up moves selection up in the currently focused pane
    Message::Navigation(NavigationMessage::Up)
}

/// Handle scroll wheel down.
fn handle_scroll_down(_model: &Model) -> Message {
    // Scroll down moves selection down in the currently focused pane
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
}
