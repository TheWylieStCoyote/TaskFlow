//! Goal/OKR tracking UI handlers.
//!
//! Handles goal-related UI messages including:
//! - Creating and editing goals
//! - Creating key results

use crate::app::{Model, UiMessage};

use super::input::start_input;
use crate::ui::InputTarget;

/// Handle goal-related UI messages.
pub fn handle_ui_goals(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::StartCreateGoal => {
            start_input(model, InputTarget::GoalName, None);
        }
        UiMessage::StartEditGoal(goal_id) => {
            let prefill = model.goals.get(&goal_id).map(|g| g.name.clone());
            start_input(model, InputTarget::EditGoalName(goal_id), prefill);
        }
        UiMessage::StartCreateKeyResult => {
            if let Some(&goal_id) = model.visible_goals.get(model.goal_view.selected_goal) {
                start_input(model, InputTarget::KeyResultName(goal_id), None);
            }
        }
        _ => {}
    }
}
