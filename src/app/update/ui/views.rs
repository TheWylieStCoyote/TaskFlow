//! View-specific selection handlers.
//!
//! Handles messages for selecting/viewing tasks in specialized views:
//! - Timeline view
//! - Kanban board
//! - Eisenhower matrix
//! - Weekly planner
//! - Network view
//! - Chain navigation

use crate::app::{Model, UiMessage};

use super::input::enter_focus_for_task;

/// Handle view-specific selection and navigation messages.
pub fn handle_ui_views(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::TimelineToggleDependencies => {
            model.timeline_state.show_dependencies = !model.timeline_state.show_dependencies;
        }
        UiMessage::TimelineViewSelected => {
            // Get timeline tasks (filtered and sorted same as timeline widget)
            let timeline_tasks: Vec<_> = model
                .visible_tasks
                .iter()
                .filter_map(|id| model.tasks.get(id))
                .filter(|t| t.scheduled_date.is_some() || t.due_date.is_some())
                .collect();

            if let Some(task) = timeline_tasks.get(model.timeline_state.selected_task_index) {
                enter_focus_for_task(model, task.id);
            }
        }
        UiMessage::KanbanViewSelected => {
            let column_tasks = model.kanban_column_tasks(model.view_selection.kanban_column);
            if let Some(&task_id) = column_tasks.get(model.view_selection.kanban_task_index) {
                enter_focus_for_task(model, task_id);
            }
        }
        UiMessage::EisenhowerViewSelected => {
            let quadrant_tasks =
                model.eisenhower_quadrant_tasks(model.view_selection.eisenhower_quadrant);
            if let Some(&task_id) = quadrant_tasks.get(model.view_selection.eisenhower_task_index) {
                enter_focus_for_task(model, task_id);
            }
        }
        UiMessage::WeeklyPlannerViewSelected => {
            let day_tasks = model.weekly_planner_day_tasks(model.view_selection.weekly_planner_day);
            if let Some(&task_id) = day_tasks.get(model.view_selection.weekly_planner_task_index) {
                enter_focus_for_task(model, task_id);
            }
        }
        UiMessage::NetworkViewSelected => {
            let network_tasks = model.network_tasks();
            if let Some(&task_id) = network_tasks.get(model.view_selection.network_task_index) {
                enter_focus_for_task(model, task_id);
            }
        }
        UiMessage::ChainNext => {
            // Navigate to next task in chain
            if let Some(current_task) = model.selected_task() {
                if let Some(next_id) = current_task.next_task_id {
                    // Find this task's position in visible_tasks
                    if let Some(pos) = model.visible_tasks.iter().position(|id| *id == next_id) {
                        model.selected_index = pos;
                        model.alerts.status_message = Some("→ Next in chain".to_string());
                    }
                }
            }
        }
        UiMessage::ChainPrev => {
            // Navigate to previous task in chain (the task that links to this one)
            if let Some(current_task) = model.selected_task() {
                let current_id = current_task.id;
                // Find task that has next_task_id pointing to current task
                if let Some(prev_task) = model
                    .tasks
                    .values()
                    .find(|t| t.next_task_id == Some(current_id))
                {
                    let prev_id = prev_task.id;
                    // Find this task's position in visible_tasks
                    if let Some(pos) = model.visible_tasks.iter().position(|id| *id == prev_id) {
                        model.selected_index = pos;
                        model.alerts.status_message = Some("← Previous in chain".to_string());
                    }
                }
            }
        }
        _ => {}
    }
}
