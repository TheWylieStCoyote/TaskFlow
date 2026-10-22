//! Timeline selection tests.
//!
//! The timeline draws only the visible tasks that carry a date, so the
//! selection must be bounded by that drawn subset, not by `visible_tasks`.

use chrono::Duration;

use crate::app::{update::update, Message, Model, NavigationMessage, UiMessage, ViewId};
use crate::domain::{today, Priority, Task};

/// Five visible tasks, of which only `dated` carry a due date.
fn timeline_model(dated: usize) -> Model {
    let mut model = Model::new();
    for i in 0..5 {
        let mut task = Task::new(format!("Task {i}"));
        if i < dated {
            task.due_date = Some(today() + Duration::days(i as i64));
        }
        model.tasks.insert(task.id, task);
    }
    model.refresh_visible_tasks();
    model.current_view = ViewId::Timeline;
    model
}

fn nav(model: &mut Model, msg: NavigationMessage) {
    update(model, Message::Navigation(msg));
}

#[test]
fn test_timeline_down_stops_at_last_dated_task() {
    let mut model = timeline_model(2);
    assert_eq!(model.visible_tasks.len(), 5);
    assert_eq!(model.timeline_tasks().len(), 2);

    for _ in 0..3 {
        nav(&mut model, NavigationMessage::TimelineDown);
    }

    assert_eq!(model.timeline_state.selected_task_index, 1);
}

#[test]
fn test_timeline_out_of_range_selection_is_clamped_on_navigation() {
    let mut model = timeline_model(2);
    model.timeline_state.selected_task_index = 7;

    nav(&mut model, NavigationMessage::TimelineDown);
    assert_eq!(model.timeline_state.selected_task_index, 1);

    model.timeline_state.selected_task_index = 7;
    nav(&mut model, NavigationMessage::TimelineUp);
    assert_eq!(
        model.timeline_state.selected_task_index, 0,
        "Up snaps to the last row first, then moves up from it"
    );

    // Any timeline navigation message clamps, not just Up/Down.
    model.timeline_state.selected_task_index = 7;
    nav(&mut model, NavigationMessage::TimelineGoToday);
    assert_eq!(model.timeline_state.selected_task_index, 1);
}

#[test]
fn test_timeline_tasks_order_by_date_then_priority_then_title() {
    let mut model = Model::new();
    let base = today();

    let mut later = Task::new("Alpha later");
    later.scheduled_date = Some(base + Duration::days(3));
    later.priority = Priority::Urgent;

    let mut early_low = Task::new("Beta early low");
    early_low.due_date = Some(base);
    early_low.priority = Priority::Low;

    let mut early_high = Task::new("Gamma early high");
    early_high.due_date = Some(base);
    early_high.priority = Priority::High;

    let mut early_high_a = Task::new("Alpha early high");
    early_high_a.scheduled_date = Some(base);
    early_high_a.priority = Priority::High;

    let mut undated = Task::new("Undated");
    undated.priority = Priority::Urgent;

    for task in [later, early_low, early_high, early_high_a, undated] {
        model.tasks.insert(task.id, task);
    }
    model.refresh_visible_tasks();

    let titles: Vec<&str> = model
        .timeline_tasks()
        .iter()
        .map(|t| t.title.as_str())
        .collect();
    assert_eq!(
        titles,
        [
            "Alpha early high",
            "Gamma early high",
            "Beta early low",
            "Alpha later"
        ]
    );

    let ids = model.timeline_task_ids();
    let from_tasks: Vec<_> = model.timeline_tasks().iter().map(|t| t.id).collect();
    assert_eq!(ids, from_tasks);
}

#[test]
fn test_timeline_down_with_no_dated_tasks_keeps_index_zero() {
    let mut model = timeline_model(0);
    assert!(model.timeline_tasks().is_empty());

    nav(&mut model, NavigationMessage::TimelineDown);
    assert_eq!(model.timeline_state.selected_task_index, 0);

    model.timeline_state.selected_task_index = 4;
    nav(&mut model, NavigationMessage::TimelineDown);
    assert_eq!(model.timeline_state.selected_task_index, 0);
}

#[test]
fn test_timeline_view_selected_opens_drawn_row() {
    let mut model = timeline_model(3);
    model.timeline_state.selected_task_index = 2;
    let expected = model.timeline_task_ids()[2];

    update(&mut model, Message::Ui(UiMessage::TimelineViewSelected));

    assert!(model.focus_mode);
    assert_eq!(model.visible_tasks[model.selected_index], expected);

    // Out of range selection opens nothing.
    let mut model = timeline_model(2);
    model.timeline_state.selected_task_index = 4;
    update(&mut model, Message::Ui(UiMessage::TimelineViewSelected));
    assert!(!model.focus_mode);
}
