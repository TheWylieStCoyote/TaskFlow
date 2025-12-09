//! Habit tracking update handlers.
//!
//! Handles all habit-related operations including creating habits,
//! recording check-ins, and managing habit state.

use chrono::Utc;
use tracing::debug;

use crate::app::{HabitMessage, Model};
use crate::domain::Habit;

/// Handle habit tracking messages.
pub(crate) fn handle_habit(model: &mut Model, msg: HabitMessage) {
    match msg {
        HabitMessage::Create(name) => {
            let habit = Habit::new(name);
            let id = habit.id;
            model.sync_habit(&habit);
            model.habits.insert(id, habit);
            model.refresh_visible_habits();
            debug!(?id, "Created new habit");
        }
        HabitMessage::CheckInToday {
            habit_id,
            completed,
        } => {
            let today = Utc::now().date_naive();
            if let Some(habit) = model.habits.get_mut(&habit_id) {
                habit.check_in(today, completed, None);
            }
            model.sync_habit_by_id(&habit_id);
            debug!(?habit_id, completed, "Recorded habit check-in for today");
        }
        HabitMessage::CheckIn {
            habit_id,
            date,
            completed,
        } => {
            if let Some(habit) = model.habits.get_mut(&habit_id) {
                habit.check_in(date, completed, None);
            }
            model.sync_habit_by_id(&habit_id);
            debug!(?habit_id, ?date, completed, "Recorded habit check-in");
        }
        HabitMessage::ToggleToday(habit_id) => {
            let today = Utc::now().date_naive();
            let completed = if let Some(habit) = model.habits.get_mut(&habit_id) {
                let currently_completed = habit.is_completed_on(today);
                habit.check_in(today, !currently_completed, None);
                !currently_completed
            } else {
                return;
            };
            model.sync_habit_by_id(&habit_id);
            debug!(?habit_id, completed, "Toggled habit check-in");
        }
        HabitMessage::Archive(habit_id) => {
            if let Some(habit) = model.habits.get_mut(&habit_id) {
                habit.archived = true;
                habit.updated_at = Utc::now();
            }
            model.sync_habit_by_id(&habit_id);
            model.refresh_visible_habits();
            debug!(?habit_id, "Archived habit");
        }
        HabitMessage::Unarchive(habit_id) => {
            if let Some(habit) = model.habits.get_mut(&habit_id) {
                habit.archived = false;
                habit.updated_at = Utc::now();
            }
            model.sync_habit_by_id(&habit_id);
            model.refresh_visible_habits();
            debug!(?habit_id, "Unarchived habit");
        }
        HabitMessage::Delete(habit_id) => {
            if model.habits.remove(&habit_id).is_some() {
                model.delete_habit_from_storage(&habit_id);
                model.refresh_visible_habits();
                debug!(?habit_id, "Deleted habit");
            }
        }
        HabitMessage::UpdateName { habit_id, name } => {
            if let Some(habit) = model.habits.get_mut(&habit_id) {
                habit.name = name;
                habit.updated_at = Utc::now();
            }
            model.sync_habit_by_id(&habit_id);
            model.refresh_visible_habits(); // Re-sort since name changed
            debug!(?habit_id, "Updated habit name");
        }
    }
}
