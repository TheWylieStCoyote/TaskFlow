use crate::ui::InputMode;

use super::{Message, Model, NavigationMessage, RunningState, SystemMessage, TaskMessage, TimeMessage, UiMessage};

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
        NavigationMessage::Up => {
            if model.selected_index > 0 {
                model.selected_index -= 1;
            }
        }
        NavigationMessage::Down => {
            if model.selected_index < model.visible_tasks.len().saturating_sub(1) {
                model.selected_index += 1;
            }
        }
        NavigationMessage::First => {
            model.selected_index = 0;
        }
        NavigationMessage::Last => {
            if !model.visible_tasks.is_empty() {
                model.selected_index = model.visible_tasks.len() - 1;
            }
        }
        NavigationMessage::PageUp => {
            model.selected_index = model.selected_index.saturating_sub(10);
        }
        NavigationMessage::PageDown => {
            let max_index = model.visible_tasks.len().saturating_sub(1);
            model.selected_index = (model.selected_index + 10).min(max_index);
        }
        NavigationMessage::Select(index) => {
            if index < model.visible_tasks.len() {
                model.selected_index = index;
            }
        }
        NavigationMessage::GoToView(view_id) => {
            model.current_view = view_id;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
    }
}

fn handle_task(model: &mut Model, msg: TaskMessage) {
    match msg {
        TaskMessage::ToggleComplete => {
            // Get the task id first to avoid borrow issues
            let task_id = model
                .visible_tasks
                .get(model.selected_index)
                .cloned();

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
        TaskMessage::Create(title) => {
            let task = crate::domain::Task::new(title);
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
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::CancelInput => {
            model.input_mode = InputMode::Normal;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::SubmitInput => {
            if !model.input_buffer.trim().is_empty() {
                let title = model.input_buffer.clone();
                let task = crate::domain::Task::new(title);
                model.sync_task(&task);
                model.tasks.insert(task.id.clone(), task);
                model.refresh_visible_tasks();
            }
            model.input_mode = InputMode::Normal;
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

        update(&mut model, Message::Navigation(NavigationMessage::Select(3)));

        assert_eq!(model.selected_index, 3);
    }

    #[test]
    fn test_navigation_select_invalid_ignored() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 2;

        update(&mut model, Message::Navigation(NavigationMessage::Select(100)));

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
            Message::Task(TaskMessage::SetStatus(task_id.clone(), TaskStatus::InProgress)),
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

        update(&mut model, Message::Task(TaskMessage::Delete(task_id.clone())));

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
}
