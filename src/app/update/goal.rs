//! Goal/OKR tracking update handlers.
//!
//! Handles all goal-related operations including creating goals and key results,
//! updating progress, and navigating the goal view.

use chrono::Utc;
use tracing::debug;

use crate::app::{GoalMessage, Model};
use crate::domain::{Goal, KeyResult};

/// Handle goal/OKR tracking messages.
pub(crate) fn handle_goal(model: &mut Model, msg: GoalMessage) {
    match msg {
        // ==================== Goal CRUD ====================
        GoalMessage::Create(name) => {
            let goal = Goal::new(name);
            let id = goal.id;
            model.goals.insert(id, goal);
            model.refresh_visible_goals();
            model.storage.dirty = true;
            debug!(?id, "Created new goal");
        }
        GoalMessage::UpdateName { id, name } => {
            if let Some(goal) = model.goals.get_mut(&id) {
                goal.name = name;
                goal.updated_at = Utc::now();
            }
            model.refresh_visible_goals();
            model.storage.dirty = true;
            debug!(?id, "Updated goal name");
        }
        GoalMessage::UpdateDescription { id, description } => {
            if let Some(goal) = model.goals.get_mut(&id) {
                goal.description = description;
                goal.updated_at = Utc::now();
            }
            model.storage.dirty = true;
            debug!(?id, "Updated goal description");
        }
        GoalMessage::SetStatus { id, status } => {
            if let Some(goal) = model.goals.get_mut(&id) {
                goal.status = status;
                goal.updated_at = Utc::now();
            }
            model.refresh_visible_goals();
            model.storage.dirty = true;
            debug!(?id, ?status, "Updated goal status");
        }
        GoalMessage::SetDates { id, start, end } => {
            if let Some(goal) = model.goals.get_mut(&id) {
                goal.start_date = start;
                goal.due_date = end;
                goal.updated_at = Utc::now();
            }
            model.storage.dirty = true;
            debug!(?id, ?start, ?end, "Updated goal dates");
        }
        GoalMessage::SetQuarter { id, quarter } => {
            if let Some(goal) = model.goals.get_mut(&id) {
                goal.quarter = quarter;
                goal.updated_at = Utc::now();
            }
            model.refresh_visible_goals();
            model.storage.dirty = true;
            debug!(?id, ?quarter, "Updated goal quarter");
        }
        GoalMessage::SetManualProgress { id, progress } => {
            if let Some(goal) = model.goals.get_mut(&id) {
                goal.manual_progress = progress.map(|p| p.min(100));
                goal.updated_at = Utc::now();
            }
            model.storage.dirty = true;
            debug!(?id, ?progress, "Updated goal manual progress");
        }
        GoalMessage::Delete(id) => {
            // Delete all key results for this goal first
            let kr_ids: Vec<_> = model
                .key_results
                .values()
                .filter(|kr| kr.goal_id == id)
                .map(|kr| kr.id)
                .collect();
            for kr_id in kr_ids {
                model.key_results.remove(&kr_id);
            }
            // Delete the goal
            if model.goals.remove(&id).is_some() {
                model.refresh_visible_goals();
                model.storage.dirty = true;
                debug!(?id, "Deleted goal");
            }
        }

        // ==================== Key Result CRUD ====================
        GoalMessage::CreateKeyResult { goal_id, name } => {
            let kr = KeyResult::new(goal_id, name);
            let id = kr.id;
            model.key_results.insert(id, kr);
            model.storage.dirty = true;
            debug!(?id, ?goal_id, "Created new key result");
        }
        GoalMessage::UpdateKeyResultName { id, name } => {
            if let Some(kr) = model.key_results.get_mut(&id) {
                kr.name = name;
                kr.updated_at = Utc::now();
            }
            model.storage.dirty = true;
            debug!(?id, "Updated key result name");
        }
        GoalMessage::SetKeyResultStatus { id, status } => {
            if let Some(kr) = model.key_results.get_mut(&id) {
                kr.status = status;
                kr.updated_at = Utc::now();
            }
            model.storage.dirty = true;
            debug!(?id, ?status, "Updated key result status");
        }
        GoalMessage::SetKeyResultTarget { id, target, unit } => {
            if let Some(kr) = model.key_results.get_mut(&id) {
                kr.target_value = target;
                kr.unit = unit;
                kr.updated_at = Utc::now();
            }
            model.storage.dirty = true;
            debug!(?id, target, "Updated key result target");
        }
        GoalMessage::SetKeyResultValue { id, value } => {
            if let Some(kr) = model.key_results.get_mut(&id) {
                kr.set_value(value);
            }
            model.storage.dirty = true;
            debug!(?id, value, "Updated key result value");
        }
        GoalMessage::SetKeyResultManualProgress { id, progress } => {
            if let Some(kr) = model.key_results.get_mut(&id) {
                kr.manual_progress = progress.map(|p| p.min(100));
                kr.updated_at = Utc::now();
            }
            model.storage.dirty = true;
            debug!(?id, ?progress, "Updated key result manual progress");
        }
        GoalMessage::LinkProject { kr_id, project_id } => {
            if let Some(kr) = model.key_results.get_mut(&kr_id) {
                kr.link_project(project_id);
            }
            model.storage.dirty = true;
            debug!(?kr_id, ?project_id, "Linked project to key result");
        }
        GoalMessage::UnlinkProject { kr_id, project_id } => {
            if let Some(kr) = model.key_results.get_mut(&kr_id) {
                kr.unlink_project(&project_id);
            }
            model.storage.dirty = true;
            debug!(?kr_id, ?project_id, "Unlinked project from key result");
        }
        GoalMessage::LinkTask { kr_id, task_id } => {
            if let Some(kr) = model.key_results.get_mut(&kr_id) {
                kr.link_task(task_id);
            }
            model.storage.dirty = true;
            debug!(?kr_id, ?task_id, "Linked task to key result");
        }
        GoalMessage::UnlinkTask { kr_id, task_id } => {
            if let Some(kr) = model.key_results.get_mut(&kr_id) {
                kr.unlink_task(&task_id);
            }
            model.storage.dirty = true;
            debug!(?kr_id, ?task_id, "Unlinked task from key result");
        }
        GoalMessage::DeleteKeyResult(id) => {
            if model.key_results.remove(&id).is_some() {
                model.storage.dirty = true;
                debug!(?id, "Deleted key result");
            }
        }

        // ==================== View Navigation ====================
        GoalMessage::ExpandGoal(id) => {
            model.goal_view.expanded_goal = Some(id);
            model.goal_view.selected_kr = 0;
            debug!(?id, "Expanded goal");
        }
        GoalMessage::CollapseGoal => {
            model.goal_view.expanded_goal = None;
            debug!("Collapsed goal");
        }
        GoalMessage::ToggleArchived => {
            model.goal_view.show_archived = !model.goal_view.show_archived;
            model.refresh_visible_goals();
            debug!(
                show_archived = model.goal_view.show_archived,
                "Toggled show archived"
            );
        }
        GoalMessage::FilterByQuarter(quarter) => {
            model.goal_view.filter_quarter = quarter;
            model.refresh_visible_goals();
            debug!(?quarter, "Filtered by quarter");
        }
        GoalMessage::NavigateUp => {
            if model.goal_view.expanded_goal.is_some() {
                // Navigate within key results
                if model.goal_view.selected_kr > 0 {
                    model.goal_view.selected_kr -= 1;
                }
            } else {
                // Navigate within goals
                if model.goal_view.selected_goal > 0 {
                    model.goal_view.selected_goal -= 1;
                }
            }
        }
        GoalMessage::NavigateDown => {
            if let Some(goal_id) = model.goal_view.expanded_goal {
                // Navigate within key results
                let kr_count = model.key_results_for_goal(goal_id).len();
                if model.goal_view.selected_kr + 1 < kr_count {
                    model.goal_view.selected_kr += 1;
                }
            } else {
                // Navigate within goals
                if model.goal_view.selected_goal + 1 < model.visible_goals.len() {
                    model.goal_view.selected_goal += 1;
                }
            }
        }
        GoalMessage::NavigateInto => {
            // Expand selected goal to show key results
            if model.goal_view.expanded_goal.is_none() {
                if let Some(goal_id) = model.visible_goals.get(model.goal_view.selected_goal) {
                    model.goal_view.expanded_goal = Some(*goal_id);
                    model.goal_view.selected_kr = 0;
                }
            }
        }
        GoalMessage::NavigateBack => {
            // Collapse back to goals list
            model.goal_view.expanded_goal = None;
        }
    }
}
