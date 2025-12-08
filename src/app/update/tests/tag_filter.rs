//! Tag filter tests.

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::Task;
use crate::ui::{InputMode, InputTarget};

use super::create_test_model_with_tasks;

#[test]
fn test_start_filter_by_tag() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Add tags to task
    model.tasks.get_mut(&task_id).unwrap().tags = vec!["work".to_string(), "urgent".to_string()];

    update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::FilterByTag));
    // Input buffer should show available tags
    assert!(model.input_buffer.contains("Available:"));
    assert!(model.input_buffer.contains("urgent"));
    assert!(model.input_buffer.contains("work"));
}

#[test]
fn test_filter_by_tag_submit() {
    let mut model = Model::new();

    // Create one tagged task and one untagged
    let task_tagged = Task::new("Tagged task").with_tags(vec!["work".to_string()]);
    let task_untagged = Task::new("Untagged task");

    model.tasks.insert(task_tagged.id, task_tagged.clone());
    model.tasks.insert(task_untagged.id, task_untagged);
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 2);

    // Start filter
    update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

    // Type tag to filter
    model.input_buffer = "work".to_string();
    model.cursor_position = 4;

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Only tagged task should be visible
    assert_eq!(model.filter.tags, Some(vec!["work".to_string()]));
    assert_eq!(model.visible_tasks.len(), 1);
    assert!(model.visible_tasks.contains(&task_tagged.id));
}

#[test]
fn test_filter_by_tag_multiple_tags() {
    let mut model = Model::new();

    // Create tasks with different tags
    let task_work =
        Task::new("Work task").with_tags(vec!["work".to_string(), "urgent".to_string()]);
    let task_home = Task::new("Home task").with_tags(vec!["home".to_string()]);
    let task_work_only = Task::new("Work only").with_tags(vec!["work".to_string()]);

    model.tasks.insert(task_work.id, task_work.clone());
    model.tasks.insert(task_home.id, task_home);
    model
        .tasks
        .insert(task_work_only.id, task_work_only.clone());
    model.refresh_visible_tasks();

    // Start filter
    update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

    // Type multiple tags (Any mode will match tasks with either)
    model.input_buffer = "work, urgent".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Both work tasks should be visible (Any mode)
    assert_eq!(model.visible_tasks.len(), 2);
    assert!(model.visible_tasks.contains(&task_work.id));
    assert!(model.visible_tasks.contains(&task_work_only.id));
}

#[test]
fn test_clear_tag_filter() {
    let mut model = Model::new();

    // Add one tagged task and one untagged
    let task_tagged = Task::new("Tagged").with_tags(vec!["work".to_string()]);
    let task_untagged = Task::new("Untagged");

    model.tasks.insert(task_tagged.id, task_tagged);
    model.tasks.insert(task_untagged.id, task_untagged);
    model.refresh_visible_tasks();

    // Set tag filter
    model.filter.tags = Some(vec!["work".to_string()]);
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 1);

    // Clear filter
    update(&mut model, Message::Ui(UiMessage::ClearTagFilter));

    assert!(model.filter.tags.is_none());
    assert_eq!(model.visible_tasks.len(), 2);
}

#[test]
fn test_filter_by_tag_empty_clears() {
    let mut model = create_test_model_with_tasks();

    // Set initial tag filter
    model.filter.tags = Some(vec!["work".to_string()]);

    // Start filter
    update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

    // Clear input
    model.input_buffer.clear();
    model.cursor_position = 0;

    // Submit empty
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Filter should be cleared
    assert!(model.filter.tags.is_none());
}

#[test]
fn test_filter_by_tag_preserves_existing() {
    let mut model = create_test_model_with_tasks();

    // Set initial tag filter
    model.filter.tags = Some(vec!["work".to_string()]);

    // Start filter - should pre-fill with existing
    update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

    assert_eq!(model.input_buffer, "work");
    assert_eq!(model.cursor_position, 4);
}
