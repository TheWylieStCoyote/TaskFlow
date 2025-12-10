//! Habit tracking tests.

use chrono::Utc;

use crate::app::{update::update, HabitMessage, Message, Model};
use crate::domain::Habit;

fn create_test_model_with_habit() -> (Model, crate::domain::HabitId) {
    let mut model = Model::new();
    let habit = Habit::new("Test Habit");
    let habit_id = habit.id;
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();
    (model, habit_id)
}

#[test]
fn test_habit_create() {
    let mut model = Model::new();
    assert!(model.habits.is_empty());

    update(
        &mut model,
        Message::Habit(HabitMessage::Create("Exercise".to_string())),
    );

    assert_eq!(model.habits.len(), 1);
    let habit = model.habits.values().next().unwrap();
    assert_eq!(habit.name, "Exercise");
}

#[test]
fn test_habit_check_in_today() {
    let (mut model, habit_id) = create_test_model_with_habit();
    let today = Utc::now().date_naive();

    // Initially not completed
    assert!(!model.habits.get(&habit_id).unwrap().is_completed_on(today));

    update(
        &mut model,
        Message::Habit(HabitMessage::CheckInToday {
            habit_id,
            completed: true,
        }),
    );

    assert!(model.habits.get(&habit_id).unwrap().is_completed_on(today));
}

#[test]
fn test_habit_check_in_specific_date() {
    let (mut model, habit_id) = create_test_model_with_habit();
    let date = Utc::now().date_naive();

    update(
        &mut model,
        Message::Habit(HabitMessage::CheckIn {
            habit_id,
            date,
            completed: true,
        }),
    );

    assert!(model.habits.get(&habit_id).unwrap().is_completed_on(date));
}

#[test]
fn test_habit_toggle_today() {
    let (mut model, habit_id) = create_test_model_with_habit();
    let today = Utc::now().date_naive();

    // Initially not completed
    assert!(!model.habits.get(&habit_id).unwrap().is_completed_on(today));

    // Toggle on
    update(
        &mut model,
        Message::Habit(HabitMessage::ToggleToday(habit_id)),
    );
    assert!(model.habits.get(&habit_id).unwrap().is_completed_on(today));

    // Toggle off
    update(
        &mut model,
        Message::Habit(HabitMessage::ToggleToday(habit_id)),
    );
    assert!(!model.habits.get(&habit_id).unwrap().is_completed_on(today));
}

#[test]
fn test_habit_archive() {
    let (mut model, habit_id) = create_test_model_with_habit();

    assert!(!model.habits.get(&habit_id).unwrap().archived);

    update(&mut model, Message::Habit(HabitMessage::Archive(habit_id)));

    assert!(model.habits.get(&habit_id).unwrap().archived);
}

#[test]
fn test_habit_unarchive() {
    let (mut model, habit_id) = create_test_model_with_habit();

    // First archive it
    model.habits.get_mut(&habit_id).unwrap().archived = true;
    assert!(model.habits.get(&habit_id).unwrap().archived);

    update(
        &mut model,
        Message::Habit(HabitMessage::Unarchive(habit_id)),
    );

    assert!(!model.habits.get(&habit_id).unwrap().archived);
}

#[test]
fn test_habit_delete() {
    let (mut model, habit_id) = create_test_model_with_habit();

    assert!(model.habits.contains_key(&habit_id));

    update(&mut model, Message::Habit(HabitMessage::Delete(habit_id)));

    assert!(!model.habits.contains_key(&habit_id));
}

#[test]
fn test_habit_update_name() {
    let (mut model, habit_id) = create_test_model_with_habit();

    assert_eq!(model.habits.get(&habit_id).unwrap().name, "Test Habit");

    update(
        &mut model,
        Message::Habit(HabitMessage::UpdateName {
            habit_id,
            name: "Renamed Habit".to_string(),
        }),
    );

    assert_eq!(model.habits.get(&habit_id).unwrap().name, "Renamed Habit");
}

#[test]
fn test_habit_toggle_nonexistent() {
    let mut model = Model::new();
    let fake_id = crate::domain::HabitId::new();

    // Should not panic
    update(
        &mut model,
        Message::Habit(HabitMessage::ToggleToday(fake_id)),
    );
}

#[test]
fn test_habit_check_in_nonexistent() {
    let mut model = Model::new();
    let fake_id = crate::domain::HabitId::new();
    let today = Utc::now().date_naive();

    // Should not panic
    update(
        &mut model,
        Message::Habit(HabitMessage::CheckIn {
            habit_id: fake_id,
            date: today,
            completed: true,
        }),
    );
}
