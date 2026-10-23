//! Calendar view and focus tests.

use chrono::{Datelike, NaiveDate};

use crate::app::{
    update::update, Message, Model, NavigationMessage, SystemMessage, TaskMessage, UiMessage,
    ViewId,
};
use crate::domain::today;
use crate::domain::{Task, TaskId, TaskStatus};

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
    let today = today();
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
    let today = today();
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

#[test]
fn test_calendar_prev_month() {
    let mut model = Model::new();
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarPrevMonth),
    );

    assert_eq!(model.calendar_state.month, 5);
    assert_eq!(model.calendar_state.year, 2024);
}

#[test]
fn test_calendar_prev_month_wraps_year() {
    let mut model = Model::new();
    model.calendar_state.year = 2024;
    model.calendar_state.month = 1;
    model.calendar_state.selected_day = Some(15);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarPrevMonth),
    );

    assert_eq!(model.calendar_state.month, 12);
    assert_eq!(model.calendar_state.year, 2023);
}

#[test]
fn test_calendar_next_month() {
    let mut model = Model::new();
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarNextMonth),
    );

    assert_eq!(model.calendar_state.month, 7);
    assert_eq!(model.calendar_state.year, 2024);
}

#[test]
fn test_calendar_next_month_wraps_year() {
    let mut model = Model::new();
    model.calendar_state.year = 2024;
    model.calendar_state.month = 12;
    model.calendar_state.selected_day = Some(15);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarNextMonth),
    );

    assert_eq!(model.calendar_state.month, 1);
    assert_eq!(model.calendar_state.year, 2025);
}

#[test]
fn test_calendar_prev_month_clamps_day() {
    let mut model = Model::new();
    // March has 31 days, but February 2024 has 29 days (leap)
    model.calendar_state.year = 2024;
    model.calendar_state.month = 3;
    model.calendar_state.selected_day = Some(31); // Day 31 doesn't exist in Feb

    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarPrevMonth),
    );

    assert_eq!(model.calendar_state.month, 2);
    assert_eq!(model.calendar_state.selected_day, Some(29)); // Clamped to last day of Feb
}

#[test]
fn test_calendar_up_cross_month() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 3;
    model.calendar_state.selected_day = Some(3); // day <= 7, cross-month
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    // Should move to February 2024
    assert_eq!(model.calendar_state.month, 2);
    assert_eq!(model.calendar_state.year, 2024);
}

#[test]
fn test_calendar_up_same_month() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.calendar_state.month, 6);
    assert_eq!(model.calendar_state.selected_day, Some(8));
}

#[test]
fn test_calendar_up_cross_year() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 1;
    model.calendar_state.selected_day = Some(5); // day <= 7, cross year
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.calendar_state.year, 2023);
    assert_eq!(model.calendar_state.month, 12);
}

#[test]
fn test_calendar_down_cross_month() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6; // June has 30 days
    model.calendar_state.selected_day = Some(25); // 25 + 7 > 30, cross-month
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    // Should move to July 2024
    assert_eq!(model.calendar_state.month, 7);
    assert_eq!(model.calendar_state.year, 2024);
}

#[test]
fn test_calendar_down_same_month() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.calendar_state.month, 6);
    assert_eq!(model.calendar_state.selected_day, Some(22));
}

#[test]
fn test_calendar_down_cross_year() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 12; // December has 31 days
    model.calendar_state.selected_day = Some(29); // 29 + 7 > 31, cross year
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.calendar_state.year, 2025);
    assert_eq!(model.calendar_state.month, 1);
}

#[test]
fn test_calendar_up_no_selected_day() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = None;
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Up)); // Should not panic
}

#[test]
fn test_calendar_down_no_selected_day() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = None;
    model.calendar_state.focus_task_list = false;

    update(&mut model, Message::Navigation(NavigationMessage::Down)); // Should not panic
}

// === ScheduleTaskToDay ===

/// Calendar view pinned to Thursday 2024-03-14 with one task due that day
/// and one task already time-blocked on it.
fn schedule_model() -> (Model, NaiveDate, TaskId, TaskId) {
    let date = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.year = 2024;
    model.calendar_state.month = 3;
    model.calendar_state.selected_day = Some(14);

    let due = Task::new("Due only").with_due_date(date);
    let mut blocked = Task::new("Already blocked");
    blocked.scheduled_date = Some(date);
    blocked.scheduled_start_time = chrono::NaiveTime::from_hms_opt(9, 0, 0);
    let (due_id, blocked_id) = (due.id, blocked.id);
    model.tasks.insert(due_id, due);
    model.tasks.insert(blocked_id, blocked);
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks, vec![blocked_id, due_id]);
    (model, date, due_id, blocked_id)
}

#[test]
fn test_schedule_task_to_day_sets_scheduled_date_with_undo() {
    let (mut model, date, due_id, _) = schedule_model();
    model.selected_index = 1; // "Due only"
    let undo_len = model.undo_stack.len();

    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));

    let task = &model.tasks[&due_id];
    assert_eq!(task.scheduled_date, Some(date));
    assert_eq!(task.due_date, Some(date), "due date is untouched");
    assert_eq!(model.undo_stack.len(), undo_len + 1);
    assert_eq!(
        model.alerts.status_message.as_deref(),
        Some("Scheduled to Thu 14 Mar")
    );

    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks[&due_id].scheduled_date, None);
}

#[test]
fn test_schedule_task_to_day_moves_task_into_scheduled_section_and_keeps_selection() {
    let (mut model, _, due_id, blocked_id) = schedule_model();
    model.selected_index = 1;

    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));

    // Untimed blocks sort after timed ones, so the task stays second
    assert_eq!(model.visible_tasks, vec![blocked_id, due_id]);
    assert_eq!(model.selected_task_id(), Some(due_id));

    // Giving it an earlier time block moves it first; selection follows
    model.tasks.get_mut(&due_id).unwrap().scheduled_start_time =
        chrono::NaiveTime::from_hms_opt(7, 0, 0);
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks, vec![due_id, blocked_id]);
}

#[test]
fn test_schedule_task_to_day_works_from_the_task_list_focus_too() {
    let (mut model, date, due_id, _) = schedule_model();
    model.calendar_state.focus_task_list = true;
    model.selected_index = 1;

    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));

    assert_eq!(model.tasks[&due_id].scheduled_date, Some(date));
}

#[test]
fn test_schedule_task_to_day_is_a_noop_when_already_scheduled_there() {
    let (mut model, _, _, blocked_id) = schedule_model();
    model.selected_index = 0; // "Already blocked"
    let before = model.tasks[&blocked_id].clone();
    let undo_len = model.undo_stack.len();

    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));

    assert_eq!(model.tasks[&blocked_id].updated_at, before.updated_at);
    assert_eq!(model.undo_stack.len(), undo_len, "no undo entry recorded");
    assert!(model.alerts.status_message.is_none());
}

#[test]
fn test_schedule_task_to_day_reschedules_from_another_day_keeping_time_block() {
    let (mut model, date, _, blocked_id) = schedule_model();
    let other = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();
    {
        let task = model.tasks.get_mut(&blocked_id).unwrap();
        task.scheduled_date = Some(other);
        task.due_date = Some(date); // keeps it in the day panel as due-only
    }
    model.refresh_visible_tasks();
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| *id == blocked_id)
        .unwrap();

    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));

    let task = &model.tasks[&blocked_id];
    assert_eq!(task.scheduled_date, Some(date));
    assert_eq!(
        task.scheduled_start_time,
        chrono::NaiveTime::from_hms_opt(9, 0, 0),
        "time block survives the move"
    );
}

#[test]
fn test_schedule_task_to_day_ignored_outside_calendar_view() {
    let (mut model, _, due_id, _) = schedule_model();
    model.current_view = ViewId::TaskList;
    model.refresh_visible_tasks();
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| *id == due_id)
        .unwrap();

    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));

    assert_eq!(model.tasks[&due_id].scheduled_date, None);
    assert!(model.undo_stack.is_empty());
}

#[test]
fn test_schedule_task_to_day_without_selected_day_or_task_does_nothing() {
    let (mut model, _, due_id, _) = schedule_model();
    model.calendar_state.selected_day = None;
    model.refresh_visible_tasks();
    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));
    assert_eq!(model.tasks[&due_id].scheduled_date, None);

    // Day selected but nothing listed on it
    model.calendar_state.selected_day = Some(1);
    model.refresh_visible_tasks();
    assert!(model.visible_tasks.is_empty());
    update(&mut model, Message::Ui(UiMessage::ScheduleTaskToDay));
    assert!(model.undo_stack.is_empty());
}
