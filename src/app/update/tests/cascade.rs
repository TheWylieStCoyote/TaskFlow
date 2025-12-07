//! Cascade completion tests (parent/child task completion behavior).

use crate::app::{update::update, Message, Model, SystemMessage, TaskMessage, UiMessage};
use crate::domain::{Task, TaskStatus};

#[test]
fn test_completing_parent_cascades_to_descendants() {
    let mut model = Model::new();

    // Create a 3-level hierarchy: root -> child -> grandchild
    let root = Task::new("Root Task");
    let mut child = Task::new("Child Task");
    child.parent_task_id = Some(root.id.clone());
    let mut grandchild = Task::new("Grandchild Task");
    grandchild.parent_task_id = Some(child.id.clone());

    let root_id = root.id.clone();
    let child_id = child.id.clone();
    let grandchild_id = grandchild.id.clone();

    model.tasks.insert(root.id.clone(), root);
    model.tasks.insert(child.id.clone(), child);
    model.tasks.insert(grandchild.id.clone(), grandchild);
    model.refresh_visible_tasks();

    // Select the root task
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| id == &root_id)
        .unwrap();

    // All tasks should be Todo initially
    assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Todo);
    assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Todo);
    assert_eq!(
        model.tasks.get(&grandchild_id).unwrap().status,
        TaskStatus::Todo
    );

    // Complete the root task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // All tasks should now be Done
    assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);
    assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);
    assert_eq!(
        model.tasks.get(&grandchild_id).unwrap().status,
        TaskStatus::Done
    );
}

#[test]
fn test_uncompleting_parent_does_not_affect_descendants() {
    let mut model = Model::new();
    model.show_completed = true; // Show completed tasks so we can select them

    // Create a hierarchy with all tasks completed
    let mut root = Task::new("Root Task");
    root.status = TaskStatus::Done;
    let mut child = Task::new("Child Task");
    child.parent_task_id = Some(root.id.clone());
    child.status = TaskStatus::Done;

    let root_id = root.id.clone();
    let child_id = child.id.clone();

    model.tasks.insert(root.id.clone(), root);
    model.tasks.insert(child.id.clone(), child);
    model.refresh_visible_tasks();

    // Select the root task
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| id == &root_id)
        .unwrap();

    // Both should be Done
    assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);
    assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);

    // Uncomplete the root task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Root should be Todo, but child stays Done (intentional design)
    assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Todo);
    assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);
}

#[test]
fn test_cascade_completion_undo() {
    let mut model = Model::new();

    // Create a hierarchy: root -> child
    let root = Task::new("Root Task");
    let mut child = Task::new("Child Task");
    child.parent_task_id = Some(root.id.clone());

    let root_id = root.id.clone();
    let child_id = child.id.clone();

    model.tasks.insert(root.id.clone(), root);
    model.tasks.insert(child.id.clone(), child);
    model.refresh_visible_tasks();

    // Select the root task
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| id == &root_id)
        .unwrap();

    // Complete the root (cascades to child)
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);
    assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);

    // Undo should restore child first (last pushed to undo stack)
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Todo);
    assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);

    // Undo again to restore root
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Todo);
}

#[test]
fn test_delete_blocked_for_task_with_subtasks() {
    let mut model = Model::new();

    // Create a parent with a child
    let parent = Task::new("Parent Task");
    let mut child = Task::new("Child Task");
    child.parent_task_id = Some(parent.id.clone());

    let parent_id = parent.id.clone();

    model.tasks.insert(parent.id.clone(), parent);
    model.tasks.insert(child.id.clone(), child);
    model.refresh_visible_tasks();

    // Select the parent task
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| id == &parent_id)
        .unwrap();

    // Try to delete - should be blocked
    update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));

    // Confirm dialog should NOT be shown
    assert!(!model.show_confirm_delete);

    // Error message should be set
    assert!(model.status_message.is_some());
    assert!(model
        .status_message
        .as_ref()
        .unwrap()
        .contains("has subtasks"));
}

#[test]
fn test_delete_allowed_for_task_without_subtasks() {
    let mut model = Model::new();

    // Create a task without children
    let task = Task::new("Standalone Task");
    let task_id = task.id.clone();

    model.tasks.insert(task.id.clone(), task);
    model.refresh_visible_tasks();

    // Select the task
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| id == &task_id)
        .unwrap();

    // Try to delete - should show confirm dialog
    update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));

    // Confirm dialog should be shown
    assert!(model.show_confirm_delete);
}

#[test]
fn test_delete_subtask_allowed() {
    let mut model = Model::new();

    // Create parent -> child hierarchy
    let parent = Task::new("Parent Task");
    let mut child = Task::new("Child Task");
    child.parent_task_id = Some(parent.id.clone());

    let child_id = child.id.clone();

    model.tasks.insert(parent.id.clone(), parent);
    model.tasks.insert(child.id.clone(), child);
    model.refresh_visible_tasks();

    // Select the child task (leaf node)
    model.selected_index = model
        .visible_tasks
        .iter()
        .position(|id| id == &child_id)
        .unwrap();

    // Try to delete child - should be allowed (it has no subtasks)
    update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));

    // Confirm dialog should be shown
    assert!(model.show_confirm_delete);
}
