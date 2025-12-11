//! Habit tracking UI handlers.
//!
//! Handles habit-related UI messages including:
//! - Creating and editing habits
//! - Navigation within habit list
//! - Toggling daily completion
//! - Analytics display
//! - Archiving and deletion

use crate::app::{Model, UiMessage};

use super::input::start_input;
use crate::ui::InputTarget;

/// Handle habit-related UI messages.
pub fn handle_ui_habits(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::StartCreateHabit => {
            start_input(model, InputTarget::NewHabit, None);
        }
        UiMessage::StartEditHabit(habit_id) => {
            let prefill = model.habits.get(&habit_id).map(|h| h.name.clone());
            start_input(model, InputTarget::EditHabit(habit_id), prefill);
        }
        UiMessage::HabitUp => {
            if model.habit_view.selected > 0 {
                model.habit_view.selected -= 1;
            }
        }
        UiMessage::HabitDown => {
            if !model.visible_habits.is_empty()
                && model.habit_view.selected < model.visible_habits.len() - 1
            {
                model.habit_view.selected += 1;
            }
        }
        UiMessage::HabitToggleToday => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_view.selected) {
                let today = chrono::Utc::now().date_naive();
                if let Some(habit) = model.habits.get_mut(&habit_id) {
                    let currently_completed = habit.is_completed_on(today);
                    habit.check_in(today, !currently_completed, None);
                }
                model.sync_habit_by_id(&habit_id);
            }
        }
        UiMessage::ShowHabitAnalytics => {
            model.habit_view.show_analytics = true;
        }
        UiMessage::HideHabitAnalytics => {
            model.habit_view.show_analytics = false;
        }
        UiMessage::HabitArchive => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_view.selected) {
                if let Some(habit) = model.habits.get_mut(&habit_id) {
                    habit.archived = true;
                    habit.updated_at = chrono::Utc::now();
                }
                model.sync_habit_by_id(&habit_id);
                model.refresh_visible_habits();
            }
        }
        UiMessage::HabitDelete => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_view.selected) {
                model.habits.remove(&habit_id);
                model.delete_habit_from_storage(&habit_id);
                model.refresh_visible_habits();
            }
        }
        UiMessage::HabitToggleShowArchived => {
            model.habit_view.show_archived = !model.habit_view.show_archived;
            model.refresh_visible_habits();
        }
        _ => {}
    }
}
