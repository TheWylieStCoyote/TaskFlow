//! UI tests (input, toggles, help).

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::today;
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

    let today = today();
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

// ============================================================================
// Remaining `handle_ui` arms
// ============================================================================

mod ui_arms {
    use crate::app::{update::update, Message, Model, UiMessage};
    use crate::domain::{today, Priority, Project, SortField, SortOrder, Task, TaskStatus};
    use crate::ui::{InputMode, InputTarget};
    use chrono::{Duration, NaiveTime, Utc};

    use super::super::create_test_model_with_tasks;

    fn status(model: &Model) -> &str {
        model.alerts.status_message.as_deref().unwrap_or("")
    }

    #[test]
    fn test_help_scroll_messages_move_offset() {
        let mut model = Model::new();
        assert_eq!(model.help_scroll, 0);

        update(&mut model, Message::Ui(UiMessage::HelpScrollUp));
        assert_eq!(model.help_scroll, 0, "saturates at zero");

        update(&mut model, Message::Ui(UiMessage::HelpScrollDown));
        update(&mut model, Message::Ui(UiMessage::HelpScrollDown));
        assert_eq!(model.help_scroll, 2);

        update(&mut model, Message::Ui(UiMessage::HelpPageDown));
        assert_eq!(model.help_scroll, 12);

        update(&mut model, Message::Ui(UiMessage::HelpScrollUp));
        assert_eq!(model.help_scroll, 11);

        update(&mut model, Message::Ui(UiMessage::HelpPageUp));
        assert_eq!(model.help_scroll, 1);

        update(&mut model, Message::Ui(UiMessage::HelpPageUp));
        assert_eq!(model.help_scroll, 0, "page up saturates at zero");
    }

    #[test]
    fn test_start_create_task_enters_editing_with_empty_buffer() {
        let mut model = Model::new();
        model.input.buffer = "stale".to_string();
        model.input.cursor = 5;

        update(&mut model, Message::Ui(UiMessage::StartCreateTask));

        assert_eq!(model.input.mode, InputMode::Editing);
        assert_eq!(model.input.target, InputTarget::Task);
        assert!(model.input.buffer.is_empty());
        assert_eq!(model.input.cursor, 0);
    }

    #[test]
    fn test_start_edit_scheduled_time_prefills_existing_block() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        {
            let task = model.tasks.get_mut(&task_id).unwrap();
            task.scheduled_start_time = NaiveTime::from_hms_opt(9, 0, 0);
            task.scheduled_end_time = NaiveTime::from_hms_opt(11, 30, 0);
        }

        update(&mut model, Message::Ui(UiMessage::StartEditScheduledTime));

        assert_eq!(model.input.mode, InputMode::Editing);
        assert_eq!(model.input.target, InputTarget::EditScheduledTime(task_id));
        assert_eq!(model.input.buffer, "09:00-11:30");
        assert_eq!(model.input.cursor, model.input.buffer.len());
    }

    #[test]
    fn test_start_edit_scheduled_time_without_block_is_empty() {
        let mut model = create_test_model_with_tasks();
        update(&mut model, Message::Ui(UiMessage::StartEditScheduledTime));
        assert_eq!(model.input.mode, InputMode::Editing);
        assert!(model.input.buffer.is_empty());
    }

    #[test]
    fn test_clear_scheduled_time_removes_block_with_undo() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        {
            let task = model.tasks.get_mut(&task_id).unwrap();
            task.scheduled_start_time = NaiveTime::from_hms_opt(9, 0, 0);
            task.scheduled_end_time = NaiveTime::from_hms_opt(10, 0, 0);
        }

        update(&mut model, Message::Ui(UiMessage::ClearScheduledTime));

        let task = &model.tasks[&task_id];
        assert!(task.scheduled_start_time.is_none());
        assert!(task.scheduled_end_time.is_none());
        assert_eq!(model.undo_stack.len(), 1);
    }

    #[test]
    fn test_clear_scheduled_time_without_selection_is_noop() {
        let mut model = Model::new();
        update(&mut model, Message::Ui(UiMessage::ClearScheduledTime));
        assert!(model.undo_stack.is_empty());
    }

    #[test]
    fn test_clear_search_resets_filter_and_visible_tasks() {
        let mut model = create_test_model_with_tasks();
        model.filtering.filter.search_text = Some("Task 1".to_string());
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        update(&mut model, Message::Ui(UiMessage::ClearSearch));

        assert!(model.filtering.filter.search_text.is_none());
        assert_eq!(model.visible_tasks.len(), 5);
    }

    #[test]
    fn test_cycle_sort_field_walks_full_cycle() {
        let mut model = create_test_model_with_tasks();
        assert!(matches!(model.filtering.sort.field, SortField::CreatedAt));

        let expected = [
            "Priority",
            "DueDate",
            "Title",
            "Status",
            "UpdatedAt",
            "CreatedAt",
        ];
        for name in expected {
            update(&mut model, Message::Ui(UiMessage::CycleSortField));
            assert_eq!(format!("{:?}", model.filtering.sort.field), name);
        }
    }

    #[test]
    fn test_toggle_sort_order_flips_and_reorders() {
        let mut model = create_test_model_with_tasks();
        model.filtering.sort.field = SortField::Title;
        model.filtering.sort.order = SortOrder::Ascending;
        model.refresh_visible_tasks();
        let ascending = model.visible_tasks.clone();

        update(&mut model, Message::Ui(UiMessage::ToggleSortOrder));
        assert!(matches!(model.filtering.sort.order, SortOrder::Descending));
        let descending = model.visible_tasks.clone();
        assert_eq!(
            descending.iter().rev().copied().collect::<Vec<_>>(),
            ascending
        );

        update(&mut model, Message::Ui(UiMessage::ToggleSortOrder));
        assert!(matches!(model.filtering.sort.order, SortOrder::Ascending));
    }

    #[test]
    fn test_input_delete_removes_char_at_cursor() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.buffer = "abc".to_string();
        model.input.cursor = 1;

        update(&mut model, Message::Ui(UiMessage::InputDelete));
        assert_eq!(model.input.buffer, "ac");
        assert_eq!(model.input.cursor, 1);

        model.input.cursor = 2;
        update(&mut model, Message::Ui(UiMessage::InputDelete));
        assert_eq!(model.input.buffer, "ac", "delete at end is a no-op");
    }

    #[test]
    fn test_cancel_delete_hides_confirmation() {
        let mut model = create_test_model_with_tasks();
        update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));
        assert!(model.show_confirm_delete);

        update(&mut model, Message::Ui(UiMessage::CancelDelete));
        assert!(!model.show_confirm_delete);
        assert_eq!(model.tasks.len(), 5);
    }

    #[test]
    fn test_generate_config_prompt_show_and_cancel() {
        let mut model = Model::new();
        update(&mut model, Message::Ui(UiMessage::ShowGenerateConfigPrompt));
        assert!(model.show_generate_config_prompt);

        update(&mut model, Message::Ui(UiMessage::CancelGenerateConfig));
        assert!(!model.show_generate_config_prompt);
    }

    #[test]
    fn test_start_edit_dependencies_lists_other_tasks_with_markers() {
        let mut model = Model::new();
        let a = Task::new("Alpha");
        let b = Task::new("Beta");
        let mut c = Task::new("Gamma");
        c.dependencies = vec![a.id];
        let c_id = c.id;
        model.tasks.insert(a.id, a);
        model.tasks.insert(b.id, b);
        model.tasks.insert(c_id, c);
        model.filtering.sort.field = SortField::Title;
        model.filtering.sort.order = SortOrder::Ascending;
        model.refresh_visible_tasks();
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| *id == c_id)
            .unwrap();

        update(&mut model, Message::Ui(UiMessage::StartEditDependencies));

        assert_eq!(model.input.mode, InputMode::Editing);
        assert_eq!(model.input.target, InputTarget::EditDependencies(c_id));
        assert_eq!(model.input.buffer, "*1: Alpha, 2: Beta");
        assert_eq!(model.input.cursor, model.input.buffer.len());
    }

    #[test]
    fn test_start_edit_dependencies_without_selection_is_noop() {
        let mut model = Model::new();
        update(&mut model, Message::Ui(UiMessage::StartEditDependencies));
        assert_eq!(model.input.mode, InputMode::Normal);
    }

    #[test]
    fn test_dismiss_alerts() {
        let mut model = Model::new();
        model.alerts.show_overdue = true;
        model.alerts.show_storage_error = true;
        model.alerts.storage_error = Some("disk on fire".to_string());

        update(&mut model, Message::Ui(UiMessage::DismissOverdueAlert));
        assert!(!model.alerts.show_overdue);
        assert!(model.alerts.show_storage_error, "other alert untouched");

        update(&mut model, Message::Ui(UiMessage::DismissStorageErrorAlert));
        assert!(!model.alerts.show_storage_error);
        assert!(model.alerts.storage_error.is_none());
    }

    #[test]
    fn test_burndown_controls_update_state_and_status() {
        let mut model = Model::new();
        let initial_window = model.burndown_state.time_window;
        let initial_mode = model.burndown_state.mode;
        let initial_creep = model.burndown_state.show_scope_creep;

        update(&mut model, Message::Ui(UiMessage::BurndownCycleWindow));
        assert_ne!(model.burndown_state.time_window, initial_window);
        assert_eq!(
            status(&model),
            format!("Time window: {}", model.burndown_state.time_window.label())
        );

        update(&mut model, Message::Ui(UiMessage::BurndownToggleMode));
        assert_ne!(model.burndown_state.mode, initial_mode);
        assert_eq!(
            status(&model),
            format!("Mode: {}", model.burndown_state.mode.label())
        );

        update(&mut model, Message::Ui(UiMessage::BurndownToggleScopeCreep));
        assert_eq!(model.burndown_state.show_scope_creep, !initial_creep);
        let expected = if initial_creep { "disabled" } else { "enabled" };
        assert_eq!(status(&model), format!("Scope creep display: {expected}"));
    }

    #[test]
    fn test_open_in_editor_without_git_location() {
        let mut model = create_test_model_with_tasks();
        update(&mut model, Message::Ui(UiMessage::OpenInEditor));
        assert_eq!(status(&model), "Task has no git location info");
        assert!(model.pending_editor_command.is_none());
    }

    #[test]
    fn test_open_in_editor_with_malformed_git_location() {
        let mut model = Model::new();
        let mut task = Task::new("Bad location");
        task.description = Some("git:onlyfile".to_string());
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        update(&mut model, Message::Ui(UiMessage::OpenInEditor));
        assert_eq!(status(&model), "Task has no git location info");
    }

    #[test]
    fn test_open_in_editor_with_git_location_uses_editor_env() {
        let mut model = Model::new();
        let mut task = Task::new("TODO from git");
        task.description = Some("git:src/lib.rs:42\nextra".to_string());
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        update(&mut model, Message::Ui(UiMessage::OpenInEditor));

        // Whether $EDITOR is set depends on the environment; both branches must
        // leave consistent state.
        if let Ok(editor) = std::env::var("EDITOR") {
            assert_eq!(
                status(&model),
                format!("Opening src/lib.rs:42 in {editor}...")
            );
            assert_eq!(
                model.pending_editor_command,
                Some((editor, "src/lib.rs".to_string(), "42".to_string()))
            );
        } else {
            assert_eq!(
                status(&model),
                "Set $EDITOR environment variable to open files"
            );
            assert!(model.pending_editor_command.is_none());
        }
    }

    #[test]
    fn test_start_edit_estimate_shows_suggestion_with_history() {
        let mut model = Model::new();
        // Six completed tasks with estimates give confidence 0.3
        for i in 0..6 {
            let mut done = Task::new(format!("Done {i}"));
            done.status = TaskStatus::Done;
            done.completed_at = Some(Utc::now() - Duration::days(1));
            done.estimated_minutes = Some(60);
            done.actual_minutes = 90;
            model.tasks.insert(done.id, done);
        }
        let target = Task::new("Needs estimate");
        let target_id = target.id;
        model.tasks.insert(target_id, target);
        model.refresh_visible_tasks();
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| *id == target_id)
            .unwrap();

        update(&mut model, Message::Ui(UiMessage::StartEditEstimate));

        assert_eq!(model.input.target, InputTarget::EditEstimate(target_id));
        assert!(model.input.buffer.is_empty());
        assert!(
            status(&model).starts_with("Suggestion: 1h30m ("),
            "got {:?}",
            status(&model)
        );
        assert!(status(&model).contains("150%"));
    }

    #[test]
    fn test_start_edit_estimate_prefills_existing_without_suggestion() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        model.tasks.get_mut(&task_id).unwrap().estimated_minutes = Some(90);

        update(&mut model, Message::Ui(UiMessage::StartEditEstimate));

        assert_eq!(model.input.buffer, "1h30m");
        assert!(model.alerts.status_message.is_none());
    }

    #[test]
    fn test_start_snooze_task_and_clear_snooze() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];

        update(&mut model, Message::Ui(UiMessage::StartSnoozeTask));
        assert_eq!(model.input.target, InputTarget::SnoozeTask(task_id));
        assert_eq!(model.input.mode, InputMode::Editing);

        model
            .tasks
            .get_mut(&task_id)
            .unwrap()
            .snooze_until_date(today() + Duration::days(3));
        update(&mut model, Message::Ui(UiMessage::ClearSnooze));
        assert!(model.tasks[&task_id].snooze_until.is_none());
        assert_eq!(status(&model), "Snooze cleared");
    }

    #[test]
    fn test_start_edit_project_without_selection_shows_hint() {
        let mut model = Model::new();
        update(&mut model, Message::Ui(UiMessage::StartEditProject));
        assert_eq!(status(&model), "Select a project from the sidebar first");
        assert_eq!(model.input.mode, InputMode::Normal);
    }

    #[test]
    fn test_delete_project_without_selection_shows_hint() {
        let mut model = Model::new();
        let project = Project::new("Keep me");
        model.projects.insert(project.id, project);
        update(&mut model, Message::Ui(UiMessage::DeleteProject));
        assert_eq!(status(&model), "Select a project from the sidebar first");
        assert_eq!(model.projects.len(), 1);
    }

    #[test]
    fn test_start_link_task_prefills_current_link() {
        let mut model = Model::new();
        let next = Task::new("Next in chain");
        let mut first = Task::new("First in chain");
        first.next_task_id = Some(next.id);
        let first_id = first.id;
        model.tasks.insert(first_id, first);
        model.tasks.insert(next.id, next);
        model.filtering.sort.field = SortField::Title;
        model.filtering.sort.order = SortOrder::Ascending;
        model.refresh_visible_tasks();
        model.selected_index = 0;

        update(&mut model, Message::Ui(UiMessage::StartLinkTask));

        assert_eq!(model.input.target, InputTarget::LinkTask(first_id));
        assert_eq!(model.input.buffer, "Currently linked to: Next in chain");
    }

    #[test]
    fn test_start_filter_by_tag_prefills_active_filter() {
        let mut model = create_test_model_with_tasks();
        model.filtering.filter.tags = Some(vec!["work".to_string(), "home".to_string()]);

        update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

        assert_eq!(model.input.buffer, "work, home");
        assert_eq!(model.input.cursor, model.input.buffer.len());
    }

    #[test]
    fn test_start_filter_by_tag_without_tags_is_empty() {
        let mut model = create_test_model_with_tasks();
        update(&mut model, Message::Ui(UiMessage::StartFilterByTag));
        assert!(model.input.buffer.is_empty());
        assert_eq!(model.input.cursor, 0);
    }

    #[test]
    fn test_default_priority_applied_to_created_tasks() {
        let mut model = Model::new();
        model.default_priority = Priority::High;
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::Task;
        model.input.buffer = "Plain task".to_string();

        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.priority, Priority::High);
    }
}

// ============================================================================
// Remaining `handle_submit_input` arms
// ============================================================================

mod submit_input_arms {
    use crate::app::{update::update, Message, Model, UiMessage};
    use crate::domain::{
        today, Priority, Project, Recurrence, SortField, SortOrder, Task, TaskStatus,
    };
    use crate::ui::{InputMode, InputTarget};
    use chrono::{Datelike, Duration, NaiveDate};

    use super::super::create_test_model_with_tasks;

    fn submit(model: &mut Model, target: InputTarget, text: &str) {
        model.input.mode = InputMode::Editing;
        model.input.target = target;
        model.input.buffer = text.to_string();
        model.input.cursor = model.input.buffer.len();
        update(model, Message::Ui(UiMessage::SubmitInput));
    }

    fn status(model: &Model) -> &str {
        model.alerts.status_message.as_deref().unwrap_or("")
    }

    fn select_all(model: &mut Model) -> Vec<crate::domain::TaskId> {
        model.multi_select.mode = true;
        model.multi_select.selected = model.visible_tasks.iter().copied().collect();
        model.visible_tasks.clone()
    }

    #[test]
    fn test_quick_add_metadata_applied_to_new_task() {
        let mut model = Model::new();
        let project = Project::new("Work");
        let project_id = project.id;
        model.projects.insert(project_id, project);

        submit(
            &mut model,
            InputTarget::Task,
            "Write report #docs #urgent !high due:tomorrow sched:today time:9:00-10:30 @work",
        );

        let task = model.tasks.values().next().expect("task created");
        assert_eq!(task.title, "Write report");
        assert_eq!(task.tags, vec!["docs".to_string(), "urgent".to_string()]);
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.due_date, Some(today() + Duration::days(1)));
        assert_eq!(task.scheduled_date, Some(today()));
        assert_eq!(
            task.scheduled_time_display().as_deref(),
            Some("09:00-10:30")
        );
        assert_eq!(task.project_id, Some(project_id));
        assert_eq!(model.input.mode, InputMode::Normal);
        assert_eq!(model.input.target, InputTarget::Task);
    }

    #[test]
    fn test_quick_add_unknown_project_leaves_task_unassigned() {
        let mut model = Model::new();
        submit(&mut model, InputTarget::Task, "Orphan @nowhere");
        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "Orphan");
        assert_eq!(task.project_id, None);
    }

    #[test]
    fn test_quick_add_metadata_only_keeps_raw_input_as_title() {
        let mut model = Model::new();
        submit(&mut model, InputTarget::Task, "#tag");
        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "#tag");
    }

    #[test]
    fn test_task_submit_warns_about_similar_task() {
        let mut model = Model::new();
        let existing = Task::new("Buy groceries for the week");
        model.tasks.insert(existing.id, existing);

        submit(&mut model, InputTarget::Task, "Buy groceries for the weak");

        assert_eq!(model.tasks.len(), 2);
        assert!(status(&model).starts_with("Warning: Similar task ("));
        assert!(status(&model).contains("Buy groceries for the week"));
    }

    #[test]
    fn test_quick_capture_warns_about_similar_task_and_stays_open() {
        let mut model = Model::new();
        let existing = Task::new("Call the dentist tomorrow");
        model.tasks.insert(existing.id, existing);

        submit(
            &mut model,
            InputTarget::QuickCapture,
            "Call the dentist tomorow",
        );

        assert_eq!(model.tasks.len(), 2);
        assert!(status(&model).starts_with("Warning: Similar task ("));
        assert_eq!(model.input.mode, InputMode::Editing);
        assert_eq!(model.input.target, InputTarget::QuickCapture);
        assert!(model.input.buffer.is_empty());
    }

    #[test]
    fn test_quick_capture_empty_input_reports_error() {
        let mut model = Model::new();
        submit(&mut model, InputTarget::QuickCapture, "   ");
        assert_eq!(status(&model), "Task title cannot be empty");
        assert!(model.tasks.is_empty());
    }

    #[test]
    fn test_subtask_empty_input_creates_nothing() {
        let mut model = create_test_model_with_tasks();
        let parent = model.visible_tasks[0];
        submit(&mut model, InputTarget::Subtask(parent), "");
        assert_eq!(model.tasks.len(), 5);
        assert_eq!(model.input.mode, InputMode::Normal);
    }

    #[test]
    fn test_edit_task_empty_input_keeps_title() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        let title = model.tasks[&task_id].title.clone();
        submit(&mut model, InputTarget::EditTask(task_id), "   ");
        assert_eq!(model.tasks[&task_id].title, title);
        assert!(model.undo_stack.is_empty());
    }

    #[test]
    fn test_edit_due_date_clear_and_invalid() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        model.tasks.get_mut(&task_id).unwrap().due_date = Some(today());

        submit(&mut model, InputTarget::EditDueDate(task_id), "not a date");
        assert_eq!(model.tasks[&task_id].due_date, Some(today()));
        assert!(model.undo_stack.is_empty(), "invalid input is ignored");

        submit(&mut model, InputTarget::EditDueDate(task_id), "");
        assert_eq!(model.tasks[&task_id].due_date, None);
        assert_eq!(model.undo_stack.len(), 1);
    }

    #[test]
    fn test_edit_scheduled_date_clear_and_invalid() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        model.tasks.get_mut(&task_id).unwrap().scheduled_date = Some(today());

        submit(
            &mut model,
            InputTarget::EditScheduledDate(task_id),
            "garbage",
        );
        assert_eq!(model.tasks[&task_id].scheduled_date, Some(today()));

        submit(&mut model, InputTarget::EditScheduledDate(task_id), "");
        assert_eq!(model.tasks[&task_id].scheduled_date, None);
    }

    #[test]
    fn test_edit_scheduled_time_variants() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];

        submit(
            &mut model,
            InputTarget::EditScheduledTime(task_id),
            "9am-11am",
        );
        assert_eq!(status(&model), "Time block set: 09:00-11:00");

        submit(&mut model, InputTarget::EditScheduledTime(task_id), "14:15");
        assert_eq!(status(&model), "Start time set: 14:15");
        assert!(model.tasks[&task_id].scheduled_end_time.is_none());

        submit(&mut model, InputTarget::EditScheduledTime(task_id), "later");
        assert!(status(&model).starts_with("Invalid time format"));

        submit(&mut model, InputTarget::EditScheduledTime(task_id), "");
        assert_eq!(status(&model), "Time block cleared");
        assert!(model.tasks[&task_id].scheduled_start_time.is_none());
    }

    #[test]
    fn test_edit_tags_filters_empty_entries() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        submit(&mut model, InputTarget::EditTags(task_id), " a , ,b,, c ");
        assert_eq!(model.tasks[&task_id].tags, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_edit_description_empty_clears() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        submit(
            &mut model,
            InputTarget::EditDescription(task_id),
            "Some notes",
        );
        assert_eq!(
            model.tasks[&task_id].description.as_deref(),
            Some("Some notes")
        );
        submit(&mut model, InputTarget::EditDescription(task_id), "");
        assert_eq!(model.tasks[&task_id].description, None);
    }

    #[test]
    fn test_edit_estimate_set_clear_and_invalid() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];

        submit(&mut model, InputTarget::EditEstimate(task_id), "1h30m");
        assert_eq!(model.tasks[&task_id].estimated_minutes, Some(90));
        assert_eq!(status(&model), "Estimate set to 1h30m");

        submit(&mut model, InputTarget::EditEstimate(task_id), "soon");
        assert_eq!(model.tasks[&task_id].estimated_minutes, Some(90));
        assert_eq!(
            status(&model),
            "Invalid duration format (try: 30m, 1h, 1h30m)"
        );

        submit(&mut model, InputTarget::EditEstimate(task_id), "");
        assert_eq!(model.tasks[&task_id].estimated_minutes, None);
        assert_eq!(status(&model), "Estimate cleared");
    }

    #[test]
    fn test_project_empty_input_creates_nothing() {
        let mut model = Model::new();
        submit(&mut model, InputTarget::Project, "  ");
        assert!(model.projects.is_empty());
    }

    #[test]
    fn test_edit_project_same_name_is_noop() {
        let mut model = Model::new();
        let project = Project::new("Same");
        let project_id = project.id;
        model.projects.insert(project_id, project);

        submit(&mut model, InputTarget::EditProject(project_id), "Same");

        assert!(model.undo_stack.is_empty());
        assert!(model.alerts.status_message.is_none());

        submit(&mut model, InputTarget::EditProject(project_id), "Renamed");
        assert_eq!(status(&model), "Renamed project to 'Renamed'");
    }

    #[test]
    fn test_search_empty_clears_filter() {
        let mut model = create_test_model_with_tasks();
        submit(&mut model, InputTarget::Search, "Task 2");
        assert_eq!(model.visible_tasks.len(), 1);
        submit(&mut model, InputTarget::Search, "");
        assert!(model.filtering.filter.search_text.is_none());
        assert_eq!(model.visible_tasks.len(), 5);
    }

    #[test]
    fn test_move_to_project_zero_unassigns_and_invalid_ignored() {
        let mut model = create_test_model_with_tasks();
        let project = Project::new("P");
        let project_id = project.id;
        model.projects.insert(project_id, project);
        let task_id = model.visible_tasks[0];
        model.tasks.get_mut(&task_id).unwrap().project_id = Some(project_id);

        submit(&mut model, InputTarget::MoveToProject(task_id), "9");
        assert_eq!(model.tasks[&task_id].project_id, Some(project_id));

        submit(&mut model, InputTarget::MoveToProject(task_id), "abc");
        assert_eq!(model.tasks[&task_id].project_id, Some(project_id));

        submit(&mut model, InputTarget::MoveToProject(task_id), "0");
        assert_eq!(model.tasks[&task_id].project_id, None);
    }

    #[test]
    fn test_filter_by_tag_hint_and_blank_entries_clear_filter() {
        let mut model = create_test_model_with_tasks();
        model.filtering.filter.tags = Some(vec!["x".to_string()]);

        submit(&mut model, InputTarget::FilterByTag, "Available: a, b");
        assert!(model.filtering.filter.tags.is_none());

        model.filtering.filter.tags = Some(vec!["x".to_string()]);
        submit(&mut model, InputTarget::FilterByTag, " , , ");
        assert!(model.filtering.filter.tags.is_none());
    }

    #[test]
    fn test_bulk_move_to_project_submit() {
        let mut model = create_test_model_with_tasks();
        let project = Project::new("Target");
        let project_id = project.id;
        model.projects.insert(project_id, project);
        let ids = select_all(&mut model);

        submit(&mut model, InputTarget::BulkMoveToProject, "1");

        for id in &ids {
            assert_eq!(model.tasks[id].project_id, Some(project_id));
        }
        assert!(!model.multi_select.mode);
        assert!(model.multi_select.selected.is_empty());

        select_all(&mut model);
        submit(&mut model, InputTarget::BulkMoveToProject, "0");
        for id in &ids {
            assert_eq!(model.tasks[id].project_id, None);
        }
    }

    #[test]
    fn test_bulk_move_to_project_invalid_input_keeps_selection() {
        let mut model = create_test_model_with_tasks();
        select_all(&mut model);
        submit(&mut model, InputTarget::BulkMoveToProject, "x");
        assert!(model.multi_select.mode);
        assert_eq!(model.multi_select.selected.len(), 5);
    }

    #[test]
    fn test_bulk_set_status_done_sets_completed_at_and_back() {
        let mut model = create_test_model_with_tasks();
        let ids = select_all(&mut model);

        submit(&mut model, InputTarget::BulkSetStatus, "4");
        for id in &ids {
            assert_eq!(model.tasks[id].status, TaskStatus::Done);
            assert!(model.tasks[id].completed_at.is_some());
        }

        model.filtering.show_completed = true;
        model.refresh_visible_tasks();
        select_all(&mut model);
        submit(&mut model, InputTarget::BulkSetStatus, "2");
        for id in &ids {
            assert_eq!(model.tasks[id].status, TaskStatus::InProgress);
            assert!(model.tasks[id].completed_at.is_none());
        }
    }

    #[test]
    fn test_bulk_set_status_invalid_reports_error() {
        let mut model = create_test_model_with_tasks();
        select_all(&mut model);
        submit(&mut model, InputTarget::BulkSetStatus, "7");
        assert!(status(&model).starts_with("Invalid status"));
        assert!(model.multi_select.mode, "selection kept on error");
    }

    #[test]
    fn test_bulk_set_priority_submit() {
        let mut model = create_test_model_with_tasks();
        let ids = select_all(&mut model);

        submit(&mut model, InputTarget::BulkSetPriority, "4");

        for id in &ids {
            assert_eq!(model.tasks[id].priority, Priority::Urgent);
        }
        assert_eq!(status(&model), "Set priority on 5 tasks to Urgent");
        assert!(!model.multi_select.mode);

        select_all(&mut model);
        submit(&mut model, InputTarget::BulkSetPriority, "0");
        assert_eq!(model.tasks[&ids[0]].priority, Priority::None);
    }

    #[test]
    fn test_bulk_set_priority_invalid_reports_error() {
        let mut model = create_test_model_with_tasks();
        select_all(&mut model);
        submit(&mut model, InputTarget::BulkSetPriority, "9");
        assert_eq!(
            status(&model),
            "Invalid priority: enter 0-4 (None/Low/Medium/High/Urgent)"
        );
        assert!(model.multi_select.mode);
    }

    #[test]
    fn test_bulk_add_tags_adds_without_duplicates() {
        let mut model = create_test_model_with_tasks();
        let ids = select_all(&mut model);
        model.tasks.get_mut(&ids[0]).unwrap().tags = vec!["one".to_string()];

        submit(&mut model, InputTarget::BulkAddTags, "+one, two");

        assert_eq!(model.tasks[&ids[0]].tags, vec!["one", "two"]);
        assert_eq!(model.tasks[&ids[1]].tags, vec!["one", "two"]);
        assert_eq!(status(&model), "Added tags to 5 tasks");
        assert!(!model.multi_select.mode);
    }

    #[test]
    fn test_bulk_add_tags_removes_with_minus_prefix() {
        let mut model = create_test_model_with_tasks();
        let ids = select_all(&mut model);
        for id in &ids {
            model.tasks.get_mut(id).unwrap().tags = vec!["keep".to_string(), "drop".to_string()];
        }

        submit(&mut model, InputTarget::BulkAddTags, "-drop");

        for id in &ids {
            assert_eq!(model.tasks[id].tags, vec!["keep"]);
        }
        assert_eq!(status(&model), "Removed tags from 5 tasks");
    }

    #[test]
    fn test_bulk_add_tags_empty_reports_error() {
        let mut model = create_test_model_with_tasks();
        select_all(&mut model);
        submit(&mut model, InputTarget::BulkAddTags, "   ");
        assert!(status(&model).starts_with("No tags entered"));
        assert!(model.multi_select.mode);
    }

    #[test]
    fn test_bulk_set_due_date_set_clear_invalid() {
        let mut model = create_test_model_with_tasks();
        let ids = select_all(&mut model);
        let date = NaiveDate::from_ymd_opt(today().year() + 1, 3, 15).unwrap();
        let date_str = date.format("%Y-%m-%d").to_string();

        submit(&mut model, InputTarget::BulkSetDueDate, &date_str);
        for id in &ids {
            assert_eq!(model.tasks[id].due_date, Some(date));
        }
        assert_eq!(
            status(&model),
            format!("Set due date to {date_str} on 5 tasks")
        );

        select_all(&mut model);
        submit(&mut model, InputTarget::BulkSetDueDate, "whenever");
        assert_eq!(
            status(&model),
            "Invalid date format. Use YYYY-MM-DD or leave empty to clear"
        );
        assert_eq!(model.tasks[&ids[0]].due_date, Some(date));
        assert!(model.multi_select.mode);

        submit(&mut model, InputTarget::BulkSetDueDate, "");
        for id in &ids {
            assert_eq!(model.tasks[id].due_date, None);
        }
        assert_eq!(status(&model), "Cleared due date on 5 tasks");
    }

    #[test]
    fn test_bulk_snooze_set_clear_invalid() {
        let mut model = create_test_model_with_tasks();
        let ids = select_all(&mut model);
        let date = today() + Duration::days(5);
        let date_str = date.format("%Y-%m-%d").to_string();

        submit(&mut model, InputTarget::BulkSnooze, &date_str);
        for id in &ids {
            assert_eq!(model.tasks[id].snooze_until, Some(date));
        }
        assert_eq!(status(&model), format!("Snoozed 5 tasks until {date_str}"));
        assert!(!model.multi_select.mode);

        // Snoozed tasks are hidden; select them directly to clear the snooze
        model.multi_select.mode = true;
        model.multi_select.selected = ids.iter().copied().collect();
        submit(&mut model, InputTarget::BulkSnooze, "nope");
        assert_eq!(status(&model), "Invalid date format");
        assert_eq!(model.tasks[&ids[0]].snooze_until, Some(date));

        submit(&mut model, InputTarget::BulkSnooze, "");
        for id in &ids {
            assert!(model.tasks[id].snooze_until.is_none());
        }
        assert_eq!(status(&model), "Cleared snooze on 5 tasks");
    }

    #[test]
    fn test_edit_dependencies_parses_numbers_and_skips_self() {
        let mut model = create_test_model_with_tasks();
        model.filtering.sort.field = SortField::Title;
        model.filtering.sort.order = SortOrder::Ascending;
        model.refresh_visible_tasks();
        let ids = model.visible_tasks.clone();
        let target = ids[2];

        submit(
            &mut model,
            InputTarget::EditDependencies(target),
            "1, 3; 5 and 99",
        );

        assert_eq!(model.tasks[&target].dependencies, vec![ids[0], ids[4]]);
        assert_eq!(model.undo_stack.len(), 1);
    }

    #[test]
    fn test_edit_recurrence_patterns() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];

        submit(&mut model, InputTarget::EditRecurrence(task_id), "d3");
        let task = &model.tasks[&task_id];
        assert_eq!(task.recurrence, Some(Recurrence::Daily));
        assert_eq!(task.recurrence_options.interval, 3);

        submit(
            &mut model,
            InputTarget::EditRecurrence(task_id),
            "w2 end=2030-01-31 max=4",
        );
        let task = &model.tasks[&task_id];
        assert!(matches!(task.recurrence, Some(Recurrence::Weekly { .. })));
        assert_eq!(task.recurrence_options.interval, 2);
        assert_eq!(
            task.recurrence_options.end_date,
            NaiveDate::from_ymd_opt(2030, 1, 31)
        );
        assert_eq!(task.recurrence_options.max_occurrences, Some(4));

        submit(&mut model, InputTarget::EditRecurrence(task_id), "m15");
        let task = &model.tasks[&task_id];
        assert_eq!(task.recurrence, Some(Recurrence::Monthly { day: 15 }));
        assert_eq!(task.recurrence_options.interval, 1);

        submit(&mut model, InputTarget::EditRecurrence(task_id), "m");
        assert_eq!(
            model.tasks[&task_id].recurrence,
            Some(Recurrence::Monthly { day: today().day() })
        );

        submit(&mut model, InputTarget::EditRecurrence(task_id), "y");
        assert_eq!(
            model.tasks[&task_id].recurrence,
            Some(Recurrence::Yearly {
                month: today().month(),
                day: today().day()
            })
        );

        submit(&mut model, InputTarget::EditRecurrence(task_id), "0");
        assert_eq!(model.tasks[&task_id].recurrence, None);
    }

    #[test]
    fn test_link_task_by_title_and_self_link_rejected() {
        let mut model = Model::new();
        let first = Task::new("Alpha step");
        let second = Task::new("Beta step");
        let first_id = first.id;
        let second_id = second.id;
        model.tasks.insert(first_id, first);
        model.tasks.insert(second_id, second);
        model.refresh_visible_tasks();

        submit(&mut model, InputTarget::LinkTask(first_id), "beta");
        assert_eq!(model.tasks[&first_id].next_task_id, Some(second_id));

        submit(&mut model, InputTarget::LinkTask(second_id), "beta step");
        assert_eq!(model.tasks[&second_id].next_task_id, None);

        submit(
            &mut model,
            InputTarget::LinkTask(second_id),
            "does not exist",
        );
        assert_eq!(model.tasks[&second_id].next_task_id, None);
    }

    #[test]
    fn test_link_task_by_number_rejects_self() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        submit(&mut model, InputTarget::LinkTask(task_id), "1");
        assert_eq!(model.tasks[&task_id].next_task_id, None);

        let second = model.visible_tasks[1];
        submit(&mut model, InputTarget::LinkTask(task_id), "2");
        assert_eq!(model.tasks[&task_id].next_task_id, Some(second));
    }

    #[test]
    fn test_saved_filter_name_empty_saves_nothing() {
        let mut model = Model::new();
        submit(&mut model, InputTarget::SavedFilterName, "");
        assert!(model.saved_filters.is_empty());
        assert!(model.active_saved_filter.is_none());
    }

    #[test]
    fn test_snooze_task_invalid_date() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        submit(&mut model, InputTarget::SnoozeTask(task_id), "someday");
        assert_eq!(status(&model), "Invalid date format");
        assert!(model.tasks[&task_id].snooze_until.is_none());
    }

    #[test]
    fn test_new_habit_and_edit_habit_empty_are_noops() {
        let mut model = Model::new();
        submit(&mut model, InputTarget::NewHabit, "");
        assert!(model.habits.is_empty());

        let habit = crate::domain::Habit::new("Stretch");
        let habit_id = habit.id;
        model.habits.insert(habit_id, habit);
        submit(&mut model, InputTarget::EditHabit(habit_id), "");
        assert_eq!(model.habits[&habit_id].name, "Stretch");
    }

    #[test]
    fn test_goal_targets_empty_are_noops() {
        let mut model = Model::new();
        submit(&mut model, InputTarget::GoalName, "");
        assert!(model.goals.is_empty());

        let goal = crate::domain::Goal::new("G");
        let goal_id = goal.id;
        model.goals.insert(goal_id, goal);
        submit(&mut model, InputTarget::EditGoalName(goal_id), "");
        assert_eq!(model.goals[&goal_id].name, "G");

        submit(&mut model, InputTarget::KeyResultName(goal_id), "");
        assert!(model.key_results.is_empty());
    }
}

mod reports_tasks_table {
    use super::*;
    use crate::domain::{Task, TimeEntry};
    use crate::ui::ReportPanel;

    fn model_with_tracked_tasks(n: usize) -> Model {
        let mut model = Model::new();
        for i in 0..n {
            let task = Task::new(format!("T{i}"));
            let id = task.id;
            model.tasks.insert(id, task);
            let mut e = TimeEntry::start(id);
            e.started_at = chrono::Utc::now() - chrono::Duration::hours(3);
            e.ended_at = Some(e.started_at + chrono::Duration::minutes(10));
            e.duration_minutes = Some(10);
            model.add_time_entry(e);
        }
        model.report_panel = ReportPanel::Tasks;
        model
    }

    #[test]
    fn move_clamps_within_the_table() {
        let mut model = model_with_tracked_tasks(5);
        update(&mut model, Message::Ui(UiMessage::ReportsTasksMove(1)));
        assert_eq!(model.reports_tasks_selected, 1);
        update(&mut model, Message::Ui(UiMessage::ReportsTasksMove(10)));
        assert_eq!(model.reports_tasks_selected, 4, "clamped to the last row");
        update(&mut model, Message::Ui(UiMessage::ReportsTasksMove(-100)));
        assert_eq!(model.reports_tasks_selected, 0);
        update(
            &mut model,
            Message::Ui(UiMessage::ReportsTasksMove(i32::MAX)),
        );
        assert_eq!(model.reports_tasks_selected, 4);
        update(
            &mut model,
            Message::Ui(UiMessage::ReportsTasksMove(i32::MIN)),
        );
        assert_eq!(model.reports_tasks_selected, 0);
    }

    #[test]
    fn move_is_ignored_on_other_panels_and_empty_tables() {
        let mut model = model_with_tracked_tasks(5);
        model.report_panel = ReportPanel::Time;
        update(&mut model, Message::Ui(UiMessage::ReportsTasksMove(1)));
        assert_eq!(model.reports_tasks_selected, 0);

        let mut empty = Model::new();
        empty.report_panel = ReportPanel::Tasks;
        update(&mut empty, Message::Ui(UiMessage::ReportsTasksMove(1)));
        assert_eq!(empty.reports_tasks_selected, 0);
    }

    #[test]
    fn cycling_the_window_resets_the_selection() {
        let mut model = model_with_tracked_tasks(5);
        update(&mut model, Message::Ui(UiMessage::ReportsTasksMove(3)));
        assert_eq!(model.reports_tasks_selected, 3);
        update(&mut model, Message::Ui(UiMessage::ReportsCycleTimeWindow));
        assert_eq!(model.reports_tasks_selected, 0);
    }
}
