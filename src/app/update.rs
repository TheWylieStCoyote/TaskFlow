use super::{Message, Model, NavigationMessage, RunningState, SystemMessage, TaskMessage, UiMessage};

/// Main update function - heart of TEA pattern
pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Navigation(msg) => handle_navigation(model, msg),
        Message::Task(msg) => handle_task(model, msg),
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
            if let Some(task) = model.selected_task_mut() {
                task.toggle_complete();
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::SetStatus(task_id, status) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.status = status;
                task.updated_at = chrono::Utc::now();
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::SetPriority(task_id, priority) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.priority = priority;
                task.updated_at = chrono::Utc::now();
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::Create(title) => {
            let task = crate::domain::Task::new(title);
            model.tasks.insert(task.id.clone(), task);
            model.refresh_visible_tasks();
        }
        TaskMessage::Delete(task_id) => {
            model.tasks.remove(&task_id);
            model.refresh_visible_tasks();
        }
        TaskMessage::MoveToProject(task_id, project_id) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.project_id = project_id;
                task.updated_at = chrono::Utc::now();
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
    }
}

fn handle_system(model: &mut Model, msg: SystemMessage) {
    match msg {
        SystemMessage::Quit => {
            model.running = RunningState::Quitting;
        }
        SystemMessage::Resize { width, height } => {
            model.terminal_size = (width, height);
        }
        SystemMessage::Tick => {
            // Handle periodic updates (e.g., timer display)
        }
    }
}
