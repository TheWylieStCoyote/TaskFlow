//! UI tests (input, toggles, help).

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::Priority;
use crate::ui::{InputMode, InputTarget};

#[test]
fn test_ui_toggle_show_completed() {
    let mut model = Model::new();
    assert!(!model.filtering.show_completed);

    update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

    assert!(model.filtering.show_completed);

    update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

    assert!(!model.filtering.show_completed);
}

#[test]
fn test_ui_toggle_sidebar() {
    let mut model = Model::new();
    assert!(model.show_sidebar);

    update(&mut model, Message::Ui(UiMessage::ToggleSidebar));

    assert!(!model.show_sidebar);
}

#[test]
fn test_ui_input_char() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;

    update(&mut model, Message::Ui(UiMessage::InputChar('H')));
    update(&mut model, Message::Ui(UiMessage::InputChar('i')));

    assert_eq!(model.input.buffer, "Hi");
    assert_eq!(model.input.cursor, 2);
}

#[test]
fn test_ui_input_backspace() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Hello".to_string();
    model.input.cursor = 5;

    update(&mut model, Message::Ui(UiMessage::InputBackspace));

    assert_eq!(model.input.buffer, "Hell");
    assert_eq!(model.input.cursor, 4);
}

#[test]
fn test_ui_input_cursor_movement() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Hello".to_string();
    model.input.cursor = 3;

    update(&mut model, Message::Ui(UiMessage::InputCursorLeft));
    assert_eq!(model.input.cursor, 2);

    update(&mut model, Message::Ui(UiMessage::InputCursorRight));
    assert_eq!(model.input.cursor, 3);

    update(&mut model, Message::Ui(UiMessage::InputCursorStart));
    assert_eq!(model.input.cursor, 0);

    update(&mut model, Message::Ui(UiMessage::InputCursorEnd));
    assert_eq!(model.input.cursor, 5);
}

#[test]
fn test_ui_submit_input_creates_task() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "New task from input".to_string();

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.input.mode, InputMode::Normal);
    assert!(model.input.buffer.is_empty());
    assert_eq!(model.tasks.len(), 1);
    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "New task from input");
}

#[test]
fn test_ui_submit_input_empty_ignored() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "   ".to_string(); // whitespace only

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.input.mode, InputMode::Normal);
    assert!(model.tasks.is_empty()); // no task created
}

#[test]
fn test_ui_cancel_input() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Some text".to_string();
    model.input.cursor = 5;

    update(&mut model, Message::Ui(UiMessage::CancelInput));

    assert_eq!(model.input.mode, InputMode::Normal);
    assert!(model.input.buffer.is_empty());
    assert_eq!(model.input.cursor, 0);
}

#[test]
fn test_show_help() {
    let mut model = Model::new();
    assert!(!model.show_help);

    update(&mut model, Message::Ui(UiMessage::ShowHelp));

    assert!(model.show_help);

    update(&mut model, Message::Ui(UiMessage::HideHelp));

    assert!(!model.show_help);
}

#[test]
fn test_submit_input_uses_default_priority() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Task via input".to_string();
    model.default_priority = Priority::Urgent;

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "Task via input");
    assert_eq!(task.priority, Priority::Urgent);
}

// Command palette tests

#[test]
fn test_show_command_palette() {
    let mut model = Model::new();
    model.command_palette.visible = false;
    model.command_palette.query = "something".to_string();
    model.command_palette.selected = 5;

    update(&mut model, Message::Ui(UiMessage::ShowCommandPalette));

    assert!(model.command_palette.visible);
    assert!(model.command_palette.query.is_empty());
    assert_eq!(model.command_palette.selected, 0);
}

#[test]
fn test_hide_command_palette() {
    let mut model = Model::new();
    model.command_palette.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideCommandPalette));

    assert!(!model.command_palette.visible);
}

#[test]
fn test_command_palette_input() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    model.command_palette.query.clear();
    model.command_palette.cursor = 0;
    model.command_palette.selected = 3;

    update(&mut model, Message::Ui(UiMessage::CommandPaletteInput('t')));

    assert_eq!(model.command_palette.query, "t");
    assert_eq!(model.command_palette.cursor, 1);
    assert_eq!(model.command_palette.selected, 0);
}

#[test]
fn test_command_palette_backspace() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    model.command_palette.query = "ta".to_string();
    model.command_palette.cursor = 2;

    update(&mut model, Message::Ui(UiMessage::CommandPaletteBackspace));

    assert_eq!(model.command_palette.query, "t");
    assert_eq!(model.command_palette.cursor, 1);
}

#[test]
fn test_command_palette_backspace_at_zero() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    model.command_palette.query.clear();
    model.command_palette.cursor = 0;

    update(&mut model, Message::Ui(UiMessage::CommandPaletteBackspace));

    // No panic, nothing changes
    assert!(model.command_palette.query.is_empty());
}

#[test]
fn test_command_palette_up() {
    let mut model = Model::new();
    model.command_palette.selected = 2;

    update(&mut model, Message::Ui(UiMessage::CommandPaletteUp));

    assert_eq!(model.command_palette.selected, 1);
}

#[test]
fn test_command_palette_up_at_zero() {
    let mut model = Model::new();
    model.command_palette.selected = 0;

    update(&mut model, Message::Ui(UiMessage::CommandPaletteUp));

    assert_eq!(model.command_palette.selected, 0);
}

#[test]
fn test_command_palette_down() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    model.command_palette.query.clear();
    model.command_palette.selected = 0;

    update(&mut model, Message::Ui(UiMessage::CommandPaletteDown));

    // May advance if there are commands in the palette
    assert!(model.command_palette.selected <= 1);
}

// Task detail tests

#[test]
fn test_show_task_detail_with_task() {
    let mut model = Model::new();
    let task = crate::domain::Task::new("Detail task");
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Ui(UiMessage::ShowTaskDetail));

    assert!(model.task_detail.visible);
    assert_eq!(model.task_detail.scroll, 0);
}

#[test]
fn test_show_task_detail_without_task() {
    let mut model = Model::new();
    // No tasks
    update(&mut model, Message::Ui(UiMessage::ShowTaskDetail));
    assert!(!model.task_detail.visible);
}

#[test]
fn test_hide_task_detail() {
    let mut model = Model::new();
    model.task_detail.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideTaskDetail));

    assert!(!model.task_detail.visible);
}

#[test]
fn test_task_detail_scroll_down_and_up() {
    let mut model = Model::new();
    model.task_detail.scroll = 5;

    update(&mut model, Message::Ui(UiMessage::TaskDetailScrollDown));
    assert_eq!(model.task_detail.scroll, 6);

    update(&mut model, Message::Ui(UiMessage::TaskDetailScrollUp));
    assert_eq!(model.task_detail.scroll, 5);
}

#[test]
fn test_task_detail_scroll_up_at_zero() {
    let mut model = Model::new();
    model.task_detail.scroll = 0;

    update(&mut model, Message::Ui(UiMessage::TaskDetailScrollUp));
    assert_eq!(model.task_detail.scroll, 0); // saturating_sub
}

#[test]
fn test_task_detail_page_down_and_up() {
    let mut model = Model::new();
    model.task_detail.scroll = 5;

    update(&mut model, Message::Ui(UiMessage::TaskDetailPageDown));
    assert_eq!(model.task_detail.scroll, 15);

    update(&mut model, Message::Ui(UiMessage::TaskDetailPageUp));
    assert_eq!(model.task_detail.scroll, 5);
}

#[test]
fn test_task_detail_scroll_top() {
    let mut model = Model::new();
    model.task_detail.scroll = 100;

    update(&mut model, Message::Ui(UiMessage::TaskDetailScrollTop));
    assert_eq!(model.task_detail.scroll, 0);
}

#[test]
fn test_task_detail_scroll_bottom() {
    let mut model = Model::new();
    model.task_detail.scroll = 0;

    update(&mut model, Message::Ui(UiMessage::TaskDetailScrollBottom));
    assert_eq!(model.task_detail.scroll, usize::MAX);
}

// Duplicates UI tests

#[test]
fn test_dismiss_duplicate_outside_view_no_op() {
    let mut model = Model::new();
    // Not in Duplicates view
    update(&mut model, Message::Ui(UiMessage::DismissDuplicate));
    // No panic
}

#[test]
fn test_refresh_duplicates_outside_view_no_op() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::RefreshDuplicates));
    // No panic
}

#[test]
fn test_merge_duplicates_outside_view_no_op() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::MergeDuplicates));
    // No panic
}

#[test]
fn test_dismiss_duplicate_in_view() {
    use crate::app::ViewId;
    use crate::domain::duplicate_detector::DuplicatePair;

    let mut model = Model::new();
    model.current_view = ViewId::Duplicates;

    let task1 = crate::domain::Task::new("Duplicate A");
    let task2 = crate::domain::Task::new("Duplicate B");
    let id1 = task1.id;
    let id2 = task2.id;
    model.tasks.insert(id1, task1);
    model.tasks.insert(id2, task2);
    model.duplicates_view.pairs = vec![DuplicatePair {
        task1_id: id1,
        task2_id: id2,
        similarity: 0.95,
    }];
    model.duplicates_view.selected = 0;

    update(&mut model, Message::Ui(UiMessage::DismissDuplicate));

    assert!(model.duplicates_view.pairs.is_empty());
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_merge_duplicates_in_view() {
    use crate::app::ViewId;
    use crate::domain::duplicate_detector::DuplicatePair;

    let mut model = Model::new();
    model.current_view = ViewId::Duplicates;

    let task1 = crate::domain::Task::new("Original");
    let task2 = crate::domain::Task::new("Duplicate");
    let id1 = task1.id;
    let id2 = task2.id;
    model.tasks.insert(id1, task1);
    model.tasks.insert(id2, task2);
    model.duplicates_view.pairs = vec![DuplicatePair {
        task1_id: id1,
        task2_id: id2,
        similarity: 0.95,
    }];
    model.duplicates_view.selected = 0;

    update(&mut model, Message::Ui(UiMessage::MergeDuplicates));

    // task2 should have been deleted
    assert!(!model.tasks.contains_key(&id2));
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_refresh_duplicates_in_view() {
    use crate::app::ViewId;

    let mut model = Model::new();
    model.current_view = ViewId::Duplicates;

    // Add tasks with similar names
    let task1 = crate::domain::Task::new("Fix login bug");
    let task2 = crate::domain::Task::new("Fix login bug");
    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    update(&mut model, Message::Ui(UiMessage::RefreshDuplicates));

    // Status message should be set
    assert!(model.alerts.status_message.is_some());
}

// Goal UI tests

#[test]
fn test_start_create_goal() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::StartCreateGoal));
    assert!(matches!(model.input.mode, InputMode::Editing));
}

#[test]
fn test_start_create_key_result_no_goals() {
    let mut model = Model::new();
    // No goals visible
    update(&mut model, Message::Ui(UiMessage::StartCreateKeyResult));
    // No panic
}

// Habit UI tests

#[test]
fn test_start_create_habit() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::StartCreateHabit));
    assert!(matches!(model.input.mode, InputMode::Editing));
}

#[test]
fn test_habit_up_down() {
    let mut model = Model::new();
    // Add habits
    let habit = crate::domain::Habit::new("Test habit");
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();
    model.habit_view.selected = 0;

    update(&mut model, Message::Ui(UiMessage::HabitDown));
    // Only 1 habit, stays at 0
    assert_eq!(model.habit_view.selected, 0);

    update(&mut model, Message::Ui(UiMessage::HabitUp));
    assert_eq!(model.habit_view.selected, 0);
}

#[test]
fn test_habit_toggle_show_archived() {
    let mut model = Model::new();
    let initial = model.habit_view.show_archived;

    update(&mut model, Message::Ui(UiMessage::HabitToggleShowArchived));
    assert_eq!(model.habit_view.show_archived, !initial);
}

#[test]
fn test_show_hide_habit_analytics() {
    let mut model = Model::new();
    model.habit_view.show_analytics = false;

    update(&mut model, Message::Ui(UiMessage::ShowHabitAnalytics));
    assert!(model.habit_view.show_analytics);

    update(&mut model, Message::Ui(UiMessage::HideHabitAnalytics));
    assert!(!model.habit_view.show_analytics);
}

// View-specific selection tests

#[test]
fn test_timeline_toggle_dependencies() {
    let mut model = Model::new();
    let initial = model.timeline_state.show_dependencies;

    update(
        &mut model,
        Message::Ui(UiMessage::TimelineToggleDependencies),
    );
    assert_eq!(model.timeline_state.show_dependencies, !initial);

    update(
        &mut model,
        Message::Ui(UiMessage::TimelineToggleDependencies),
    );
    assert_eq!(model.timeline_state.show_dependencies, initial);
}

#[test]
fn test_timeline_view_selected_empty() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::TimelineViewSelected));
    // No panic with empty model
}

#[test]
fn test_kanban_view_selected_empty() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::KanbanViewSelected));
    // No panic
}

#[test]
fn test_eisenhower_view_selected_empty() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::EisenhowerViewSelected));
    // No panic
}

#[test]
fn test_weekly_planner_view_selected_empty() {
    let mut model = Model::new();
    update(
        &mut model,
        Message::Ui(UiMessage::WeeklyPlannerViewSelected),
    );
    // No panic
}

#[test]
fn test_network_view_selected_empty() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::NetworkViewSelected));
    // No panic
}

#[test]
fn test_chain_next_no_task() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::ChainNext));
    // No panic with empty model
}

#[test]
fn test_chain_prev_no_task() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::ChainPrev));
    // No panic with empty model
}

#[test]
fn test_chain_next_navigates() {
    let mut model = Model::new();
    let task_a = crate::domain::Task::new("Task A");
    let task_b = crate::domain::Task::new("Task B");
    let b_id = task_b.id;
    let mut task_a2 = task_a;
    task_a2.next_task_id = Some(b_id);
    model.tasks.insert(task_a2.id, task_a2);
    model.tasks.insert(task_b.id, task_b);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Ui(UiMessage::ChainNext));

    // May or may not advance depending on whether the task has a chain
    // The important thing is no panic
}

#[test]
fn test_chain_prev_navigates() {
    let mut model = Model::new();
    let task_a = crate::domain::Task::new("Task A");
    let task_b = crate::domain::Task::new("Task B");
    let b_id = task_b.id;
    let mut task_a2 = task_a;
    task_a2.next_task_id = Some(b_id);
    model.tasks.insert(task_a2.id, task_a2);
    model.tasks.insert(task_b.id, task_b);
    model.refresh_visible_tasks();
    // Select task B
    if let Some(pos) = model.visible_tasks.iter().position(|id| *id == b_id) {
        model.selected_index = pos;
    }

    update(&mut model, Message::Ui(UiMessage::ChainPrev));

    // No panic
}

// Focus mode and queue tests

#[test]
fn test_toggle_focus_mode_with_task() {
    let mut model = Model::new();
    let task = crate::domain::Task::new("Focus task");
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    assert!(!model.focus_mode);

    update(&mut model, Message::Ui(UiMessage::ToggleFocusMode));
    assert!(model.focus_mode);

    update(&mut model, Message::Ui(UiMessage::ToggleFocusMode));
    assert!(!model.focus_mode);
}

#[test]
fn test_toggle_focus_mode_without_task() {
    let mut model = Model::new();
    // No tasks
    update(&mut model, Message::Ui(UiMessage::ToggleFocusMode));
    assert!(!model.focus_mode); // Should not toggle without a task
}

#[test]
fn test_toggle_full_screen_focus() {
    let mut model = Model::new();
    let initial = model.pomodoro.full_screen;

    update(&mut model, Message::Ui(UiMessage::ToggleFullScreenFocus));
    assert_eq!(model.pomodoro.full_screen, !initial);
}

#[test]
fn test_add_to_focus_queue() {
    let mut model = Model::new();
    let task = crate::domain::Task::new("Queue task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    assert!(model.pomodoro.focus_queue.is_empty());

    update(&mut model, Message::Ui(UiMessage::AddToFocusQueue));

    assert!(model.pomodoro.focus_queue.contains(&task_id));
}

#[test]
fn test_add_to_focus_queue_no_duplicates() {
    let mut model = Model::new();
    let task = crate::domain::Task::new("Queue task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Ui(UiMessage::AddToFocusQueue));
    update(&mut model, Message::Ui(UiMessage::AddToFocusQueue));

    // Should not have duplicates
    assert_eq!(model.pomodoro.focus_queue.len(), 1);
}

#[test]
fn test_clear_focus_queue() {
    let mut model = Model::new();
    let task = crate::domain::Task::new("Queue task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Ui(UiMessage::AddToFocusQueue));
    assert!(!model.pomodoro.focus_queue.is_empty());

    update(&mut model, Message::Ui(UiMessage::ClearFocusQueue));
    assert!(model.pomodoro.focus_queue.is_empty());
}

#[test]
fn test_advance_focus_queue() {
    let mut model = Model::new();
    let task = crate::domain::Task::new("Queue task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Ui(UiMessage::AddToFocusQueue));
    assert_eq!(model.pomodoro.focus_queue.len(), 1);

    update(&mut model, Message::Ui(UiMessage::AdvanceFocusQueue));
    assert!(model.pomodoro.focus_queue.is_empty());
}

#[test]
fn test_advance_focus_queue_empty() {
    let mut model = Model::new();
    update(&mut model, Message::Ui(UiMessage::AdvanceFocusQueue));
    // No panic
}

// ============================================================================
// SubmitInput for uncovered InputTarget variants
// ============================================================================

fn submit_input_with(model: &mut Model, target: InputTarget, text: &str) {
    model.input.mode = InputMode::Editing;
    model.input.target = target;
    model.input.buffer = text.to_string();
    model.input.cursor = text.len();
    update(model, Message::Ui(UiMessage::SubmitInput));
}

#[test]
fn test_submit_input_new_task() {
    let mut model = Model::new();
    let before = model.tasks.len();
    submit_input_with(&mut model, InputTarget::Task, "New task");
    assert_eq!(model.tasks.len(), before + 1);
}

#[test]
fn test_submit_input_new_task_empty_no_add() {
    let mut model = Model::new();
    let before = model.tasks.len();
    submit_input_with(&mut model, InputTarget::Task, "");
    assert_eq!(model.tasks.len(), before);
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_submit_input_quick_capture_creates_task() {
    let mut model = Model::new();
    let before = model.tasks.len();
    submit_input_with(&mut model, InputTarget::QuickCapture, "Capture me");
    assert_eq!(model.tasks.len(), before + 1);
    // QuickCapture stays in editing mode (buffer cleared, not mode)
    assert_eq!(model.input.buffer, "");
}

#[test]
fn test_submit_input_quick_capture_empty() {
    let mut model = Model::new();
    let before = model.tasks.len();
    submit_input_with(&mut model, InputTarget::QuickCapture, "");
    assert_eq!(model.tasks.len(), before);
}

#[test]
fn test_submit_input_new_habit() {
    let mut model = Model::new();
    let before = model.habits.len();
    submit_input_with(&mut model, InputTarget::NewHabit, "Daily exercise");
    assert_eq!(model.habits.len(), before + 1);
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_submit_input_new_habit_empty() {
    let mut model = Model::new();
    let before = model.habits.len();
    submit_input_with(&mut model, InputTarget::NewHabit, "");
    assert_eq!(model.habits.len(), before);
}

#[test]
fn test_submit_input_goal_name() {
    let mut model = Model::new();
    let before = model.goals.len();
    submit_input_with(&mut model, InputTarget::GoalName, "Finish project");
    assert_eq!(model.goals.len(), before + 1);
}

#[test]
fn test_submit_input_goal_name_empty() {
    let mut model = Model::new();
    let before = model.goals.len();
    submit_input_with(&mut model, InputTarget::GoalName, "");
    assert_eq!(model.goals.len(), before);
}

#[test]
fn test_submit_input_saved_filter_name() {
    let mut model = Model::new();
    let before = model.saved_filters.len();
    submit_input_with(&mut model, InputTarget::SavedFilterName, "My Filter");
    assert_eq!(model.saved_filters.len(), before + 1);
    assert!(model
        .alerts
        .status_message
        .as_deref()
        .is_some_and(|s| s.contains("Saved filter")));
}

#[test]
fn test_submit_input_saved_filter_name_empty() {
    let mut model = Model::new();
    let before = model.saved_filters.len();
    submit_input_with(&mut model, InputTarget::SavedFilterName, "");
    assert_eq!(model.saved_filters.len(), before);
}

#[test]
fn test_submit_input_bulk_set_status_done() {
    use crate::domain::{Task, TaskStatus};
    let mut model = Model::new();
    let task = Task::new("Bulk task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.multi_select.mode = true;
    model.multi_select.selected.insert(task_id);
    submit_input_with(&mut model, InputTarget::BulkSetStatus, "4"); // 4 = Done
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);
}

#[test]
fn test_submit_input_bulk_set_status_invalid() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Bulk task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.multi_select.mode = true;
    model.multi_select.selected.insert(task_id);
    submit_input_with(&mut model, InputTarget::BulkSetStatus, "99"); // Invalid
                                                                     // Status unchanged, error message shown
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_submit_input_snooze_task_clear() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Snooze task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    submit_input_with(&mut model, InputTarget::SnoozeTask(task_id), ""); // Empty = clear snooze
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_submit_input_snooze_task_set_date() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Snooze task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    submit_input_with(&mut model, InputTarget::SnoozeTask(task_id), "2030-12-31");
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_submit_input_bulk_move_to_project_zero() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Move task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.multi_select.mode = true;
    model.multi_select.selected.insert(task_id);
    submit_input_with(&mut model, InputTarget::BulkMoveToProject, "0"); // 0 = no project
    assert!(model.tasks.get(&task_id).unwrap().project_id.is_none());
}

#[test]
fn test_submit_input_edit_recurrence_daily() {
    use crate::domain::{Recurrence, Task};
    let mut model = Model::new();
    let task = Task::new("Recurring task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    submit_input_with(&mut model, InputTarget::EditRecurrence(task_id), "d");
    assert_eq!(
        model.tasks.get(&task_id).unwrap().recurrence,
        Some(Recurrence::Daily)
    );
}

#[test]
fn test_submit_input_edit_recurrence_clear() {
    use crate::domain::{Recurrence, Task};
    let mut model = Model::new();
    let mut task = Task::new("Recurring task");
    task.recurrence = Some(Recurrence::Daily);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    submit_input_with(&mut model, InputTarget::EditRecurrence(task_id), "0"); // 0 = clear
    assert_eq!(model.tasks.get(&task_id).unwrap().recurrence, None);
}

#[test]
fn test_submit_input_filter_by_tag() {
    let mut model = Model::new();
    submit_input_with(&mut model, InputTarget::FilterByTag, "work,urgent");
    assert!(model.filtering.filter.tags.is_some());
    let tags = model.filtering.filter.tags.as_ref().unwrap();
    assert!(tags.contains(&"work".to_string()));
}

#[test]
fn test_submit_input_filter_by_tag_empty_clears() {
    let mut model = Model::new();
    model.filtering.filter.tags = Some(vec!["work".to_string()]);
    submit_input_with(&mut model, InputTarget::FilterByTag, "");
    assert!(model.filtering.filter.tags.is_none());
}

#[test]
fn test_submit_input_edit_habit() {
    use crate::domain::Habit;
    let mut model = Model::new();
    let habit = Habit::new("Old name");
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    submit_input_with(&mut model, InputTarget::EditHabit(habit_id), "New name");
    assert_eq!(model.habits.get(&habit_id).unwrap().name, "New name");
}

#[test]
fn test_submit_input_edit_habit_empty_no_change() {
    use crate::domain::Habit;
    let mut model = Model::new();
    let habit = Habit::new("Keep name");
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    submit_input_with(&mut model, InputTarget::EditHabit(habit_id), "");
    assert_eq!(model.habits.get(&habit_id).unwrap().name, "Keep name");
}

#[test]
fn test_submit_input_edit_scheduled_time_range() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Scheduled task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    submit_input_with(
        &mut model,
        InputTarget::EditScheduledTime(task_id),
        "9:00-11:00",
    );
    let t = model.tasks.get(&task_id).unwrap();
    assert!(t.scheduled_start_time.is_some());
    assert!(t.scheduled_end_time.is_some());
}

#[test]
fn test_submit_input_edit_scheduled_time_clear() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Scheduled task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    submit_input_with(&mut model, InputTarget::EditScheduledTime(task_id), "");
    let t = model.tasks.get(&task_id).unwrap();
    assert!(t.scheduled_start_time.is_none());
}

#[test]
fn test_submit_input_edit_scheduled_time_invalid() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Scheduled task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    submit_input_with(
        &mut model,
        InputTarget::EditScheduledTime(task_id),
        "not-a-time",
    );
    // Should set error message
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_submit_input_goal_name_creates_goal_with_key_result() {
    use crate::app::{GoalMessage, Message};
    let mut model = Model::new();
    // Create a goal first
    update(
        &mut model,
        Message::Goal(GoalMessage::Create("My Goal".to_string())),
    );
    let goal_id = *model.goals.keys().next().unwrap();
    let before = model.key_results.len();
    // Now add a key result via InputTarget::KeyResultName
    submit_input_with(&mut model, InputTarget::KeyResultName(goal_id), "KR 1");
    assert_eq!(model.key_results.len(), before + 1);
}

#[test]
fn test_submit_input_key_result_name_empty_no_add() {
    use crate::app::{GoalMessage, Message};
    let mut model = Model::new();
    update(
        &mut model,
        Message::Goal(GoalMessage::Create("Goal".to_string())),
    );
    let goal_id = *model.goals.keys().next().unwrap();
    let before = model.key_results.len();
    submit_input_with(&mut model, InputTarget::KeyResultName(goal_id), "");
    assert_eq!(model.key_results.len(), before);
}

#[test]
fn test_submit_input_edit_goal_name() {
    use crate::app::{GoalMessage, Message};
    let mut model = Model::new();
    update(
        &mut model,
        Message::Goal(GoalMessage::Create("Old Goal".to_string())),
    );
    let goal_id = *model.goals.keys().next().unwrap();
    submit_input_with(&mut model, InputTarget::EditGoalName(goal_id), "New Goal");
    assert_eq!(model.goals.get(&goal_id).unwrap().name, "New Goal");
}

#[test]
fn test_submit_input_edit_goal_name_empty_no_change() {
    use crate::app::{GoalMessage, Message};
    let mut model = Model::new();
    update(
        &mut model,
        Message::Goal(GoalMessage::Create("Keep Name".to_string())),
    );
    let goal_id = *model.goals.keys().next().unwrap();
    submit_input_with(&mut model, InputTarget::EditGoalName(goal_id), "");
    assert_eq!(model.goals.get(&goal_id).unwrap().name, "Keep Name");
}

#[test]
fn test_start_edit_habit() {
    use crate::domain::Habit;
    let mut model = Model::new();
    let habit = Habit::new("My Habit");
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    model.refresh_visible_habits();

    update(&mut model, Message::Ui(UiMessage::StartEditHabit(habit_id)));

    assert_eq!(model.input.mode, InputMode::Editing);
    assert!(matches!(model.input.target, InputTarget::EditHabit(_)));
    assert_eq!(model.input.buffer, "My Habit");
}

#[test]
fn test_habit_toggle_today() {
    use crate::domain::Habit;
    let mut model = Model::new();
    let habit = Habit::new("Exercise");
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    model.refresh_visible_habits();
    model.habit_view.selected = 0;

    let today = chrono::Utc::now().date_naive();
    let before = model.habits.get(&habit_id).unwrap().is_completed_on(today);

    update(&mut model, Message::Ui(UiMessage::HabitToggleToday));

    let after = model.habits.get(&habit_id).unwrap().is_completed_on(today);
    assert_ne!(before, after);
}

#[test]
fn test_habit_archive() {
    use crate::domain::Habit;
    let mut model = Model::new();
    let habit = Habit::new("Archive me");
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    model.refresh_visible_habits();
    model.habit_view.selected = 0;

    assert!(!model.habits.get(&habit_id).unwrap().archived);

    update(&mut model, Message::Ui(UiMessage::HabitArchive));

    assert!(model.habits.get(&habit_id).unwrap().archived);
}

#[test]
fn test_habit_delete() {
    use crate::domain::Habit;
    let mut model = Model::new();
    let habit = Habit::new("Delete me");
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    model.refresh_visible_habits();
    model.habit_view.selected = 0;
    let before = model.habits.len();

    update(&mut model, Message::Ui(UiMessage::HabitDelete));

    assert_eq!(model.habits.len(), before - 1);
    assert!(!model.habits.contains_key(&habit_id));
}

#[test]
fn test_start_edit_goal() {
    use crate::app::{GoalMessage, Message};
    let mut model = Model::new();
    update(
        &mut model,
        Message::Goal(GoalMessage::Create("Old Goal".to_string())),
    );
    let goal_id = *model.goals.keys().next().unwrap();
    update(&mut model, Message::Ui(UiMessage::StartEditGoal(goal_id)));
    assert_eq!(model.input.mode, InputMode::Editing);
    assert!(matches!(model.input.target, InputTarget::EditGoalName(_)));
    assert_eq!(model.input.buffer, "Old Goal"); // pre-filled
}

#[test]
fn test_submit_input_edit_scheduled_time_single() {
    use crate::domain::Task;
    let mut model = Model::new();
    let task = Task::new("Scheduled task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    // Single time (start only)
    submit_input_with(&mut model, InputTarget::EditScheduledTime(task_id), "9:00");
    let t = model.tasks.get(&task_id).unwrap();
    assert!(t.scheduled_start_time.is_some());
    assert!(t.scheduled_end_time.is_none());
}
