//! Navigation message handlers
//!
//! Handles all navigation-related messages including:
//! - Task list navigation (up, down, page up, page down)
//! - Sidebar navigation
//! - Calendar navigation
//! - View switching

use crate::app::{
    FocusPane, Model, NavigationMessage, ViewId, SIDEBAR_FIRST_PROJECT_INDEX,
    SIDEBAR_PROJECTS_HEADER_INDEX, SIDEBAR_SEPARATOR_INDEX, SIDEBAR_VIEWS,
};

/// Handle navigation messages
#[allow(clippy::too_many_lines)]
pub fn handle_navigation(model: &mut Model, msg: NavigationMessage) {
    match msg {
        NavigationMessage::Up => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    if model.calendar_state.focus_task_list {
                        // Navigate tasks in calendar task list
                        if model.selected_index > 0 {
                            model.selected_index -= 1;
                        }
                    } else {
                        // In calendar grid, up moves to previous week (or wraps)
                        handle_calendar_up(model);
                    }
                } else if model.selected_index > 0 {
                    model.selected_index -= 1;
                }
            }
            FocusPane::Sidebar => {
                if model.sidebar_selected > 0 {
                    model.sidebar_selected -= 1;
                    // Skip separator
                    if model.sidebar_selected == SIDEBAR_SEPARATOR_INDEX {
                        model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX - 1;
                    }
                }
            }
        },
        NavigationMessage::Down => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    if model.calendar_state.focus_task_list {
                        // Navigate tasks in calendar task list
                        let task_count = model.tasks_for_selected_day().len();
                        if model.selected_index < task_count.saturating_sub(1) {
                            model.selected_index += 1;
                        }
                    } else {
                        // In calendar grid, down moves to next week (or wraps)
                        handle_calendar_down(model);
                    }
                } else if model.selected_index < model.visible_tasks.len().saturating_sub(1) {
                    model.selected_index += 1;
                }
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                if model.sidebar_selected < max_index {
                    model.sidebar_selected += 1;
                    // Skip separator
                    if model.sidebar_selected == SIDEBAR_SEPARATOR_INDEX {
                        model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX + 1;
                    }
                }
            }
        },
        NavigationMessage::First => match model.focus_pane {
            FocusPane::TaskList => model.selected_index = 0,
            FocusPane::Sidebar => model.sidebar_selected = 0,
        },
        NavigationMessage::Last => match model.focus_pane {
            FocusPane::TaskList => {
                if !model.visible_tasks.is_empty() {
                    model.selected_index = model.visible_tasks.len() - 1;
                }
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_item_count().saturating_sub(1);
            }
        },
        NavigationMessage::PageUp => match model.focus_pane {
            FocusPane::TaskList => {
                model.selected_index = model.selected_index.saturating_sub(10);
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_selected.saturating_sub(5);
            }
        },
        NavigationMessage::PageDown => match model.focus_pane {
            FocusPane::TaskList => {
                let max_index = model.visible_tasks.len().saturating_sub(1);
                model.selected_index = (model.selected_index + 10).min(max_index);
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                model.sidebar_selected = (model.sidebar_selected + 5).min(max_index);
            }
        },
        NavigationMessage::Select(index) => {
            if index < model.visible_tasks.len() {
                model.selected_index = index;
            }
        }
        NavigationMessage::GoToView(view_id) => {
            model.current_view = view_id;
            model.selected_index = 0;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.habit_view.show_analytics = false; // Clear modal state when switching views
            model.refresh_visible_tasks();
        }
        NavigationMessage::FocusSidebar => {
            if model.show_sidebar {
                model.focus_pane = FocusPane::Sidebar;
            }
        }
        NavigationMessage::FocusTaskList => {
            model.focus_pane = FocusPane::TaskList;
        }
        NavigationMessage::SelectSidebarItem => {
            handle_sidebar_selection(model);
        }
        NavigationMessage::CalendarPrevMonth => {
            if model.calendar_state.month == 1 {
                model.calendar_state.month = 12;
                model.calendar_state.year -= 1;
            } else {
                model.calendar_state.month -= 1;
            }
            // Adjust selected day if it exceeds days in new month
            let days_in_month =
                days_in_month(model.calendar_state.year, model.calendar_state.month);
            if let Some(day) = model.calendar_state.selected_day {
                if day > days_in_month {
                    model.calendar_state.selected_day = Some(days_in_month);
                }
            }
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarNextMonth => {
            if model.calendar_state.month == 12 {
                model.calendar_state.month = 1;
                model.calendar_state.year += 1;
            } else {
                model.calendar_state.month += 1;
            }
            // Adjust selected day if it exceeds days in new month
            let days_in_month =
                days_in_month(model.calendar_state.year, model.calendar_state.month);
            if let Some(day) = model.calendar_state.selected_day {
                if day > days_in_month {
                    model.calendar_state.selected_day = Some(days_in_month);
                }
            }
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarSelectDay(day) => {
            model.calendar_state.selected_day = Some(day);
            model.calendar_state.focus_task_list = false; // Reset focus to grid
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarFocusTaskList => {
            if model.current_view == ViewId::Calendar {
                // Only focus task list if there are tasks for the selected day
                if !model.tasks_for_selected_day().is_empty() {
                    model.calendar_state.focus_task_list = true;
                    model.selected_index = 0;
                }
            }
        }
        NavigationMessage::CalendarFocusGrid => {
            model.calendar_state.focus_task_list = false;
        }
        NavigationMessage::ReportsNextPanel => {
            if model.current_view == ViewId::Reports {
                model.report_panel = model.report_panel.next();
            }
        }
        NavigationMessage::ReportsPrevPanel => {
            if model.current_view == ViewId::Reports {
                model.report_panel = model.report_panel.prev();
            }
        }
        NavigationMessage::TimelineScrollLeft => {
            if model.current_view == ViewId::Timeline {
                model.timeline_state.viewport_start -= chrono::Duration::days(7);
            }
        }
        NavigationMessage::TimelineScrollRight => {
            if model.current_view == ViewId::Timeline {
                model.timeline_state.viewport_start += chrono::Duration::days(7);
            }
        }
        NavigationMessage::TimelineZoomIn => {
            if model.current_view == ViewId::Timeline {
                use crate::app::TimelineZoom;
                if model.timeline_state.zoom_level == TimelineZoom::Week {
                    model.timeline_state.zoom_level = TimelineZoom::Day;
                }
            }
        }
        NavigationMessage::TimelineZoomOut => {
            if model.current_view == ViewId::Timeline {
                use crate::app::TimelineZoom;
                if model.timeline_state.zoom_level == TimelineZoom::Day {
                    model.timeline_state.zoom_level = TimelineZoom::Week;
                }
            }
        }
        NavigationMessage::TimelineGoToday => {
            if model.current_view == ViewId::Timeline {
                let today = chrono::Utc::now().date_naive();
                model.timeline_state.viewport_start = today - chrono::Duration::days(7);
            }
        }
        NavigationMessage::TimelineUp => {
            if model.current_view == ViewId::Timeline
                && model.timeline_state.selected_task_index > 0
            {
                model.timeline_state.selected_task_index -= 1;
            }
        }
        NavigationMessage::TimelineDown => {
            if model.current_view == ViewId::Timeline {
                let max_index = model.visible_tasks.len().saturating_sub(1);
                if model.timeline_state.selected_task_index < max_index {
                    model.timeline_state.selected_task_index += 1;
                }
            }
        }
        NavigationMessage::KanbanLeft => {
            if model.current_view == ViewId::Kanban && model.view_selection.kanban_column > 0 {
                model.view_selection.kanban_column -= 1;
            }
        }
        NavigationMessage::KanbanRight => {
            if model.current_view == ViewId::Kanban && model.view_selection.kanban_column < 3 {
                model.view_selection.kanban_column += 1;
            }
        }
        NavigationMessage::EisenhowerUp => {
            if model.current_view == ViewId::Eisenhower
                && model.view_selection.eisenhower_quadrant >= 2
            {
                model.view_selection.eisenhower_quadrant -= 2;
            }
        }
        NavigationMessage::EisenhowerDown => {
            if model.current_view == ViewId::Eisenhower
                && model.view_selection.eisenhower_quadrant < 2
            {
                model.view_selection.eisenhower_quadrant += 2;
            }
        }
        NavigationMessage::EisenhowerLeft => {
            if model.current_view == ViewId::Eisenhower
                && model.view_selection.eisenhower_quadrant % 2 == 1
            {
                model.view_selection.eisenhower_quadrant -= 1;
            }
        }
        NavigationMessage::EisenhowerRight => {
            if model.current_view == ViewId::Eisenhower
                && model.view_selection.eisenhower_quadrant.is_multiple_of(2)
            {
                model.view_selection.eisenhower_quadrant += 1;
            }
        }
        NavigationMessage::WeeklyPlannerLeft => {
            if model.current_view == ViewId::WeeklyPlanner
                && model.view_selection.weekly_planner_day > 0
            {
                model.view_selection.weekly_planner_day -= 1;
            }
        }
        NavigationMessage::WeeklyPlannerRight => {
            if model.current_view == ViewId::WeeklyPlanner
                && model.view_selection.weekly_planner_day < 6
            {
                model.view_selection.weekly_planner_day += 1;
            }
        }
    }
}

/// Helper to get days in a month
pub fn days_in_month(year: i32, month: u32) -> u32 {
    use chrono::{Datelike, NaiveDate};
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .and_then(|d| d.pred_opt())
    .map(|d| d.day())
    .unwrap_or(28)
}

/// Handle calendar up navigation (move to previous week)
fn handle_calendar_up(model: &mut Model) {
    if let Some(day) = model.calendar_state.selected_day {
        if day > 7 {
            model.calendar_state.selected_day = Some(day - 7);
        } else {
            // Move to previous month, last row
            if model.calendar_state.month == 1 {
                model.calendar_state.month = 12;
                model.calendar_state.year -= 1;
            } else {
                model.calendar_state.month -= 1;
            }
            let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
            // Try to land on same weekday in last week
            let new_day = days - (7 - day);
            model.calendar_state.selected_day = Some(new_day.max(1));
        }
        model.calendar_state.focus_task_list = false; // Reset focus to grid
        model.selected_index = 0;
        model.refresh_visible_tasks();
    }
}

/// Handle calendar down navigation (move to next week)
fn handle_calendar_down(model: &mut Model) {
    if let Some(day) = model.calendar_state.selected_day {
        let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
        if day + 7 <= days {
            model.calendar_state.selected_day = Some(day + 7);
        } else {
            // Move to next month, first row
            if model.calendar_state.month == 12 {
                model.calendar_state.month = 1;
                model.calendar_state.year += 1;
            } else {
                model.calendar_state.month += 1;
            }
            // Try to land on same weekday in first week
            let new_day = (day + 7) - days;
            model.calendar_state.selected_day = Some(new_day.min(7));
        }
        model.calendar_state.focus_task_list = false; // Reset focus to grid
        model.selected_index = 0;
        model.refresh_visible_tasks();
    }
}

/// Handle sidebar item selection
fn handle_sidebar_selection(model: &mut Model) {
    let selected = model.sidebar_selected;

    // Sidebar layout uses SIDEBAR_VIEWS array from model.rs:
    // [0..SIDEBAR_VIEW_COUNT-1]: View items from SIDEBAR_VIEWS
    // SIDEBAR_SEPARATOR_INDEX: Separator (skip)
    // SIDEBAR_PROJECTS_HEADER_INDEX: "Projects" header
    // SIDEBAR_FIRST_PROJECT_INDEX+: Individual projects

    // Helper to activate a view
    let activate_view = |model: &mut Model, view: ViewId| {
        model.current_view = view;
        model.selected_project = None;
        model.focus_pane = FocusPane::TaskList;
        model.selected_index = 0;
        model.refresh_visible_tasks();
    };

    // Check if it's a view from SIDEBAR_VIEWS array
    if let Some(&view_id) = SIDEBAR_VIEWS.get(selected) {
        activate_view(model, view_id);
        return;
    }

    // Handle special items after the views
    match selected {
        n if n == SIDEBAR_SEPARATOR_INDEX => {} // Separator, do nothing
        n if n == SIDEBAR_PROJECTS_HEADER_INDEX => {
            // Projects header - go to Projects view showing all project tasks
            activate_view(model, ViewId::Projects);
        }
        n if n >= SIDEBAR_FIRST_PROJECT_INDEX => {
            // Select a specific project
            let project_index = n - SIDEBAR_FIRST_PROJECT_INDEX;
            let project_ids: Vec<_> = model.projects.keys().cloned().collect();
            if let Some(project_id) = project_ids.get(project_index) {
                model.current_view = ViewId::TaskList;
                model.selected_project = Some(*project_id);
                model.focus_pane = FocusPane::TaskList;
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_in_month() {
        // January has 31 days
        assert_eq!(days_in_month(2024, 1), 31);
        // February 2024 (leap year) has 29 days
        assert_eq!(days_in_month(2024, 2), 29);
        // February 2023 (non-leap year) has 28 days
        assert_eq!(days_in_month(2023, 2), 28);
        // April has 30 days
        assert_eq!(days_in_month(2024, 4), 30);
        // December has 31 days
        assert_eq!(days_in_month(2024, 12), 31);
    }
}
