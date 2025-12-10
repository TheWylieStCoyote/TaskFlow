//! Task chain tests (blocking/linked tasks).

use crate::app::{update::update, Message, Model, SystemMessage, TaskMessage, UiMessage};
use crate::domain::Task;
use crate::ui::{InputMode, InputTarget};

use super::create_test_model_with_tasks;

#[test]
fn test_start_link_task_enters_editing_mode() {
    let mut model = create_test_model_with_tasks();
    assert_eq!(model.input.mode, InputMode::Normal);

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    assert_eq!(model.input.mode, InputMode::Editing);
    assert!(matches!(model.input.target, InputTarget::LinkTask(_)));
}

#[test]
fn test_start_link_task_shows_current_link() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let target_id = model.visible_tasks[1];
    let target_title = model.tasks.get(&target_id).unwrap().title.clone();

    // Set existing link
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id);

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Should show the linked task title
    assert_eq!(
        model.input.buffer,
        format!("Currently linked to: {target_title}")
    );
}

#[test]
fn test_link_task_by_number() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let target_id = model.visible_tasks[2];

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Enter task number "3" (1-indexed)
    model.input.buffer = "3".to_string();
    model.input.cursor = 1;

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should link to the third task
    assert_eq!(
        model.tasks.get(&task_id).unwrap().next_task_id,
        Some(target_id)
    );
}

#[test]
fn test_link_task_by_title_search() {
    let mut model = Model::new();

    // Create tasks with distinct titles
    let task1 = Task::new("First task");
    let task2 = Task::new("Second task");
    let task3 = Task::new("Target unique title");
    let task1_id = task1.id;
    let task3_id = task3.id;

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);
    model.refresh_visible_tasks();

    // Find the visible index for task1
    let task1_visible_idx = model
        .visible_tasks
        .iter()
        .position(|id| *id == task1_id)
        .expect("task1 should be in visible_tasks");
    model.selected_index = task1_visible_idx;

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Enter part of target title
    model.input.buffer = "Target unique".to_string();
    model.input.cursor = model.input.buffer.len();

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should link to the task with matching title
    assert_eq!(
        model.tasks.get(&task1_id).unwrap().next_task_id,
        Some(task3_id)
    );
}

#[test]
fn test_link_task_prevents_self_linking() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Try to link task 1 to itself
    model.input.buffer = "1".to_string();
    model.input.cursor = 1;

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should NOT create self-link
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_link_task_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let target_id = model.visible_tasks[1];

    // Link task
    update(&mut model, Message::Ui(UiMessage::StartLinkTask));
    model.input.buffer = "2".to_string();
    model.input.cursor = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(
        model.tasks.get(&task_id).unwrap().next_task_id,
        Some(target_id)
    );

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_unlink_task_removes_link() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let target_id = model.visible_tasks[1];

    // Set existing link
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id);

    update(&mut model, Message::Ui(UiMessage::UnlinkTask));

    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_unlink_task_when_not_linked_is_noop() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Ensure no link exists
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

    update(&mut model, Message::Ui(UiMessage::UnlinkTask));

    // Should still be None, no error
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_unlink_task_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let target_id = model.visible_tasks[1];

    // Set existing link
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id);

    // Unlink
    update(&mut model, Message::Ui(UiMessage::UnlinkTask));
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(
        model.tasks.get(&task_id).unwrap().next_task_id,
        Some(target_id)
    );
}

#[test]
fn test_completing_chained_task_schedules_next() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let next_id = model.visible_tasks[1];

    // Link tasks
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(next_id);

    // Next task should have no scheduled date initially
    assert!(model.tasks.get(&next_id).unwrap().scheduled_date.is_none());

    // Complete the first task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Next task should now be scheduled for today (local time)
    let today = chrono::Local::now().date_naive();
    assert_eq!(
        model.tasks.get(&next_id).unwrap().scheduled_date,
        Some(today)
    );
}

#[test]
fn test_completing_unchained_task_no_scheduling() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let other_id = model.visible_tasks[1];

    // No link - task is standalone
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

    // Other task has no scheduled date
    assert!(model.tasks.get(&other_id).unwrap().scheduled_date.is_none());

    // Complete the first task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Other task should NOT be scheduled
    assert!(model.tasks.get(&other_id).unwrap().scheduled_date.is_none());
}
