use crate::ui::{InputMode, InputTarget};

use super::{
    FocusPane, Message, Model, NavigationMessage, RunningState, SystemMessage, TaskMessage,
    TimeMessage, UiMessage, ViewId,
};

/// Main update function - heart of TEA pattern
pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Navigation(msg) => handle_navigation(model, msg),
        Message::Task(msg) => handle_task(model, msg),
        Message::Time(msg) => handle_time(model, msg),
        Message::Ui(msg) => handle_ui(model, msg),
        Message::System(msg) => handle_system(model, msg),
        Message::None => {}
    }
}

fn handle_navigation(model: &mut Model, msg: NavigationMessage) {
    match msg {
        NavigationMessage::Up => match model.focus_pane {
            FocusPane::TaskList => {
                if model.selected_index > 0 {
                    model.selected_index -= 1;
                }
            }
            FocusPane::Sidebar => {
                if model.sidebar_selected > 0 {
                    model.sidebar_selected -= 1;
                    // Skip separator (index 3)
                    if model.sidebar_selected == 3 {
                        model.sidebar_selected = 2;
                    }
                }
            }
        },
        NavigationMessage::Down => match model.focus_pane {
            FocusPane::TaskList => {
                if model.selected_index < model.visible_tasks.len().saturating_sub(1) {
                    model.selected_index += 1;
                }
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                if model.sidebar_selected < max_index {
                    model.sidebar_selected += 1;
                    // Skip separator (index 3)
                    if model.sidebar_selected == 3 {
                        model.sidebar_selected = 4;
                    }
                }
            }
        },
        NavigationMessage::First => match model.focus_pane {
            FocusPane::TaskList => model.selected_index = 0,
            FocusPane::Sidebar => model.sidebar_selected = 0,
        },
        NavigationMessage::Last => match model.focus_pane {
            FocusPane::TaskList => {
                if !model.visible_tasks.is_empty() {
                    model.selected_index = model.visible_tasks.len() - 1;
                }
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_item_count().saturating_sub(1);
            }
        },
        NavigationMessage::PageUp => match model.focus_pane {
            FocusPane::TaskList => {
                model.selected_index = model.selected_index.saturating_sub(10);
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_selected.saturating_sub(5);
            }
        },
        NavigationMessage::PageDown => match model.focus_pane {
            FocusPane::TaskList => {
                let max_index = model.visible_tasks.len().saturating_sub(1);
                model.selected_index = (model.selected_index + 10).min(max_index);
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                model.sidebar_selected = (model.sidebar_selected + 5).min(max_index);
            }
        },
        NavigationMessage::Select(index) => {
            if index < model.visible_tasks.len() {
                model.selected_index = index;
            }
        }
        NavigationMessage::GoToView(view_id) => {
            model.current_view = view_id;
            model.selected_index = 0;
            model.selected_project = None;
            model.refresh_visible_tasks();
        }
        NavigationMessage::FocusSidebar => {
            if model.show_sidebar {
                model.focus_pane = FocusPane::Sidebar;
            }
        }
        NavigationMessage::FocusTaskList => {
            model.focus_pane = FocusPane::TaskList;
        }
        NavigationMessage::SelectSidebarItem => {
            handle_sidebar_selection(model);
        }
    }
}

fn handle_sidebar_selection(model: &mut Model) {
    let selected = model.sidebar_selected;

    // Sidebar layout:
    // 0: All Tasks (TaskList view)
    // 1: Today
    // 2: Upcoming
    // 3: Separator (skip)
    // 4: "Projects" header (skip or go to Projects view)
    // 5+: Individual projects

    match selected {
        0 => {
            model.current_view = ViewId::TaskList;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        1 => {
            model.current_view = ViewId::Today;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        2 => {
            model.current_view = ViewId::Upcoming;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        3 => {} // Separator, do nothing
        4 => {
            // Projects header - go to Projects view showing all project tasks
            model.current_view = ViewId::Projects;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        n if n >= 5 => {
            // Select a specific project
            let project_index = n - 5;
            let project_ids: Vec<_> = model.projects.keys().cloned().collect();
            if let Some(project_id) = project_ids.get(project_index) {
                model.current_view = ViewId::TaskList;
                model.selected_project = Some(project_id.clone());
                model.focus_pane = FocusPane::TaskList;
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}

fn handle_task(model: &mut Model, msg: TaskMessage) {
    match msg {
        TaskMessage::ToggleComplete => {
            // Get the task id first to avoid borrow issues
            let task_id = model.visible_tasks.get(model.selected_index).cloned();

            if let Some(id) = task_id {
                if let Some(task) = model.tasks.get_mut(&id) {
                    task.toggle_complete();
                    let task_clone = task.clone();
                    model.sync_task(&task_clone);
                }
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::SetStatus(task_id, status) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.status = status;
                task.updated_at = chrono::Utc::now();
                let task_clone = task.clone();
                model.sync_task(&task_clone);
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::SetPriority(task_id, priority) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.priority = priority;
                task.updated_at = chrono::Utc::now();
                let task_clone = task.clone();
                model.sync_task(&task_clone);
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::CyclePriority => {
            use crate::domain::Priority;
            let task_id = model.visible_tasks.get(model.selected_index).cloned();

            if let Some(id) = task_id {
                if let Some(task) = model.tasks.get_mut(&id) {
                    task.priority = match task.priority {
                        Priority::None => Priority::Low,
                        Priority::Low => Priority::Medium,
                        Priority::Medium => Priority::High,
                        Priority::High => Priority::Urgent,
                        Priority::Urgent => Priority::None,
                    };
                    task.updated_at = chrono::Utc::now();
                    let task_clone = task.clone();
                    model.sync_task(&task_clone);
                }
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::Create(title) => {
            let task = crate::domain::Task::new(title).with_priority(model.default_priority);
            model.sync_task(&task);
            model.tasks.insert(task.id.clone(), task);
            model.refresh_visible_tasks();
        }
        TaskMessage::Delete(task_id) => {
            model.delete_task_from_storage(&task_id);
            model.tasks.remove(&task_id);
            model.refresh_visible_tasks();
        }
        TaskMessage::MoveToProject(task_id, project_id) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.project_id = project_id;
                task.updated_at = chrono::Utc::now();
                let task_clone = task.clone();
                model.sync_task(&task_clone);
            }
        }
    }
}

fn handle_ui(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ToggleShowCompleted => {
            model.show_completed = !model.show_completed;
            model.refresh_visible_tasks();
        }
        UiMessage::ToggleSidebar => {
            model.show_sidebar = !model.show_sidebar;
        }
        UiMessage::ShowHelp => {
            model.show_help = true;
        }
        UiMessage::HideHelp => {
            model.show_help = false;
        }
        // Input mode handling
        UiMessage::StartCreateTask => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::Task;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::StartCreateProject => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::Project;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::StartEditTask => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditTask(task_id);
                    model.input_buffer = task.title.clone();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditDueDate => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditDueDate(task_id);
                    // Pre-fill with existing due date or empty
                    model.input_buffer = task
                        .due_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::CancelInput => {
            model.input_mode = InputMode::Normal;
            model.input_target = InputTarget::default();
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::SubmitInput => {
            let input = model.input_buffer.trim().to_string();
            match &model.input_target {
                InputTarget::Task => {
                    if !input.is_empty() {
                        let task =
                            crate::domain::Task::new(input).with_priority(model.default_priority);
                        model.sync_task(&task);
                        model.tasks.insert(task.id.clone(), task);
                        model.refresh_visible_tasks();
                    }
                }
                InputTarget::EditTask(task_id) => {
                    if !input.is_empty() {
                        if let Some(task) = model.tasks.get_mut(task_id) {
                            task.title = input;
                            task.updated_at = chrono::Utc::now();
                            let task_clone = task.clone();
                            model.sync_task(&task_clone);
                        }
                        model.refresh_visible_tasks();
                    }
                }
                InputTarget::EditDueDate(task_id) => {
                    use chrono::NaiveDate;
                    if let Some(task) = model.tasks.get_mut(task_id) {
                        // Empty input clears the due date
                        if input.is_empty() {
                            task.due_date = None;
                        } else if let Ok(date) = NaiveDate::parse_from_str(&input, "%Y-%m-%d") {
                            task.due_date = Some(date);
                        }
                        // If parsing fails, just ignore (keep old date)
                        task.updated_at = chrono::Utc::now();
                        let task_clone = task.clone();
                        model.sync_task(&task_clone);
                    }
                    model.refresh_visible_tasks();
                }
                InputTarget::Project => {
                    if !input.is_empty() {
                        let project = crate::domain::Project::new(input);
                        model.sync_project(&project);
                        model.projects.insert(project.id.clone(), project);
                    }
                }
            }
            model.input_mode = InputMode::Normal;
            model.input_target = InputTarget::default();
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::InputChar(c) => {
            model.input_buffer.insert(model.cursor_position, c);
            model.cursor_position += 1;
        }
        UiMessage::InputBackspace => {
            if model.cursor_position > 0 {
                model.cursor_position -= 1;
                model.input_buffer.remove(model.cursor_position);
            }
        }
        UiMessage::InputDelete => {
            if model.cursor_position < model.input_buffer.len() {
                model.input_buffer.remove(model.cursor_position);
            }
        }
        UiMessage::InputCursorLeft => {
            model.cursor_position = model.cursor_position.saturating_sub(1);
        }
        UiMessage::InputCursorRight => {
            if model.cursor_position < model.input_buffer.len() {
                model.cursor_position += 1;
            }
        }
        UiMessage::InputCursorStart => {
            model.cursor_position = 0;
        }
        UiMessage::InputCursorEnd => {
            model.cursor_position = model.input_buffer.len();
        }
        // Delete confirmation
        UiMessage::ShowDeleteConfirm => {
            if model.selected_task().is_some() {
                model.show_confirm_delete = true;
            }
        }
        UiMessage::ConfirmDelete => {
            if let Some(id) = model.visible_tasks.get(model.selected_index).cloned() {
                model.delete_task_from_storage(&id);
                model.tasks.remove(&id);
                model.refresh_visible_tasks();
            }
            model.show_confirm_delete = false;
        }
        UiMessage::CancelDelete => {
            model.show_confirm_delete = false;
        }
    }
}

fn handle_time(model: &mut Model, msg: TimeMessage) {
    match msg {
        TimeMessage::StartTracking => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                model.start_time_tracking(task_id);
            }
        }
        TimeMessage::StopTracking => {
            model.stop_time_tracking();
        }
        TimeMessage::ToggleTracking => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if model.is_tracking_task(&task_id) {
                    model.stop_time_tracking();
                } else {
                    model.start_time_tracking(task_id);
                }
            }
        }
    }
}

fn handle_system(model: &mut Model, msg: SystemMessage) {
    match msg {
        SystemMessage::Quit => {
            // Stop any running timer before quitting
            model.stop_time_tracking();
            model.running = RunningState::Quitting;
        }
        SystemMessage::Save => {
            let _ = model.save();
        }
        SystemMessage::Resize { width, height } => {
            model.terminal_size = (width, height);
        }
        SystemMessage::Tick => {
            // Handle periodic updates (e.g., timer display)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, Task, TaskStatus};

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
        model.current_view = super::super::ViewId::TaskList;

        update(
            &mut model,
            Message::Navigation(NavigationMessage::GoToView(super::super::ViewId::Today)),
        );

        assert_eq!(model.current_view, super::super::ViewId::Today);
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
        assert!(model.tasks.get(&task_id).is_none());
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
        model.sidebar_selected = 2; // Upcoming (before separator at 3)

        // Move down should skip separator (3) and go to Projects header (4)
        update(&mut model, Message::Navigation(NavigationMessage::Down));
        assert_eq!(model.sidebar_selected, 4);

        // Move up should skip separator and go back to Upcoming (2)
        update(&mut model, Message::Navigation(NavigationMessage::Up));
        assert_eq!(model.sidebar_selected, 2);
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
        model.sidebar_selected = 5; // First project (index 5 = after header items)

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
}
