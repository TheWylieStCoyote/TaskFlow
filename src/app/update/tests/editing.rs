//! Task editing tests (title, due date, tags, description).

use chrono::NaiveDate;

use crate::app::{update::update, Message, SystemMessage, UiMessage};
use crate::ui::{InputMode, InputTarget};

use super::create_test_model_with_tasks;

// === Task Title Editing ===

#[test]
fn test_start_edit_task() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let original_title = model.tasks.get(&task_id).unwrap().title.clone();

    update(&mut model, Message::Ui(UiMessage::StartEditTask));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.input_target, InputTarget::EditTask(task_id));
    assert_eq!(model.input_buffer, original_title);
    assert_eq!(model.cursor_position, original_title.len());
}

#[test]
fn test_edit_task_title() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditTask));

    // Clear and type new title
    model.input_buffer.clear();
    model.cursor_position = 0;
    for c in "Updated Title".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Title should be updated
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.title, "Updated Title");
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_cancel_edit_task() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let original_title = model.tasks.get(&task_id).unwrap().title.clone();

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditTask));

    // Type something
    model.input_buffer = "Changed".to_string();

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // Title should NOT be changed
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.title, original_title);
    assert_eq!(model.input_mode, InputMode::Normal);
}

// === Due Date Editing ===

#[test]
fn test_edit_due_date() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing due date
    update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::EditDueDate(_)));

    // Type a date
    model.input_buffer = "2025-12-25".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Due date should be set
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(
        task.due_date,
        Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
    );
}

#[test]
fn test_clear_due_date() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set an initial due date
    model.tasks.get_mut(&task_id).unwrap().due_date =
        Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

    // Start editing due date
    update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

    // Clear the buffer
    model.input_buffer.clear();
    model.cursor_position = 0;

    // Submit empty
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Due date should be cleared
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.due_date.is_none());
}

#[test]
fn test_invalid_due_date_keeps_old() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set an initial due date
    let original_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    model.tasks.get_mut(&task_id).unwrap().due_date = Some(original_date);

    // Start editing due date
    update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

    // Type invalid date
    model.input_buffer = "not-a-date".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Due date should be unchanged
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.due_date, Some(original_date));
}

// === Tag Editing ===

#[test]
fn test_start_edit_tags() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Add some initial tags
    model.tasks.get_mut(&task_id).unwrap().tags = vec!["work".to_string(), "urgent".to_string()];

    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::EditTags(_)));
    assert_eq!(model.input_buffer, "work, urgent");
}

#[test]
fn test_edit_tags_add_new() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Task has no tags initially
    assert!(model.tasks.get(&task_id).unwrap().tags.is_empty());

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type new tags
    model.input_buffer = "feature, bug, priority".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Tags should be set
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, vec!["feature", "bug", "priority"]);
}

#[test]
fn test_edit_tags_clear() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set initial tags
    model.tasks.get_mut(&task_id).unwrap().tags = vec!["work".to_string()];

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Clear input
    model.input_buffer.clear();
    model.cursor_position = 0;

    // Submit empty
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Tags should be cleared
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.tags.is_empty());
}

#[test]
fn test_edit_tags_trims_whitespace() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type tags with extra whitespace
    model.input_buffer = "  work  ,  play  , rest ".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Tags should be trimmed
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, vec!["work", "play", "rest"]);
}

#[test]
fn test_edit_tags_filters_empty() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type tags with empty entries
    model.input_buffer = "work,,, ,play".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Only non-empty tags should remain
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, vec!["work", "play"]);
}

#[test]
fn test_cancel_edit_tags() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set initial tags
    let original_tags = vec!["original".to_string()];
    model.tasks.get_mut(&task_id).unwrap().tags = original_tags.clone();

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type something different
    model.input_buffer = "new, tags, here".to_string();

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // Tags should NOT be changed
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, original_tags);
    assert_eq!(model.input_mode, InputMode::Normal);
}

// === Description Editing ===

#[test]
fn test_start_edit_description_enters_edit_mode() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Task starts with no description
    assert!(model.tasks.get(&task_id).unwrap().description.is_none());

    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(
        model.input_target,
        InputTarget::EditDescription(_)
    ));
    assert!(model.input_buffer.is_empty());
}

#[test]
fn test_start_edit_description_prefills_existing() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set existing description
    model.tasks.get_mut(&task_id).unwrap().description = Some("Existing notes here".to_string());

    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    assert_eq!(model.input_buffer, "Existing notes here");
}

#[test]
fn test_edit_description_add_new() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing description
    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    // Type new description
    model.input_buffer = "This is a detailed task description".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Description should be set
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(
        task.description,
        Some("This is a detailed task description".to_string())
    );
}

#[test]
fn test_edit_description_clear() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set initial description
    model.tasks.get_mut(&task_id).unwrap().description = Some("Old description".to_string());

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    // Clear input
    model.input_buffer.clear();
    model.cursor_position = 0;

    // Submit empty
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Description should be cleared
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.description.is_none());
}

#[test]
fn test_edit_description_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start with no description
    assert!(model.tasks.get(&task_id).unwrap().description.is_none());

    // Add a description
    update(&mut model, Message::Ui(UiMessage::StartEditDescription));
    model.input_buffer = "New description".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Verify description was set
    assert_eq!(
        model.tasks.get(&task_id).unwrap().description,
        Some("New description".to_string())
    );

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    // Description should be gone
    assert!(model.tasks.get(&task_id).unwrap().description.is_none());
}

// === Time Estimate Editing ===

#[test]
fn test_start_edit_estimate() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Task starts with no estimate
    assert!(model
        .tasks
        .get(&task_id)
        .unwrap()
        .estimated_minutes
        .is_none());

    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::EditEstimate(_)));
    // Empty buffer for task with no estimate
    assert_eq!(model.input_buffer, "");
}

#[test]
fn test_edit_estimate_set_minutes() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing estimate
    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));

    // Type a duration
    model.input_buffer = "30m".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Estimate should be set
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.estimated_minutes, Some(30));
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_edit_estimate_set_hours() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));
    model.input_buffer = "2h".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.estimated_minutes, Some(120));
}

#[test]
fn test_edit_estimate_set_hours_and_minutes() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));
    model.input_buffer = "1h30m".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.estimated_minutes, Some(90));
}

#[test]
fn test_edit_estimate_clear() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // First set an estimate
    model.tasks.get_mut(&task_id).unwrap().estimated_minutes = Some(60);

    // Edit and clear
    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));

    // Input buffer should be pre-filled with existing estimate
    assert_eq!(model.input_buffer, "1h");

    // Clear it
    model.input_buffer.clear();
    model.cursor_position = 0;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Estimate should be cleared
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.estimated_minutes.is_none());
}

#[test]
fn test_edit_estimate_invalid_keeps_old() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set an initial estimate
    model.tasks.get_mut(&task_id).unwrap().estimated_minutes = Some(60);

    // Try to set invalid input
    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));
    model.input_buffer = "invalid".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Estimate should remain unchanged
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.estimated_minutes, Some(60));

    // Should show error message
    assert!(model
        .status_message
        .as_ref()
        .is_some_and(|m| m.contains("Invalid")));
}

#[test]
fn test_edit_estimate_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start with no estimate
    assert!(model
        .tasks
        .get(&task_id)
        .unwrap()
        .estimated_minutes
        .is_none());

    // Add an estimate
    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));
    model.input_buffer = "45m".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Verify estimate was set
    assert_eq!(
        model.tasks.get(&task_id).unwrap().estimated_minutes,
        Some(45)
    );

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    // Estimate should be gone
    assert!(model
        .tasks
        .get(&task_id)
        .unwrap()
        .estimated_minutes
        .is_none());

    // Redo
    update(&mut model, Message::System(SystemMessage::Redo));

    // Estimate should be back
    assert_eq!(
        model.tasks.get(&task_id).unwrap().estimated_minutes,
        Some(45)
    );
}

#[test]
fn test_edit_estimate_prefill_existing() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set various estimates and check prefill
    let test_cases = [
        (30, "30m"),
        (60, "1h"),
        (90, "1h30m"),
        (120, "2h"),
        (135, "2h15m"),
    ];

    for (minutes, expected_display) in test_cases {
        model.tasks.get_mut(&task_id).unwrap().estimated_minutes = Some(minutes);

        update(&mut model, Message::Ui(UiMessage::StartEditEstimate));

        assert_eq!(
            model.input_buffer, expected_display,
            "Failed for {minutes} minutes"
        );
        assert_eq!(model.cursor_position, expected_display.len());

        // Cancel to reset
        update(&mut model, Message::Ui(UiMessage::CancelInput));
    }
}

#[test]
fn test_cancel_edit_estimate() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set an estimate
    model.tasks.get_mut(&task_id).unwrap().estimated_minutes = Some(60);

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditEstimate));

    // Type something different
    model.input_buffer = "999m".to_string();

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // Estimate should NOT be changed
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.estimated_minutes, Some(60));
    assert_eq!(model.input_mode, InputMode::Normal);
}
