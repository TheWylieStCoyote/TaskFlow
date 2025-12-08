//! Project creation and management tests.

use crate::app::{update::update, Message, Model, SystemMessage, UiMessage};
use crate::ui::{InputMode, InputTarget};

#[test]
fn test_start_create_project() {
    let mut model = Model::new();
    assert_eq!(model.input_mode, InputMode::Normal);
    assert_eq!(model.input_target, InputTarget::Task); // Default

    update(&mut model, Message::Ui(UiMessage::StartCreateProject));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.input_target, InputTarget::Project);
    assert!(model.input_buffer.is_empty());
}

#[test]
fn test_submit_input_creates_project() {
    let mut model = Model::new();
    assert!(model.projects.is_empty());

    // Start project creation
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));

    // Type project name
    for c in "My New Project".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Project should be created
    assert_eq!(model.projects.len(), 1);
    let project = model.projects.values().next().unwrap();
    assert_eq!(project.name, "My New Project");

    // Should return to normal mode
    assert_eq!(model.input_mode, InputMode::Normal);
    assert_eq!(model.input_target, InputTarget::Task); // Reset to default
}

#[test]
fn test_cancel_project_creation() {
    let mut model = Model::new();

    // Start project creation
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));

    // Type something
    update(&mut model, Message::Ui(UiMessage::InputChar('T')));

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // No project should be created
    assert!(model.projects.is_empty());
    assert_eq!(model.input_mode, InputMode::Normal);
    assert!(model.input_buffer.is_empty());
}

#[test]
fn test_empty_project_name_not_created() {
    let mut model = Model::new();

    // Start project creation
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));

    // Submit with empty name
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // No project should be created
    assert!(model.projects.is_empty());
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_edit_project_with_undo() {
    use crate::app::FocusPane;
    use crate::app::SIDEBAR_FIRST_PROJECT_INDEX;
    use crate::domain::Project;

    let mut model = Model::new();

    // Create a project
    let project = Project::new("Original Name");
    let project_id = project.id;
    model.projects.insert(project_id, project);

    // Focus sidebar and select project
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = SIDEBAR_FIRST_PROJECT_INDEX;
    model.selected_project = Some(project_id);

    // Start editing project
    update(&mut model, Message::Ui(UiMessage::StartEditProject));

    // Should be in editing mode with project name
    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::EditProject(_)));
    assert_eq!(model.input_buffer, "Original Name");

    // Change the name
    model.input_buffer = "New Name".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Verify name changed
    assert_eq!(model.projects.get(&project_id).unwrap().name, "New Name");

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    // Should be back to original
    assert_eq!(
        model.projects.get(&project_id).unwrap().name,
        "Original Name"
    );

    // Redo
    update(&mut model, Message::System(SystemMessage::Redo));

    // Should be new name again
    assert_eq!(model.projects.get(&project_id).unwrap().name, "New Name");
}

#[test]
fn test_delete_project_with_undo() {
    use crate::app::FocusPane;
    use crate::app::SIDEBAR_FIRST_PROJECT_INDEX;
    use crate::domain::Project;

    let mut model = Model::new();

    // Create a project
    let project = Project::new("To Delete");
    let project_id = project.id;
    model.projects.insert(project_id, project);

    // Focus sidebar and select project
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = SIDEBAR_FIRST_PROJECT_INDEX;
    model.selected_project = Some(project_id);

    // Delete project
    update(&mut model, Message::Ui(UiMessage::DeleteProject));

    // Project should be gone
    assert!(model.projects.is_empty());
    assert!(!model.projects.contains_key(&project_id));

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    // Project should be back
    assert_eq!(model.projects.len(), 1);
    assert!(model.projects.contains_key(&project_id));
    assert_eq!(model.projects.get(&project_id).unwrap().name, "To Delete");

    // Redo
    update(&mut model, Message::System(SystemMessage::Redo));

    // Project should be gone again
    assert!(model.projects.is_empty());
}

#[test]
fn test_edit_project_requires_selection() {
    use crate::app::FocusPane;
    use crate::app::SIDEBAR_SEPARATOR_INDEX;

    let mut model = Model::new();

    // Focus sidebar but select a non-project item (separator)
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX;

    // Try to edit - should not enter editing mode
    update(&mut model, Message::Ui(UiMessage::StartEditProject));

    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_delete_project_requires_selection() {
    use crate::app::FocusPane;
    use crate::domain::Project;

    let mut model = Model::new();

    // Create a project
    let project = Project::new("Existing");
    model.projects.insert(project.id, project);

    // Focus sidebar but select a view item (not project)
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 0; // All Tasks view

    // Try to delete - should do nothing
    update(&mut model, Message::Ui(UiMessage::DeleteProject));

    // Project should still exist
    assert_eq!(model.projects.len(), 1);
}
