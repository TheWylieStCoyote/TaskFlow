//! Undo/redo tests.

use chrono::NaiveDate;

use crate::app::{update::update, Message, Model, SystemMessage, TaskMessage, UiMessage};
use crate::domain::{Priority, TaskStatus};

use super::create_test_model_with_tasks;

// === Undo Tests ===

#[test]
fn test_undo_task_create() {
    let mut model = Model::new();
    assert!(model.tasks.is_empty());
    assert!(model.undo_stack.is_empty());

    // Create a task
    update(
        &mut model,
        Message::Task(TaskMessage::Create("New task".to_string())),
    );

    assert_eq!(model.tasks.len(), 1);
    assert_eq!(model.undo_stack.len(), 1);

    // Undo should remove the task
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.tasks.is_empty());
    assert!(model.undo_stack.is_empty());
}

#[test]
fn test_undo_task_delete() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();
    let task_id = model.visible_tasks[0];
    let original_title = model.tasks.get(&task_id).unwrap().title.clone();

    // Delete the task via confirm dialog path
    model.selected_index = 0;
    update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));
    update(&mut model, Message::Ui(UiMessage::ConfirmDelete));

    assert_eq!(model.tasks.len(), initial_count - 1);
    assert!(!model.tasks.contains_key(&task_id));

    // Undo should restore the task
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(model.tasks.len(), initial_count);
    let restored_task = model.tasks.get(&task_id).unwrap();
    assert_eq!(restored_task.title, original_title);
}

#[test]
fn test_undo_task_toggle_complete() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Task starts as Todo
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

    // Toggle complete
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);

    // Undo should restore to Todo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);
}

#[test]
fn test_undo_task_edit_title() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let original_title = model.tasks.get(&task_id).unwrap().title.clone();

    // Edit the title
    update(&mut model, Message::Ui(UiMessage::StartEditTask));
    model.input_buffer = "Changed Title".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.tasks.get(&task_id).unwrap().title, "Changed Title");

    // Undo should restore original title
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(model.tasks.get(&task_id).unwrap().title, original_title);
}

#[test]
fn test_undo_task_cycle_priority() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Set initial priority
    model.tasks.get_mut(&task_id).unwrap().priority = Priority::None;

    // Cycle priority
    update(&mut model, Message::Task(TaskMessage::CyclePriority));

    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::Low);

    // Undo should restore to None
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::None);
}

#[test]
fn test_undo_project_create() {
    let mut model = Model::new();
    assert!(model.projects.is_empty());

    // Create a project
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));
    for c in "My Project".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.projects.len(), 1);

    // Undo should remove the project
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.projects.is_empty());
}

#[test]
fn test_undo_multiple_actions() {
    let mut model = Model::new();

    // Create three tasks
    for i in 1..=3 {
        update(
            &mut model,
            Message::Task(TaskMessage::Create(format!("Task {i}"))),
        );
    }

    assert_eq!(model.tasks.len(), 3);
    assert_eq!(model.undo_stack.len(), 3);

    // Undo all three
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), 2);

    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), 1);

    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.tasks.is_empty());
    assert!(model.undo_stack.is_empty());
}

#[test]
fn test_undo_empty_stack() {
    let mut model = Model::new();
    assert!(model.undo_stack.is_empty());

    // Undo with empty stack should do nothing
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.undo_stack.is_empty());
}

#[test]
fn test_undo_edit_due_date() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Set initial due date
    let original_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    model.tasks.get_mut(&task_id).unwrap().due_date = Some(original_date);

    // Edit due date
    update(&mut model, Message::Ui(UiMessage::StartEditDueDate));
    model.input_buffer = "2025-12-25".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(
        model.tasks.get(&task_id).unwrap().due_date,
        Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
    );

    // Undo should restore original date
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(
        model.tasks.get(&task_id).unwrap().due_date,
        Some(original_date)
    );
}

#[test]
fn test_undo_edit_tags() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Set initial tags
    model.tasks.get_mut(&task_id).unwrap().tags = vec!["original".to_string()];

    // Edit tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));
    model.input_buffer = "new, tags".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.tasks.get(&task_id).unwrap().tags, vec!["new", "tags"]);

    // Undo should restore original tags
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(
        model.tasks.get(&task_id).unwrap().tags,
        vec!["original".to_string()]
    );
}

// === Redo Tests ===

#[test]
fn test_redo_task_create() {
    let mut model = Model::new();

    // Create a task
    update(
        &mut model,
        Message::Task(TaskMessage::Create("New task".to_string())),
    );
    let task_id = model.visible_tasks[0];
    assert_eq!(model.tasks.len(), 1);

    // Undo should remove the task
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.tasks.is_empty());
    assert!(model.undo_stack.can_redo());

    // Redo should restore the task
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.tasks.len(), 1);
    assert!(model.tasks.contains_key(&task_id));
    assert!(!model.undo_stack.can_redo());
}

#[test]
fn test_redo_task_delete() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();
    let task_id = model.visible_tasks[0];

    // Delete the task
    model.selected_index = 0;
    update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));
    update(&mut model, Message::Ui(UiMessage::ConfirmDelete));
    assert_eq!(model.tasks.len(), initial_count - 1);

    // Undo should restore the task
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), initial_count);

    // Redo should delete it again
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.tasks.len(), initial_count - 1);
    assert!(!model.tasks.contains_key(&task_id));
}

#[test]
fn test_redo_task_modify() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let original_title = model.tasks.get(&task_id).unwrap().title.clone();

    // Edit the title
    update(&mut model, Message::Ui(UiMessage::StartEditTask));
    model.input_buffer = "New Title".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));
    assert_eq!(model.tasks.get(&task_id).unwrap().title, "New Title");

    // Undo should restore original
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.get(&task_id).unwrap().title, original_title);

    // Redo should apply the change again
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.tasks.get(&task_id).unwrap().title, "New Title");
}

#[test]
fn test_redo_project_create() {
    let mut model = Model::new();

    // Create a project
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));
    for c in "My Project".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::SubmitInput));
    assert_eq!(model.projects.len(), 1);
    let project_id = *model.projects.keys().next().unwrap();

    // Undo should remove the project
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.projects.is_empty());

    // Redo should restore the project
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.projects.len(), 1);
    assert!(model.projects.contains_key(&project_id));
}

#[test]
fn test_new_action_clears_redo() {
    let mut model = Model::new();

    // Create and undo a task
    update(
        &mut model,
        Message::Task(TaskMessage::Create("Task 1".to_string())),
    );
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.undo_stack.can_redo());

    // New action should clear redo
    update(
        &mut model,
        Message::Task(TaskMessage::Create("Task 2".to_string())),
    );
    assert!(!model.undo_stack.can_redo());
}

#[test]
fn test_multiple_undo_redo() {
    let mut model = Model::new();

    // Create 3 tasks
    for i in 1..=3 {
        update(
            &mut model,
            Message::Task(TaskMessage::Create(format!("Task {i}"))),
        );
    }
    assert_eq!(model.tasks.len(), 3);

    // Undo all 3
    update(&mut model, Message::System(SystemMessage::Undo));
    update(&mut model, Message::System(SystemMessage::Undo));
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.tasks.is_empty());
    assert_eq!(model.undo_stack.redo_len(), 3);

    // Redo 2
    update(&mut model, Message::System(SystemMessage::Redo));
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.tasks.len(), 2);
    assert_eq!(model.undo_stack.redo_len(), 1);

    // Undo 1
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), 1);
    assert_eq!(model.undo_stack.redo_len(), 2);
}

#[test]
fn test_redo_empty_does_nothing() {
    let mut model = Model::new();
    assert!(!model.undo_stack.can_redo());

    // Redo with empty stack should do nothing
    update(&mut model, Message::System(SystemMessage::Redo));
    assert!(model.tasks.is_empty());
}
