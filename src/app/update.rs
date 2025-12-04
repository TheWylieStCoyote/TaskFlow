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
