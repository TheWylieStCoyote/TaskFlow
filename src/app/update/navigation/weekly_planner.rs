//! Weekly planner view navigation handlers.

use crate::app::{Model, NavigationMessage, ViewId};

/// Handle weekly planner-specific navigation messages.
pub fn handle_weekly_planner_navigation(model: &mut Model, msg: NavigationMessage) {
    if model.current_view != ViewId::WeeklyPlanner {
        return;
    }

    match msg {
        NavigationMessage::WeeklyPlannerLeft => {
            if model.view_selection.weekly_planner_day > 0 {
                model.view_selection.weekly_planner_day -= 1;
                model.view_selection.weekly_planner_task_index = 0; // Reset task selection
            }
        }
        NavigationMessage::WeeklyPlannerRight => {
            if model.view_selection.weekly_planner_day < 6 {
                model.view_selection.weekly_planner_day += 1;
                model.view_selection.weekly_planner_task_index = 0; // Reset task selection
            }
        }
        NavigationMessage::WeeklyPlannerUp => {
            if model.view_selection.weekly_planner_task_index > 0 {
                model.view_selection.weekly_planner_task_index -= 1;
            }
        }
        NavigationMessage::WeeklyPlannerDown => {
            let day_tasks = model.weekly_planner_day_tasks(model.view_selection.weekly_planner_day);
            if model.view_selection.weekly_planner_task_index + 1 < day_tasks.len() {
                model.view_selection.weekly_planner_task_index += 1;
            }
        }
        NavigationMessage::WeeklyPlannerSelectDay(day) => {
            if day < 7 {
                model.view_selection.weekly_planner_day = day;
                model.view_selection.weekly_planner_task_index = 0; // Reset task selection
                model.selected_index = 0;
            }
        }
        _ => {}
    }
}
