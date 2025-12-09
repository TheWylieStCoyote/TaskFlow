//! Move to project tests.

use crate::app::{update::update, Message, SystemMessage, UiMessage};
use crate::domain::Project;
use crate::ui::{InputMode, InputTarget};

use super::create_test_model_with_tasks;

#[test]
fn test_start_move_to_project() {
    let mut model = create_test_model_with_tasks();
    let _task_id = model.visible_tasks[0];

    // Add some projects
    let project1 = Project::new("Project Alpha");
    let project2 = Project::new("Project Beta");
    model.projects.insert(project1.id, project1);
    model.projects.insert(project2.id, project2);

    // Start move to project
    update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::MoveToProject(_)));
    // Input buffer should contain project list
    assert!(model.input_buffer.contains("0: (none)"));
}

#[test]
fn test_move_to_project_assign() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Initially no project
    assert!(model.tasks.get(&task_id).unwrap().project_id.is_none());

    // Add a project
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Start move to project
    update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

    // Type "1" to select the first project
    model.input_buffer = "1".to_string();
    model.cursor_position = 1;

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Task should now belong to the project
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.project_id, Some(project_id));
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_move_to_project_remove() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add a project and assign task to it
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);
    model.tasks.get_mut(&task_id).unwrap().project_id = Some(project_id);

    // Start move to project
    update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

    // Type "0" to remove from project
    model.input_buffer = "0".to_string();
    model.cursor_position = 1;

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Task should no longer belong to any project
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.project_id.is_none());
}

#[test]
fn test_move_to_project_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add a project
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Move task to project
    update(&mut model, Message::Ui(UiMessage::StartMoveToProject));
    model.input_buffer = "1".to_string();
    model.cursor_position = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Verify task is in project
    assert_eq!(
        model.tasks.get(&task_id).unwrap().project_id,
        Some(project_id)
    );

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    // Task should no longer be in project
    assert!(model.tasks.get(&task_id).unwrap().project_id.is_none());
}

#[test]
fn test_move_to_project_invalid_input_ignored() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add a project
    let project = Project::new("Test Project");
    model.projects.insert(project.id, project);

    // Start move to project
    update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

    // Type invalid input
    model.input_buffer = "abc".to_string();
    model.cursor_position = 3;

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Task should not have changed
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.project_id.is_none());
}

#[test]
fn test_move_to_project_out_of_range_ignored() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add one project
    let project = Project::new("Test Project");
    model.projects.insert(project.id, project);

    // Start move to project
    update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

    // Type index out of range (99)
    model.input_buffer = "99".to_string();
    model.cursor_position = 2;

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Task should not have changed (out of range index is ignored)
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.project_id.is_none());
}
