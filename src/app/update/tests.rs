use crate::app::{
    update::update, FocusPane, Message, Model, NavigationMessage, PomodoroMessage, RunningState,
    SystemMessage, TaskMessage, TimeMessage, UiMessage, ViewId, SIDEBAR_FIRST_PROJECT_INDEX,
    SIDEBAR_PROJECTS_HEADER_INDEX, SIDEBAR_SEPARATOR_INDEX,
};
use crate::domain::{Priority, Task, TaskStatus, TimeEntry};
use crate::ui::{InputMode, InputTarget};

fn create_test_model_with_tasks() -> Model {
    let mut model = Model::new();

    for i in 0..5 {
        let task = Task::new(format!("Task {}", i));
        model.tasks.insert(task.id.clone(), task);
    }
    model.refresh_visible_tasks();
    model
}

// Navigation tests
#[test]
fn test_navigation_up() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.selected_index, 1);
}

#[test]
fn test_navigation_up_stops_at_zero() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.selected_index, 0);
}

#[test]
fn test_navigation_down() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.selected_index, 3);
}

#[test]
fn test_navigation_down_stops_at_max() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 4;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.selected_index, 4);
}

#[test]
fn test_navigation_first() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 3;

    update(&mut model, Message::Navigation(NavigationMessage::First));

    assert_eq!(model.selected_index, 0);
}

#[test]
fn test_navigation_last() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::Last));

    assert_eq!(model.selected_index, 4);
}

#[test]
fn test_navigation_page_up() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 4;

    update(&mut model, Message::Navigation(NavigationMessage::PageUp));

    assert_eq!(model.selected_index, 0); // saturating_sub from 4 - 10
}

#[test]
fn test_navigation_page_down() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::PageDown));

    assert_eq!(model.selected_index, 4); // capped at max
}

#[test]
fn test_navigation_select() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::Select(3)),
    );

    assert_eq!(model.selected_index, 3);
}

#[test]
fn test_navigation_select_invalid_ignored() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::Select(100)),
    );

    assert_eq!(model.selected_index, 2); // unchanged
}

#[test]
fn test_navigation_go_to_view() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 3;
    model.current_view = ViewId::TaskList;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::GoToView(ViewId::Today)),
    );

    assert_eq!(model.current_view, ViewId::Today);
    assert_eq!(model.selected_index, 0); // reset to 0
}

// Task tests
#[test]
fn test_task_toggle_complete() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Task should be Todo initially
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);
}

#[test]
fn test_task_set_status() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    update(
        &mut model,
        Message::Task(TaskMessage::SetStatus(
            task_id.clone(),
            TaskStatus::InProgress,
        )),
    );

    assert_eq!(
        model.tasks.get(&task_id).unwrap().status,
        TaskStatus::InProgress
    );
}

#[test]
fn test_task_set_priority() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    update(
        &mut model,
        Message::Task(TaskMessage::SetPriority(task_id.clone(), Priority::Urgent)),
    );

    assert_eq!(
        model.tasks.get(&task_id).unwrap().priority,
        Priority::Urgent
    );
}

#[test]
fn test_task_create() {
    let mut model = Model::new();
    assert!(model.tasks.is_empty());

    update(
        &mut model,
        Message::Task(TaskMessage::Create("New task".to_string())),
    );

    assert_eq!(model.tasks.len(), 1);
    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "New task");
}

#[test]
fn test_task_delete() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let initial_count = model.tasks.len();

    update(
        &mut model,
        Message::Task(TaskMessage::Delete(task_id.clone())),
    );

    assert_eq!(model.tasks.len(), initial_count - 1);
    assert!(!model.tasks.contains_key(&task_id));
}

// Time tests
#[test]
fn test_time_toggle_tracking_start() {
    let mut model = create_test_model_with_tasks();
    assert!(model.active_time_entry.is_none());

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_some());
}

#[test]
fn test_time_toggle_tracking_stop() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    model.start_time_tracking(task_id);

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_none());
}

// UI tests
#[test]
fn test_ui_toggle_show_completed() {
    let mut model = Model::new();
    assert!(!model.show_completed);

    update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

    assert!(model.show_completed);

    update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

    assert!(!model.show_completed);
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
    model.input_mode = InputMode::Editing;

    update(&mut model, Message::Ui(UiMessage::InputChar('H')));
    update(&mut model, Message::Ui(UiMessage::InputChar('i')));

    assert_eq!(model.input_buffer, "Hi");
    assert_eq!(model.cursor_position, 2);
}

#[test]
fn test_ui_input_backspace() {
    let mut model = Model::new();
    model.input_mode = InputMode::Editing;
    model.input_buffer = "Hello".to_string();
    model.cursor_position = 5;

    update(&mut model, Message::Ui(UiMessage::InputBackspace));

    assert_eq!(model.input_buffer, "Hell");
    assert_eq!(model.cursor_position, 4);
}

#[test]
fn test_ui_input_cursor_movement() {
    let mut model = Model::new();
    model.input_mode = InputMode::Editing;
    model.input_buffer = "Hello".to_string();
    model.cursor_position = 3;

    update(&mut model, Message::Ui(UiMessage::InputCursorLeft));
    assert_eq!(model.cursor_position, 2);

    update(&mut model, Message::Ui(UiMessage::InputCursorRight));
    assert_eq!(model.cursor_position, 3);

    update(&mut model, Message::Ui(UiMessage::InputCursorStart));
    assert_eq!(model.cursor_position, 0);

    update(&mut model, Message::Ui(UiMessage::InputCursorEnd));
    assert_eq!(model.cursor_position, 5);
}

#[test]
fn test_ui_submit_input_creates_task() {
    let mut model = Model::new();
    model.input_mode = InputMode::Editing;
    model.input_buffer = "New task from input".to_string();

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.input_mode, InputMode::Normal);
    assert!(model.input_buffer.is_empty());
    assert_eq!(model.tasks.len(), 1);
    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "New task from input");
}

#[test]
fn test_ui_submit_input_empty_ignored() {
    let mut model = Model::new();
    model.input_mode = InputMode::Editing;
    model.input_buffer = "   ".to_string(); // whitespace only

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.input_mode, InputMode::Normal);
    assert!(model.tasks.is_empty()); // no task created
}

#[test]
fn test_ui_cancel_input() {
    let mut model = Model::new();
    model.input_mode = InputMode::Editing;
    model.input_buffer = "Some text".to_string();
    model.cursor_position = 5;

    update(&mut model, Message::Ui(UiMessage::CancelInput));

    assert_eq!(model.input_mode, InputMode::Normal);
    assert!(model.input_buffer.is_empty());
    assert_eq!(model.cursor_position, 0);
}

// System tests
#[test]
fn test_system_quit_stops_timer() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    model.start_time_tracking(task_id);

    assert!(model.active_time_entry.is_some());

    update(&mut model, Message::System(SystemMessage::Quit));

    assert!(model.active_time_entry.is_none());
    assert_eq!(model.running, RunningState::Quitting);
}

#[test]
fn test_system_resize() {
    let mut model = Model::new();

    update(
        &mut model,
        Message::System(SystemMessage::Resize {
            width: 120,
            height: 40,
        }),
    );

    assert_eq!(model.terminal_size, (120, 40));
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
fn test_task_create_uses_default_priority() {
    let mut model = Model::new();
    model.default_priority = Priority::High;

    update(
        &mut model,
        Message::Task(TaskMessage::Create("High priority task".to_string())),
    );

    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "High priority task");
    assert_eq!(task.priority, Priority::High);
}

#[test]
fn test_submit_input_uses_default_priority() {
    let mut model = Model::new();
    model.input_mode = InputMode::Editing;
    model.input_buffer = "Task via input".to_string();
    model.default_priority = Priority::Urgent;

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "Task via input");
    assert_eq!(task.priority, Priority::Urgent);
}

// Sidebar navigation tests
#[test]
fn test_focus_sidebar() {
    let mut model = Model::new();
    assert_eq!(model.focus_pane, FocusPane::TaskList);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::FocusSidebar),
    );

    assert_eq!(model.focus_pane, FocusPane::Sidebar);
}

#[test]
fn test_focus_task_list() {
    let mut model = Model::new();
    model.focus_pane = FocusPane::Sidebar;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::FocusTaskList),
    );

    assert_eq!(model.focus_pane, FocusPane::TaskList);
}

#[test]
fn test_sidebar_navigation_up_down() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 0;

    // Move down
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, 1);

    // Move down again
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, 2);

    // Move up
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.sidebar_selected, 1);
}

#[test]
fn test_sidebar_navigation_skips_separator() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    // Position at last view item (just before separator)
    model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX - 1;

    // Move down should skip separator and go to Projects header
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, SIDEBAR_PROJECTS_HEADER_INDEX);

    // Move up should skip separator and go back to last view item
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.sidebar_selected, SIDEBAR_SEPARATOR_INDEX - 1);
}

#[test]
fn test_sidebar_select_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 1; // Today view

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Today);
    assert!(model.selected_project.is_none());
}

#[test]
fn test_sidebar_select_overdue_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 3; // Overdue view

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Overdue);
    assert!(model.selected_project.is_none());
    assert_eq!(model.focus_pane, FocusPane::TaskList);
}

#[test]
fn test_sidebar_select_project() {
    use crate::domain::Project;

    let mut model = Model::new();
    // Add a project
    let project = Project::new("Test Project");
    let project_id = project.id.clone();
    model.projects.insert(project.id.clone(), project);

    // Add a task with this project
    let mut task = Task::new("Task in project");
    task.project_id = Some(project_id.clone());
    model.tasks.insert(task.id.clone(), task);

    // Add a task without project
    let task2 = Task::new("Task without project");
    model.tasks.insert(task2.id.clone(), task2);

    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 2);

    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = SIDEBAR_FIRST_PROJECT_INDEX; // First project

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    // Project should be selected
    assert_eq!(model.selected_project, Some(project_id));
    // Only project tasks should be visible
    assert_eq!(model.visible_tasks.len(), 1);
}

#[test]
fn test_sidebar_select_all_tasks_clears_project_filter() {
    use crate::domain::Project;

    let mut model = Model::new();
    let project = Project::new("Test Project");
    let project_id = project.id.clone();
    model.projects.insert(project.id.clone(), project);
    model.selected_project = Some(project_id);

    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 0; // All Tasks

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert!(model.selected_project.is_none());
    assert_eq!(model.current_view, ViewId::TaskList);
}

// Project creation tests
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

// Task editing tests
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

#[test]
fn test_cycle_priority() {
    use crate::domain::Priority;
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set initial priority to None
    model.tasks.get_mut(&task_id).unwrap().priority = Priority::None;

    // Cycle through priorities
    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::Low);

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(
        model.tasks.get(&task_id).unwrap().priority,
        Priority::Medium
    );

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::High);

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(
        model.tasks.get(&task_id).unwrap().priority,
        Priority::Urgent
    );

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::None);
}

#[test]
fn test_edit_due_date() {
    use chrono::NaiveDate;
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
    use chrono::NaiveDate;
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
    use chrono::NaiveDate;
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

// Tag management tests
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

// Description editing tests
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

// Move to project tests
#[test]
fn test_start_move_to_project() {
    use crate::domain::Project;

    let mut model = create_test_model_with_tasks();
    let _task_id = model.visible_tasks[0].clone();

    // Add some projects
    let project1 = Project::new("Project Alpha");
    let project2 = Project::new("Project Beta");
    model.projects.insert(project1.id.clone(), project1);
    model.projects.insert(project2.id.clone(), project2);

    // Start move to project
    update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::MoveToProject(_)));
    // Input buffer should contain project list
    assert!(model.input_buffer.contains("0: (none)"));
}

#[test]
fn test_move_to_project_assign() {
    use crate::domain::Project;

    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Initially no project
    assert!(model.tasks.get(&task_id).unwrap().project_id.is_none());

    // Add a project
    let project = Project::new("Test Project");
    let project_id = project.id.clone();
    model.projects.insert(project.id.clone(), project);

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
    use crate::domain::Project;

    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Add a project and assign task to it
    let project = Project::new("Test Project");
    let project_id = project.id.clone();
    model.projects.insert(project.id.clone(), project);
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
    use crate::domain::Project;

    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Add a project
    let project = Project::new("Test Project");
    let project_id = project.id.clone();
    model.projects.insert(project.id.clone(), project);

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
    use crate::domain::Project;

    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Add a project
    let project = Project::new("Test Project");
    model.projects.insert(project.id.clone(), project);

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
    use crate::domain::Project;

    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Add one project
    let project = Project::new("Test Project");
    model.projects.insert(project.id.clone(), project);

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

// Tag filter tests
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

    model
        .tasks
        .insert(task_tagged.id.clone(), task_tagged.clone());
    model.tasks.insert(task_untagged.id.clone(), task_untagged);
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

    model.tasks.insert(task_work.id.clone(), task_work.clone());
    model.tasks.insert(task_home.id.clone(), task_home);
    model
        .tasks
        .insert(task_work_only.id.clone(), task_work_only.clone());
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

    model.tasks.insert(task_tagged.id.clone(), task_tagged);
    model.tasks.insert(task_untagged.id.clone(), task_untagged);
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

// Undo tests
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
    let task_id = model.visible_tasks[0].clone();
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
    let task_id = model.visible_tasks[0].clone();

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
    let task_id = model.visible_tasks[0].clone();
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
    let task_id = model.visible_tasks[0].clone();

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
            Message::Task(TaskMessage::Create(format!("Task {}", i))),
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
    use chrono::NaiveDate;
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

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
    let task_id = model.visible_tasks[0].clone();

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

// Redo tests
#[test]
fn test_redo_task_create() {
    let mut model = Model::new();

    // Create a task
    update(
        &mut model,
        Message::Task(TaskMessage::Create("New task".to_string())),
    );
    let task_id = model.visible_tasks[0].clone();
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
    let task_id = model.visible_tasks[0].clone();

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
    let task_id = model.visible_tasks[0].clone();
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
    let project_id = model.projects.keys().next().unwrap().clone();

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
            Message::Task(TaskMessage::Create(format!("Task {}", i))),
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

// Subtask tests
#[test]
fn test_start_create_subtask() {
    let mut model = create_test_model_with_tasks();
    let _parent_id = model.visible_tasks[0].clone();

    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::Subtask(_)));
    assert!(model.input_buffer.is_empty());
}

#[test]
fn test_start_create_subtask_no_selection() {
    let mut model = Model::new();
    // No tasks, so no selection

    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Should remain in normal mode since there's no parent task
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_submit_subtask_creates_with_parent() {
    let mut model = create_test_model_with_tasks();
    let parent_id = model.visible_tasks[0].clone();
    let initial_count = model.tasks.len();

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Type subtask name
    model.input_buffer = "My subtask".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should have one more task
    assert_eq!(model.tasks.len(), initial_count + 1);

    // Find the new subtask
    let subtask = model
        .tasks
        .values()
        .find(|t| t.title == "My subtask")
        .expect("Subtask should exist");

    // Should have parent_task_id set
    assert_eq!(subtask.parent_task_id, Some(parent_id));
}

#[test]
fn test_subtask_inherits_default_priority() {
    let mut model = create_test_model_with_tasks();
    model.default_priority = Priority::High;

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));
    model.input_buffer = "Priority subtask".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let subtask = model
        .tasks
        .values()
        .find(|t| t.title == "Priority subtask")
        .expect("Subtask should exist");

    assert_eq!(subtask.priority, Priority::High);
}

#[test]
fn test_cancel_subtask_creation() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Type something
    model.input_buffer = "Will be cancelled".to_string();

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // No new task should be created
    assert_eq!(model.tasks.len(), initial_count);
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_subtask_empty_name_not_created() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Submit with empty name
    model.input_buffer = "   ".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // No new task should be created
    assert_eq!(model.tasks.len(), initial_count);
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_subtask_undo() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Create subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));
    model.input_buffer = "Subtask to undo".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.tasks.len(), initial_count + 1);

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(model.tasks.len(), initial_count);
    assert!(!model.tasks.values().any(|t| t.title == "Subtask to undo"));
}

// Bulk operation tests
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
    let task_id = model.visible_tasks[0].clone();

    assert!(!model.selected_tasks.contains(&task_id));

    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
    assert!(model.selected_tasks.contains(&task_id));

    update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
    assert!(!model.selected_tasks.contains(&task_id));
}

#[test]
fn test_toggle_task_selection_not_in_multi_mode() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

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
    model.selected_tasks = model.visible_tasks.iter().cloned().collect();

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
    let task1 = model.visible_tasks[0].clone();
    let task2 = model.visible_tasks[1].clone();
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
    model.selected_tasks = model.visible_tasks.iter().cloned().collect();

    // Exit multi-select mode
    update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));

    assert!(!model.multi_select_mode);
    assert!(model.selected_tasks.is_empty());
}

// Recurrence tests
#[test]
fn test_set_recurrence_daily() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing recurrence
    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    assert_eq!(model.input_mode, InputMode::Editing);

    // Set to daily
    model.input_buffer = "d".to_string();
    model.cursor_position = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert!(matches!(
        task.recurrence,
        Some(crate::domain::Recurrence::Daily)
    ));
}

#[test]
fn test_set_recurrence_weekly() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    model.input_buffer = "w".to_string();
    model.cursor_position = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert!(matches!(
        task.recurrence,
        Some(crate::domain::Recurrence::Weekly { .. })
    ));
}

#[test]
fn test_clear_recurrence() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // First set recurrence
    if let Some(task) = model.tasks.get_mut(&task_id) {
        task.recurrence = Some(crate::domain::Recurrence::Daily);
    }

    // Now clear it
    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    model.input_buffer = "0".to_string();
    model.cursor_position = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.recurrence.is_none());
}

#[test]
fn test_completing_recurring_task_creates_next() {
    use chrono::NaiveDate;
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let initial_count = model.tasks.len();

    // Set task as recurring with a due date
    if let Some(task) = model.tasks.get_mut(&task_id) {
        task.recurrence = Some(crate::domain::Recurrence::Daily);
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
    }
    model.refresh_visible_tasks();

    // Complete the task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Should have created a new task
    assert_eq!(model.tasks.len(), initial_count + 1);

    // The new task should have the same title and be recurring
    let new_tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.id != task_id && t.recurrence.is_some())
        .collect();
    assert_eq!(new_tasks.len(), 1);
    let new_task = new_tasks[0];
    assert!(new_task.recurrence.is_some());
    assert!(new_task.due_date.is_some());
}

#[test]
fn test_completing_non_recurring_task_no_new_task() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Complete a non-recurring task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Should NOT create a new task
    assert_eq!(model.tasks.len(), initial_count);
}

#[test]
fn test_recurrence_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set recurrence
    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    model.input_buffer = "d".to_string();
    model.cursor_position = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert!(model.tasks.get(&task_id).unwrap().recurrence.is_some());

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.tasks.get(&task_id).unwrap().recurrence.is_none());
}

// Task chain tests
#[test]
fn test_start_link_task_enters_editing_mode() {
    let mut model = create_test_model_with_tasks();
    assert_eq!(model.input_mode, InputMode::Normal);

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::LinkTask(_)));
}

#[test]
fn test_start_link_task_shows_current_link() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let target_id = model.visible_tasks[1].clone();
    let target_title = model.tasks.get(&target_id).unwrap().title.clone();

    // Set existing link
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id.clone());

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Should show the linked task title
    assert_eq!(
        model.input_buffer,
        format!("Currently linked to: {target_title}")
    );
}

#[test]
fn test_link_task_by_number() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let target_id = model.visible_tasks[2].clone();

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Enter task number "3" (1-indexed)
    model.input_buffer = "3".to_string();
    model.cursor_position = 1;

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should link to the third task
    assert_eq!(
        model.tasks.get(&task_id).unwrap().next_task_id,
        Some(target_id)
    );
}

#[test]
fn test_link_task_by_title_search() {
    let mut model = Model::new();

    // Create tasks with distinct titles
    let task1 = Task::new("First task");
    let task2 = Task::new("Second task");
    let task3 = Task::new("Target unique title");
    let task1_id = task1.id.clone();
    let task3_id = task3.id.clone();

    model.tasks.insert(task1.id.clone(), task1);
    model.tasks.insert(task2.id.clone(), task2);
    model.tasks.insert(task3.id.clone(), task3);
    model.refresh_visible_tasks();

    // Find the visible index for task1
    let task1_visible_idx = model
        .visible_tasks
        .iter()
        .position(|id| *id == task1_id)
        .expect("task1 should be in visible_tasks");
    model.selected_index = task1_visible_idx;

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Enter part of target title
    model.input_buffer = "Target unique".to_string();
    model.cursor_position = model.input_buffer.len();

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should link to the task with matching title
    assert_eq!(
        model.tasks.get(&task1_id).unwrap().next_task_id,
        Some(task3_id)
    );
}

#[test]
fn test_link_task_prevents_self_linking() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    update(&mut model, Message::Ui(UiMessage::StartLinkTask));

    // Try to link task 1 to itself
    model.input_buffer = "1".to_string();
    model.cursor_position = 1;

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should NOT create self-link
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_link_task_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let target_id = model.visible_tasks[1].clone();

    // Link task
    update(&mut model, Message::Ui(UiMessage::StartLinkTask));
    model.input_buffer = "2".to_string();
    model.cursor_position = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(
        model.tasks.get(&task_id).unwrap().next_task_id,
        Some(target_id)
    );

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_unlink_task_removes_link() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let target_id = model.visible_tasks[1].clone();

    // Set existing link
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id);

    update(&mut model, Message::Ui(UiMessage::UnlinkTask));

    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_unlink_task_when_not_linked_is_noop() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Ensure no link exists
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

    update(&mut model, Message::Ui(UiMessage::UnlinkTask));

    // Should still be None, no error
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
}

#[test]
fn test_unlink_task_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let target_id = model.visible_tasks[1].clone();

    // Set existing link
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id.clone());

    // Unlink
    update(&mut model, Message::Ui(UiMessage::UnlinkTask));
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(
        model.tasks.get(&task_id).unwrap().next_task_id,
        Some(target_id)
    );
}

#[test]
fn test_completing_chained_task_schedules_next() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let next_id = model.visible_tasks[1].clone();

    // Link tasks
    model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(next_id.clone());

    // Next task should have no scheduled date initially
    assert!(model.tasks.get(&next_id).unwrap().scheduled_date.is_none());

    // Complete the first task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Next task should now be scheduled for today (local time)
    let today = chrono::Local::now().date_naive();
    assert_eq!(
        model.tasks.get(&next_id).unwrap().scheduled_date,
        Some(today)
    );
}

#[test]
fn test_completing_unchained_task_no_scheduling() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let other_id = model.visible_tasks[1].clone();

    // No link - task is standalone
    assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

    // Other task has no scheduled date
    assert!(model.tasks.get(&other_id).unwrap().scheduled_date.is_none());

    // Complete the first task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Other task should NOT be scheduled
    assert!(model.tasks.get(&other_id).unwrap().scheduled_date.is_none());
}

// === Pomodoro Timer Tests ===

#[test]
fn test_pomodoro_start() {
    let mut model = create_test_model_with_tasks();

    assert!(model.pomodoro_session.is_none());
    assert!(!model.focus_mode);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(model.pomodoro_session.is_some());
    assert!(model.focus_mode);

    let session = model.pomodoro_session.as_ref().unwrap();
    assert_eq!(session.session_goal, 4);
    assert_eq!(session.cycles_completed, 0);
    assert!(!session.paused);
}

#[test]
fn test_pomodoro_pause_resume() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(!model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::Pause));
    assert!(model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::Resume));
    assert!(!model.pomodoro_session.as_ref().unwrap().paused);
}

#[test]
fn test_pomodoro_toggle_pause() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(!model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::TogglePause));
    assert!(model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::TogglePause));
    assert!(!model.pomodoro_session.as_ref().unwrap().paused);
}

#[test]
fn test_pomodoro_stop() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(model.pomodoro_session.is_some());

    update(&mut model, Message::Pomodoro(PomodoroMessage::Stop));
    assert!(model.pomodoro_session.is_none());
}

#[test]
fn test_pomodoro_tick_decrements_time() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    let initial_remaining = model.pomodoro_session.as_ref().unwrap().remaining_secs;

    update(&mut model, Message::Pomodoro(PomodoroMessage::Tick));

    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().remaining_secs,
        initial_remaining - 1
    );
}

#[test]
fn test_pomodoro_tick_paused_no_decrement() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );
    update(&mut model, Message::Pomodoro(PomodoroMessage::Pause));

    let initial_remaining = model.pomodoro_session.as_ref().unwrap().remaining_secs;

    update(&mut model, Message::Pomodoro(PomodoroMessage::Tick));

    // Time should not decrement when paused
    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().remaining_secs,
        initial_remaining
    );
}

#[test]
fn test_pomodoro_skip_phase() {
    use crate::domain::PomodoroPhase;

    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    // Should be in Work phase
    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().phase,
        PomodoroPhase::Work
    );

    // Skip to break
    update(&mut model, Message::Pomodoro(PomodoroMessage::Skip));

    // Should now be in ShortBreak phase and cycle completed
    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().phase,
        PomodoroPhase::ShortBreak
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().cycles_completed, 1);
}

#[test]
fn test_pomodoro_goal_adjustment() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 4);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::IncrementGoal),
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 5);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 4);

    // Cannot go below 1
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 1);
}

#[test]
fn test_pomodoro_config_changes() {
    let mut model = Model::new();

    assert_eq!(model.pomodoro_config.work_duration_mins, 25);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetWorkDuration(30)),
    );
    assert_eq!(model.pomodoro_config.work_duration_mins, 30);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetShortBreak(10)),
    );
    assert_eq!(model.pomodoro_config.short_break_mins, 10);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetLongBreak(20)),
    );
    assert_eq!(model.pomodoro_config.long_break_mins, 20);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetCyclesBeforeLongBreak(3)),
    );
    assert_eq!(model.pomodoro_config.cycles_before_long_break, 3);
}

// ============ Keybindings Editor Tests ============

#[test]
fn test_show_keybindings_editor() {
    let mut model = Model::new();
    assert!(!model.show_keybindings_editor);

    update(&mut model, Message::Ui(UiMessage::ShowKeybindingsEditor));
    assert!(model.show_keybindings_editor);
    assert_eq!(model.keybinding_selected, 0);
    assert!(!model.keybinding_capturing);
}

#[test]
fn test_hide_keybindings_editor() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_capturing = true;

    update(&mut model, Message::Ui(UiMessage::HideKeybindingsEditor));
    assert!(!model.show_keybindings_editor);
    assert!(!model.keybinding_capturing);
}

#[test]
fn test_keybindings_navigation() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_selected = 5;

    update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
    assert_eq!(model.keybinding_selected, 4);

    update(&mut model, Message::Ui(UiMessage::KeybindingsDown));
    assert_eq!(model.keybinding_selected, 5);

    // Navigate up at 0 should stay at 0
    model.keybinding_selected = 0;
    update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
    assert_eq!(model.keybinding_selected, 0);
}

#[test]
fn test_start_edit_keybinding() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;

    update(&mut model, Message::Ui(UiMessage::StartEditKeybinding));
    assert!(model.keybinding_capturing);
    assert!(model.status_message.is_some());
}

#[test]
fn test_cancel_edit_keybinding() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_capturing = true;
    model.status_message = Some("Press a key...".to_string());

    update(&mut model, Message::Ui(UiMessage::CancelEditKeybinding));
    assert!(!model.keybinding_capturing);
    assert!(model.status_message.is_none());
}

#[test]
fn test_apply_keybinding() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_capturing = true;

    // Get the first binding's action
    let bindings = model.keybindings.sorted_bindings();
    let (_, first_action) = &bindings[0];
    let original_action = first_action.clone();

    // Apply a new key to that action
    update(
        &mut model,
        Message::Ui(UiMessage::ApplyKeybinding("z".to_string())),
    );

    assert!(!model.keybinding_capturing);
    // The action should now be bound to 'z'
    assert_eq!(model.keybindings.get_action("z"), Some(&original_action));
}

#[test]
fn test_reset_all_keybindings() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;

    // Modify a keybinding
    model
        .keybindings
        .set_binding("z".to_string(), crate::config::Action::Quit);

    // Verify it was changed
    assert_eq!(
        model.keybindings.get_action("z"),
        Some(&crate::config::Action::Quit)
    );

    // Reset all
    update(&mut model, Message::Ui(UiMessage::ResetAllKeybindings));

    // Should be back to default (z is not a default binding)
    assert_eq!(model.keybindings.get_action("z"), None);
    assert!(model.status_message.is_some());
}

// Calendar focus tests

#[test]
fn test_calendar_focus_toggle() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Calendar;
    model.refresh_visible_tasks();

    // Initially focus should be on calendar grid
    assert!(!model.calendar_state.focus_task_list);

    // Focus task list (should work if there are tasks)
    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarFocusTaskList),
    );

    // Should be focused on task list if there are tasks for the day
    if !model.tasks_for_selected_day().is_empty() {
        assert!(model.calendar_state.focus_task_list);
    }

    // Focus back to grid
    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarFocusGrid),
    );
    assert!(!model.calendar_state.focus_task_list);
}

#[test]
fn test_calendar_task_navigation() {
    use chrono::Datelike;

    let mut model = Model::new();

    // Add multiple tasks for the same day
    let today = chrono::Utc::now().date_naive();
    let task1 = crate::domain::Task::new("Task 1").with_due_date(today);
    let task2 = crate::domain::Task::new("Task 2").with_due_date(today);
    let task3 = crate::domain::Task::new("Task 3").with_due_date(today);

    model.tasks.insert(task1.id.clone(), task1);
    model.tasks.insert(task2.id.clone(), task2);
    model.tasks.insert(task3.id.clone(), task3);

    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(today.day());
    model.calendar_state.year = today.year();
    model.calendar_state.month = today.month();
    model.refresh_visible_tasks();

    // Focus on task list
    model.calendar_state.focus_task_list = true;
    model.selected_index = 0;

    // Navigate down
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.selected_index, 1);

    // Navigate down again
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.selected_index, 2);

    // Navigate down at end should stay at end
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.selected_index, 2);

    // Navigate up
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.selected_index, 1);
}

#[test]
fn test_calendar_focus_reset_on_day_change() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(15);
    model.calendar_state.focus_task_list = true;

    // Select a new day
    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarSelectDay(20)),
    );

    // Focus should be reset to grid
    assert!(!model.calendar_state.focus_task_list);
    assert_eq!(model.calendar_state.selected_day, Some(20));
}

#[test]
fn test_calendar_focus_only_with_tasks() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(15);
    model.refresh_visible_tasks();

    // No tasks for the day, focus should not switch
    assert!(!model.calendar_state.focus_task_list);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::CalendarFocusTaskList),
    );

    // Should still be on grid since there are no tasks
    assert!(!model.calendar_state.focus_task_list);
}

#[test]
fn test_calendar_task_actions_when_focused() {
    use chrono::Datelike;

    let mut model = Model::new();

    // Add a task for today
    let today = chrono::Utc::now().date_naive();
    let task = crate::domain::Task::new("Test task").with_due_date(today);
    let task_id = task.id.clone();
    model.tasks.insert(task_id.clone(), task);

    model.current_view = ViewId::Calendar;
    model.calendar_state.selected_day = Some(today.day());
    model.calendar_state.year = today.year();
    model.calendar_state.month = today.month();
    model.calendar_state.focus_task_list = true;
    model.refresh_visible_tasks();
    model.selected_index = 0;

    // Task should be Todo initially
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

    // Toggle complete
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Task should now be Done
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);
}

// Import tests
#[test]
fn test_start_import_csv_sets_input_mode() {
    let mut model = Model::new();

    update(&mut model, Message::System(SystemMessage::StartImportCsv));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(
        model.input_target,
        InputTarget::ImportFilePath(crate::storage::ImportFormat::Csv)
    ));
    assert!(model.input_buffer.is_empty());
}

#[test]
fn test_start_import_ics_sets_input_mode() {
    let mut model = Model::new();

    update(&mut model, Message::System(SystemMessage::StartImportIcs));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(
        model.input_target,
        InputTarget::ImportFilePath(crate::storage::ImportFormat::Ics)
    ));
}

#[test]
fn test_cancel_import_resets_state() {
    let mut model = Model::new();

    // Set up pending import state
    model.show_import_preview = true;
    model.pending_import = Some(crate::storage::ImportResult {
        imported: vec![],
        skipped: vec![],
        errors: vec![],
    });

    update(&mut model, Message::System(SystemMessage::CancelImport));

    assert!(!model.show_import_preview);
    assert!(model.pending_import.is_none());
    assert!(model.status_message.is_some());
    assert!(model.status_message.as_ref().unwrap().contains("cancelled"));
}

#[test]
fn test_confirm_import_adds_tasks() {
    let mut model = Model::new();

    // Create a task to import
    let task = Task::new("Imported Task");

    model.show_import_preview = true;
    model.pending_import = Some(crate::storage::ImportResult {
        imported: vec![task.clone()],
        skipped: vec![],
        errors: vec![],
    });

    update(&mut model, Message::System(SystemMessage::ConfirmImport));

    assert!(!model.show_import_preview);
    assert!(model.pending_import.is_none());
    assert_eq!(model.tasks.len(), 1);
    assert!(model.tasks.values().any(|t| t.title == "Imported Task"));
    assert!(model.status_message.is_some());
    assert!(model
        .status_message
        .as_ref()
        .unwrap()
        .contains("Imported 1"));
}

#[test]
fn test_confirm_import_multiple_tasks() {
    let mut model = Model::new();

    // Create multiple tasks to import
    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    let task3 = Task::new("Task 3");

    model.show_import_preview = true;
    model.pending_import = Some(crate::storage::ImportResult {
        imported: vec![task1, task2, task3],
        skipped: vec![],
        errors: vec![],
    });

    update(&mut model, Message::System(SystemMessage::ConfirmImport));

    assert_eq!(model.tasks.len(), 3);
    assert!(model
        .status_message
        .as_ref()
        .unwrap()
        .contains("Imported 3"));
}

#[test]
fn test_import_empty_path_shows_error() {
    use crate::ui::InputTarget;

    let mut model = Model::new();

    // Set up for file path input
    model.input_mode = InputMode::Editing;
    model.input_target = InputTarget::ImportFilePath(crate::storage::ImportFormat::Csv);
    model.input_buffer = "   ".to_string(); // Whitespace only

    // Submit the input
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should show error, not crash
    assert!(model.status_message.is_some());
    assert!(model
        .status_message
        .as_ref()
        .unwrap()
        .contains("No file path"));
}

#[test]
fn test_reports_panel_navigation() {
    use crate::ui::ReportPanel;

    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Reports;
    assert_eq!(model.report_panel, ReportPanel::Overview);

    // Navigate to next panel
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Velocity);

    // Navigate to next panel again
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Tags);

    // Navigate back
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsPrevPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Velocity);
}

#[test]
fn test_reports_navigation_only_works_in_reports_view() {
    use crate::ui::ReportPanel;

    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::TaskList; // Not in reports view
    assert_eq!(model.report_panel, ReportPanel::Overview);

    // Try to navigate - should have no effect
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Overview); // Unchanged
}

#[test]
fn test_sidebar_select_reports_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 7; // Reports view index

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Reports);
    assert_eq!(model.focus_pane, FocusPane::TaskList);
}

#[test]
fn test_completing_parent_cascades_to_descendants() {
    use crate::domain::{Task, TaskStatus};

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
    use crate::domain::{Task, TaskStatus};

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
    use crate::domain::{Task, TaskStatus};

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
    use crate::domain::Task;

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
    use crate::domain::Task;

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
    use crate::domain::Task;

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

#[test]
fn test_edit_project_with_undo() {
    let mut model = Model::new();

    // Create a project first
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));
    for c in "Original Name".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::SubmitInput));
    assert_eq!(model.projects.len(), 1);

    let project_id = model.projects.keys().next().unwrap().clone();
    assert_eq!(
        model.projects.get(&project_id).unwrap().name,
        "Original Name"
    );

    // Select the project (simulate sidebar selection)
    model.selected_project = Some(project_id.clone());

    // Start editing the project
    update(&mut model, Message::Ui(UiMessage::StartEditProject));
    assert!(matches!(model.input_target, InputTarget::EditProject(_)));

    // Clear buffer and type new name
    model.input_buffer.clear();
    model.cursor_position = 0;
    for c in "New Name".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Project should be renamed
    assert_eq!(model.projects.get(&project_id).unwrap().name, "New Name");

    // Undo should restore original name
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(
        model.projects.get(&project_id).unwrap().name,
        "Original Name"
    );

    // Redo should apply the rename again
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.projects.get(&project_id).unwrap().name, "New Name");
}

#[test]
fn test_delete_project_with_undo() {
    let mut model = Model::new();

    // Create a project
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));
    for c in "Test Project".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::SubmitInput));
    assert_eq!(model.projects.len(), 1);

    let project_id = model.projects.keys().next().unwrap().clone();

    // Create a task in the project
    update(
        &mut model,
        Message::Task(TaskMessage::Create("Task in project".to_string())),
    );
    let task_id = model.tasks.keys().next().unwrap().clone();

    // Assign task to project
    if let Some(task) = model.tasks.get_mut(&task_id) {
        task.project_id = Some(project_id.clone());
    }

    // Select the project
    model.selected_project = Some(project_id.clone());

    // Delete the project
    update(&mut model, Message::Ui(UiMessage::DeleteProject));

    // Project should be deleted
    assert!(model.projects.is_empty());
    // Task should be unassigned
    assert!(model.tasks.get(&task_id).unwrap().project_id.is_none());
    // Selected project should be cleared
    assert!(model.selected_project.is_none());

    // Undo should restore the project
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.projects.len(), 1);
    assert!(model.projects.contains_key(&project_id));
    assert_eq!(
        model.projects.get(&project_id).unwrap().name,
        "Test Project"
    );

    // Note: task assignment is not restored by project undo (tasks were modified separately)
}

#[test]
fn test_edit_project_requires_selection() {
    let mut model = Model::new();

    // Create a project but don't select it
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));
    for c in "Test".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Try to edit without selection
    model.selected_project = None;
    update(&mut model, Message::Ui(UiMessage::StartEditProject));

    // Should show error message, not enter edit mode
    assert!(model.status_message.is_some());
    assert!(!matches!(model.input_target, InputTarget::EditProject(_)));
}

#[test]
fn test_delete_project_requires_selection() {
    let mut model = Model::new();

    // Create a project but don't select it
    update(&mut model, Message::Ui(UiMessage::StartCreateProject));
    for c in "Test".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::SubmitInput));
    assert_eq!(model.projects.len(), 1);

    // Try to delete without selection
    model.selected_project = None;
    update(&mut model, Message::Ui(UiMessage::DeleteProject));

    // Project should still exist
    assert_eq!(model.projects.len(), 1);
    // Should show error message
    assert!(model.status_message.is_some());
}

// Time tracking undo tests
#[test]
fn test_time_tracking_start_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start time tracking
    update(&mut model, Message::Time(TimeMessage::StartTracking));

    // Verify tracking started
    assert!(model.active_time_entry.is_some());
    assert!(model.is_tracking_task(&task_id));
    let entry_id = model.active_time_entry.clone().unwrap();
    assert!(model.time_entries.contains_key(&entry_id));

    // Undo should stop tracking and remove entry
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.active_time_entry.is_none());
    assert!(!model.time_entries.contains_key(&entry_id));

    // Redo should restore tracking
    update(&mut model, Message::System(SystemMessage::Redo));

    assert!(model.active_time_entry.is_some());
    assert!(model.is_tracking_task(&task_id));
}

#[test]
fn test_time_tracking_stop_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start tracking
    model.start_time_tracking(task_id.clone());
    assert!(model.active_time_entry.is_some());
    let entry_id = model.active_time_entry.clone().unwrap();

    // Stop tracking
    update(&mut model, Message::Time(TimeMessage::StopTracking));

    // Verify stopped
    assert!(model.active_time_entry.is_none());
    let stopped_entry = model.time_entries.get(&entry_id).unwrap();
    assert!(!stopped_entry.is_running());

    // Undo should restore running state
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.active_time_entry.is_some());
    let running_entry = model.time_entries.get(&entry_id).unwrap();
    assert!(running_entry.is_running());
}

#[test]
fn test_time_tracking_toggle_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Toggle to start
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    assert!(model.is_tracking_task(&task_id));

    // Toggle to stop
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    assert!(!model.is_tracking_task(&task_id));

    // Undo stop - should be tracking again
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.is_tracking_task(&task_id));

    // Undo start - should not be tracking
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(!model.is_tracking_task(&task_id));
}

#[test]
fn test_time_tracking_actual_minutes_update() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Check initial actual_minutes is 0
    assert_eq!(model.tasks.get(&task_id).unwrap().actual_minutes, 0);

    // Create a time entry with known duration (simulate past entry)
    let mut entry = TimeEntry::start(task_id.clone());
    entry.stop();
    entry.duration_minutes = Some(30); // 30 minutes
    model.time_entries.insert(entry.id.clone(), entry);

    // Recalculate
    let total = model.total_time_for_task(&task_id);
    assert_eq!(total, 30);
}

#[test]
fn test_time_tracking_persistence_simulation() {
    // Simulate first session: start tracking
    let mut model1 = create_test_model_with_tasks();
    let task_id = model1.visible_tasks[0].clone();

    // Start tracking
    update(&mut model1, Message::Time(TimeMessage::StartTracking));
    assert!(model1.active_time_entry.is_some());

    // Get the running entry
    let running_entry = model1.active_time_entry().unwrap().clone();
    assert!(running_entry.is_running());

    // Simulate "restart" by creating new model and loading entry
    let mut model2 = create_test_model_with_tasks();

    // Load the running entry (simulating storage restore)
    model2
        .time_entries
        .insert(running_entry.id.clone(), running_entry.clone());
    if running_entry.is_running() {
        model2.active_time_entry = Some(running_entry.id.clone());
    }

    // Verify tracking is still active after "restart"
    assert!(model2.active_time_entry.is_some());
    assert!(model2.is_tracking_task(&task_id));

    // The entry should still be running
    let restored_entry = model2.active_time_entry().unwrap();
    assert!(restored_entry.is_running());

    // Duration should calculate from original start time (just verify it works)
    let _duration = restored_entry.calculated_duration_minutes();
}

#[test]
fn test_time_tracking_switch_task() {
    let mut model = create_test_model_with_tasks();
    let task_id_1 = model.visible_tasks[0].clone();
    let task_id_2 = model.visible_tasks[1].clone();

    // Start tracking task 1
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::StartTracking));
    assert!(model.is_tracking_task(&task_id_1));
    let entry1_id = model.active_time_entry.clone().unwrap();

    // Switch to task 2 (should stop task 1 and start task 2)
    model.selected_index = 1;
    update(&mut model, Message::Time(TimeMessage::StartTracking));

    // Task 2 should now be tracked
    assert!(model.is_tracking_task(&task_id_2));
    assert!(!model.is_tracking_task(&task_id_1));

    // First entry should be stopped
    let entry1 = model.time_entries.get(&entry1_id).unwrap();
    assert!(!entry1.is_running());

    // Undo should restore task 1 tracking
    update(&mut model, Message::System(SystemMessage::Undo)); // undo start task 2
    update(&mut model, Message::System(SystemMessage::Undo)); // undo stop task 1

    assert!(model.is_tracking_task(&task_id_1));
}
