//! Bulk operation tests (multi-select, bulk delete).

use crate::app::{update::update, Message, UiMessage};

use super::create_test_model_with_tasks;

#[test]
fn test_toggle_multi_select() {
    let mut model = create_test_model_with_tasks();

    assert!(!model.multi_select_mode);

    update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));
    assert!(model.multi_select_mode);

    update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));
    assert!(!model.multi_select_mode);
}

#[test]
fn test_toggle_task_selection() {
    let mut model = create_test_model_with_tasks();
    model.multi_select_mode = true;
    let task_id = model.visible_tasks[0];

    assert!(!model.selected_tasks.contains(&task_id));

    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
    assert!(model.selected_tasks.contains(&task_id));

    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
    assert!(!model.selected_tasks.contains(&task_id));
}

#[test]
fn test_toggle_task_selection_not_in_multi_mode() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Not in multi-select mode
    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));

    // Should not select anything
    assert!(!model.selected_tasks.contains(&task_id));
}

#[test]
fn test_select_all() {
    let mut model = create_test_model_with_tasks();
    let task_count = model.visible_tasks.len();

    assert!(!model.multi_select_mode);
    assert!(model.selected_tasks.is_empty());

    update(&mut model, Message::Ui(UiMessage::SelectAll));

    assert!(model.multi_select_mode);
    assert_eq!(model.selected_tasks.len(), task_count);
}

#[test]
fn test_clear_selection() {
    let mut model = create_test_model_with_tasks();
    model.multi_select_mode = true;
    model.selected_tasks = model.visible_tasks.iter().copied().collect();

    update(&mut model, Message::Ui(UiMessage::ClearSelection));

    assert!(!model.multi_select_mode);
    assert!(model.selected_tasks.is_empty());
}

#[test]
fn test_bulk_delete() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Select first two tasks
    model.multi_select_mode = true;
    let task1 = model.visible_tasks[0];
    let task2 = model.visible_tasks[1];
    model.selected_tasks.insert(task1);
    model.selected_tasks.insert(task2);

    update(&mut model, Message::Ui(UiMessage::BulkDelete));

    assert_eq!(model.tasks.len(), initial_count - 2);
    assert!(!model.multi_select_mode);
    assert!(model.selected_tasks.is_empty());
}

#[test]
fn test_bulk_delete_not_in_multi_mode() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Not in multi-select mode
    update(&mut model, Message::Ui(UiMessage::BulkDelete));

    // Nothing should be deleted
    assert_eq!(model.tasks.len(), initial_count);
}

#[test]
fn test_exiting_multi_select_clears_selection() {
    let mut model = create_test_model_with_tasks();
    model.multi_select_mode = true;
    model.selected_tasks = model.visible_tasks.iter().copied().collect();

    // Exit multi-select mode
    update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));

    assert!(!model.multi_select_mode);
    assert!(model.selected_tasks.is_empty());
}
