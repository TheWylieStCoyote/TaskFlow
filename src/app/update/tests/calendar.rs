//! Calendar view and focus tests.

use chrono::Datelike;

use crate::app::{
    update::update, Message, Model, NavigationMessage, TaskMessage, UiMessage, ViewId,
};
use crate::domain::{Task, TaskStatus};

#[test]
fn test_calendar_focus_toggle() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Calendar;
    model.refresh_visible_tasks();

    // Initially focus should be on calendar grid
    assert!(!model.calendar_state.focus_task_list);

    // Focus task list (should work if there are tasks)
    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarFocusTaskList),
    );

    // Should be focused on task list if there are tasks for the day
    if !model.tasks_for_selected_day().is_empty() {
        assert!(model.calendar_state.focus_task_list);
    }

    // Focus back to grid
    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarFocusGrid),
    );
    assert!(!model.calendar_state.focus_task_list);
}

#[test]
fn test_calendar_task_navigation() {
    let mut model = Model::new();

    // Add multiple tasks for the same day
    let today = chrono::Utc::now().date_naive();
    let task1 = Task::new("Task 1").with_due_date(today);
    let task2 = Task::new("Task 2").with_due_date(today);
    let task3 = Task::new("Task 3").with_due_date(today);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(today.day());
    model.calendar_state.year = today.year();
    model.calendar_state.month = today.month();
    model.refresh_visible_tasks();

    // Focus on task list
    model.calendar_state.focus_task_list = true;
    model.selected_index = 0;

    // Navigate down
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.selected_index, 1);

    // Navigate down again
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.selected_index, 2);

    // Navigate down at end should stay at end
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.selected_index, 2);

    // Navigate up
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.selected_index, 1);
}

#[test]
fn test_calendar_focus_reset_on_day_change() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(15);
    model.calendar_state.focus_task_list = true;

    // Select a new day
    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarSelectDay(20)),
    );

    // Focus should be reset to grid
    assert!(!model.calendar_state.focus_task_list);
    assert_eq!(model.calendar_state.selected_day, Some(20));
}

#[test]
fn test_calendar_focus_only_with_tasks() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(15);
    model.refresh_visible_tasks();

    // No tasks for the day, focus should not switch
    assert!(!model.calendar_state.focus_task_list);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarFocusTaskList),
    );

    // Should still be on grid since there are no tasks
    assert!(!model.calendar_state.focus_task_list);
}

#[test]
fn test_calendar_task_actions_when_focused() {
    let mut model = Model::new();

    // Add a task for today
    let today = chrono::Utc::now().date_naive();
    let task = Task::new("Test task").with_due_date(today);
    let task_id = task.id;
    model.tasks.insert(task_id, task);

    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(today.day());
    model.calendar_state.year = today.year();
    model.calendar_state.month = today.month();
    model.calendar_state.focus_task_list = true;
    model.refresh_visible_tasks();
    model.selected_index = 0;

    // Task should be Todo initially
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

    // Toggle complete
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Task should now be Done
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);
}

#[test]
fn test_calendar_prev_day() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);

    update(&mut model, Message::Ui(UiMessage::CalendarPrevDay));

    assert_eq!(model.calendar_state.selected_day, Some(14));
}

#[test]
fn test_calendar_prev_day_wraps_month() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(1);

    update(&mut model, Message::Ui(UiMessage::CalendarPrevDay));

    // Should wrap to May 31
    assert_eq!(model.calendar_state.month, 5);
    assert_eq!(model.calendar_state.selected_day, Some(31));
}

#[test]
fn test_calendar_prev_day_wraps_year() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 1;
    model.calendar_state.selected_day = Some(1);

    update(&mut model, Message::Ui(UiMessage::CalendarPrevDay));

    // Should wrap to December 31 of previous year
    assert_eq!(model.calendar_state.year, 2023);
    assert_eq!(model.calendar_state.month, 12);
    assert_eq!(model.calendar_state.selected_day, Some(31));
}

#[test]
fn test_calendar_next_day() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);

    update(&mut model, Message::Ui(UiMessage::CalendarNextDay));

    assert_eq!(model.calendar_state.selected_day, Some(16));
}

#[test]
fn test_calendar_next_day_wraps_month() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(30); // June has 30 days

    update(&mut model, Message::Ui(UiMessage::CalendarNextDay));

    // Should wrap to July 1
    assert_eq!(model.calendar_state.month, 7);
    assert_eq!(model.calendar_state.selected_day, Some(1));
}

#[test]
fn test_calendar_next_day_wraps_year() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 12;
    model.calendar_state.selected_day = Some(31);

    update(&mut model, Message::Ui(UiMessage::CalendarNextDay));

    // Should wrap to January 1 of next year
    assert_eq!(model.calendar_state.year, 2025);
    assert_eq!(model.calendar_state.month, 1);
    assert_eq!(model.calendar_state.selected_day, Some(1));
}

#[test]
fn test_calendar_nav_does_nothing_outside_calendar_view() {
    let mut model = Model::new();
    model.current_view = ViewId::TaskList; // Not calendar
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);

    update(&mut model, Message::Ui(UiMessage::CalendarNextDay));

    // Should not change
    assert_eq!(model.calendar_state.selected_day, Some(15));
}

#[test]
fn test_calendar_nav_with_no_selected_day() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = None;

    update(&mut model, Message::Ui(UiMessage::CalendarNextDay));

    // Should stay None
    assert!(model.calendar_state.selected_day.is_none());
}
