//! Template picker handlers

use crate::app::{Model, UiMessage, UndoAction};
use crate::ui::{InputMode, InputTarget};

/// Handle template picker messages
pub fn handle_ui_templates(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowTemplates => {
            model.template_picker.visible = true;
            model.template_picker.selected = 0;
        }
        UiMessage::HideTemplates => {
            model.template_picker.visible = false;
        }
        UiMessage::SelectTemplate(index) => {
            if let Some(template) = model.template_manager.get(index) {
                // Create a new task from the template
                let mut task = template.create_task();

                // Apply default priority from settings if template has none
                if task.priority == crate::domain::Priority::None {
                    task.priority = model.default_priority;
                }

                // Push undo action
                model
                    .undo_stack
                    .push(UndoAction::TaskCreated(Box::new(task.clone())));

                // Store the task
                model.sync_task(&task);
                model.tasks.insert(task.id, task.clone());

                // Start editing the task title
                model.input.mode = InputMode::Editing;
                model.input.target = InputTarget::EditTask(task.id);
                model.input.buffer = task.title;
                model.input.cursor = model.input.buffer.len();

                model.template_picker.visible = false;
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}
