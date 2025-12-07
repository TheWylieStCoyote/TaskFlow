//! System message tests (quit, resize).

use crate::app::{update::update, Message, Model, RunningState, SystemMessage, TimeMessage};

use super::create_test_model_with_tasks;

#[test]
fn test_system_quit_stops_timer() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    model.start_time_tracking(task_id);

    assert!(model.active_time_entry.is_some());

    update(&mut model, Message::System(SystemMessage::Quit));

    assert!(model.active_time_entry.is_none());
    assert_eq!(model.running, RunningState::Quitting);
}

#[test]
fn test_system_resize() {
    let mut model = Model::new();

    update(
        &mut model,
        Message::System(SystemMessage::Resize {
            width: 120,
            height: 40,
        }),
    );

    assert_eq!(model.terminal_size, (120, 40));
}

#[test]
fn test_time_toggle_tracking_start() {
    let mut model = create_test_model_with_tasks();
    assert!(model.active_time_entry.is_none());

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_some());
}

#[test]
fn test_time_toggle_tracking_stop() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    model.start_time_tracking(task_id);

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_none());
}
