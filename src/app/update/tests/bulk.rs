//! Bulk operation tests (multi-select, bulk delete).

use crate::app::{update::update, Message, UiMessage};

use super::create_test_model_with_tasks;

#[test]
fn test_toggle_multi_select() {
    let mut model = create_test_model_with_tasks();

    assert!(!model.multi_select.mode);

    update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));
    assert!(model.multi_select.mode);

    update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));
    assert!(!model.multi_select.mode);
}

#[test]
fn test_toggle_task_selection() {
    let mut model = create_test_model_with_tasks();
    model.multi_select.mode = true;
    let task_id = model.visible_tasks[0];

    assert!(!model.multi_select.selected.contains(&task_id));

    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
    assert!(model.multi_select.selected.contains(&task_id));

    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
    assert!(!model.multi_select.selected.contains(&task_id));
}

#[test]
fn test_toggle_task_selection_not_in_multi_mode() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Not in multi-select mode
    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));

    // Should not select anything
    assert!(!model.multi_select.selected.contains(&task_id));
}

#[test]
fn test_select_all() {
    let mut model = create_test_model_with_tasks();
    let task_count = model.visible_tasks.len();

    assert!(!model.multi_select.mode);
    assert!(model.multi_select.selected.is_empty());

    update(&mut model, Message::Ui(UiMessage::SelectAll));

    assert!(model.multi_select.mode);
    assert_eq!(model.multi_select.selected.len(), task_count);
}

#[test]
fn test_clear_selection() {
    let mut model = create_test_model_with_tasks();
    model.multi_select.mode = true;
    model.multi_select.selected = model.visible_tasks.iter().copied().collect();

    update(&mut model, Message::Ui(UiMessage::ClearSelection));

    assert!(!model.multi_select.mode);
    assert!(model.multi_select.selected.is_empty());
}

#[test]
fn test_bulk_delete() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Select first two tasks
    model.multi_select.mode = true;
    let task1 = model.visible_tasks[0];
    let task2 = model.visible_tasks[1];
    model.multi_select.selected.insert(task1);
    model.multi_select.selected.insert(task2);

    update(&mut model, Message::Ui(UiMessage::BulkDelete));

    assert_eq!(model.tasks.len(), initial_count - 2);
    assert!(!model.multi_select.mode);
    assert!(model.multi_select.selected.is_empty());
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
    model.multi_select.mode = true;
    model.multi_select.selected = model.visible_tasks.iter().copied().collect();

    // Exit multi-select mode
    update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));

    assert!(!model.multi_select.mode);
    assert!(model.multi_select.selected.is_empty());
}

// ============================================================================
// Bulk operation entry points (StartBulk*)
// ============================================================================

mod start_bulk {
    use crate::app::{update::update, Message, Model, UiMessage};
    use crate::domain::Project;
    use crate::ui::{InputMode, InputTarget};

    use super::create_test_model_with_tasks;

    fn selected_model() -> Model {
        let mut model = create_test_model_with_tasks();
        model.multi_select.mode = true;
        model.multi_select.selected = model.visible_tasks.iter().take(2).copied().collect();
        model
    }

    fn ui(model: &mut Model, msg: UiMessage) {
        update(model, Message::Ui(msg));
    }

    #[test]
    fn test_start_bulk_move_to_project_lists_projects() {
        let mut model = selected_model();
        let project = Project::new("Alpha");
        model.projects.insert(project.id, project);

        ui(&mut model, UiMessage::StartBulkMoveToProject);

        assert_eq!(model.input.mode, InputMode::Editing);
        assert_eq!(model.input.target, InputTarget::BulkMoveToProject);
        assert_eq!(model.input.buffer, "0: (none), 1: Alpha");
        assert_eq!(model.input.cursor, model.input.buffer.len());
    }

    #[test]
    fn test_start_bulk_set_status_shows_options() {
        let mut model = selected_model();
        ui(&mut model, UiMessage::StartBulkSetStatus);
        assert_eq!(model.input.target, InputTarget::BulkSetStatus);
        assert_eq!(
            model.input.buffer,
            "1: Todo, 2: In Progress, 3: Blocked, 4: Done, 5: Cancelled"
        );
        assert_eq!(model.input.cursor, model.input.buffer.len());
    }

    #[test]
    fn test_start_bulk_set_priority_shows_options() {
        let mut model = selected_model();
        ui(&mut model, UiMessage::StartBulkSetPriority);
        assert_eq!(model.input.target, InputTarget::BulkSetPriority);
        assert_eq!(
            model.input.buffer,
            "0: None, 1: Low, 2: Medium, 3: High, 4: Urgent"
        );
    }

    #[test]
    fn test_start_bulk_add_tags_starts_empty() {
        let mut model = selected_model();
        model.input.buffer = "leftover".to_string();
        ui(&mut model, UiMessage::StartBulkAddTags);
        assert_eq!(model.input.target, InputTarget::BulkAddTags);
        assert!(model.input.buffer.is_empty());
        assert_eq!(model.input.cursor, 0);
    }

    #[test]
    fn test_start_bulk_set_due_date_starts_empty() {
        let mut model = selected_model();
        model.input.buffer = "leftover".to_string();
        ui(&mut model, UiMessage::StartBulkSetDueDate);
        assert_eq!(model.input.target, InputTarget::BulkSetDueDate);
        assert!(model.input.buffer.is_empty());
        assert_eq!(model.input.mode, InputMode::Editing);
    }

    #[test]
    fn test_start_bulk_snooze_starts_empty() {
        let mut model = selected_model();
        model.input.buffer = "leftover".to_string();
        ui(&mut model, UiMessage::StartBulkSnooze);
        assert_eq!(model.input.target, InputTarget::BulkSnooze);
        assert!(model.input.buffer.is_empty());
        assert_eq!(model.input.cursor, 0);
    }

    #[test]
    fn test_start_bulk_requires_multi_select_mode() {
        let mut model = create_test_model_with_tasks();
        model.multi_select.mode = false;
        model.multi_select.selected = model.visible_tasks.iter().copied().collect();

        for msg in [
            UiMessage::StartBulkMoveToProject,
            UiMessage::StartBulkSetStatus,
            UiMessage::StartBulkSetPriority,
            UiMessage::StartBulkAddTags,
            UiMessage::StartBulkSetDueDate,
            UiMessage::StartBulkSnooze,
        ] {
            ui(&mut model, msg);
            assert_eq!(model.input.mode, InputMode::Normal);
            assert_eq!(model.input.target, InputTarget::Task);
        }
    }

    #[test]
    fn test_start_bulk_requires_a_selection() {
        let mut model = create_test_model_with_tasks();
        model.multi_select.mode = true;
        model.multi_select.selected.clear();

        for msg in [
            UiMessage::StartBulkMoveToProject,
            UiMessage::StartBulkSetStatus,
            UiMessage::StartBulkSetPriority,
            UiMessage::StartBulkAddTags,
            UiMessage::StartBulkSetDueDate,
            UiMessage::StartBulkSnooze,
        ] {
            ui(&mut model, msg);
            assert_eq!(model.input.mode, InputMode::Normal);
        }
    }

    #[test]
    fn test_bulk_delete_stops_active_timer_of_deleted_task() {
        let mut model = selected_model();
        let tracked = *model.multi_select.selected.iter().next().unwrap();
        model.start_time_tracking(tracked);
        assert!(model.active_time_entry.is_some());

        ui(&mut model, UiMessage::BulkDelete);

        assert!(model.active_time_entry.is_none());
        assert!(model.time_entries.is_empty());
        assert_eq!(model.tasks.len(), 3);
    }

    #[test]
    fn test_bulk_delete_with_empty_selection_is_noop() {
        let mut model = create_test_model_with_tasks();
        model.multi_select.mode = true;
        ui(&mut model, UiMessage::BulkDelete);
        assert_eq!(model.tasks.len(), 5);
        assert!(model.multi_select.mode);
    }
}
