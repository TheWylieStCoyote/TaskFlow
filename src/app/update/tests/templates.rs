//! Template picker tests.

use crate::app::{update::update, Message, Model, UiMessage};
use crate::ui::InputMode;

#[test]
fn test_show_templates() {
    let mut model = Model::new();
    model.template_picker.visible = false;
    model.template_picker.selected = 5;

    update(&mut model, Message::Ui(UiMessage::ShowTemplates));

    assert!(model.template_picker.visible);
    assert_eq!(model.template_picker.selected, 0);
}

#[test]
fn test_hide_templates() {
    let mut model = Model::new();
    model.template_picker.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideTemplates));

    assert!(!model.template_picker.visible);
}

#[test]
fn test_select_template() {
    let mut model = Model::new();
    // Use existing templates from TemplateManager
    model.template_picker.visible = true;

    // Model already has default templates
    assert!(!model.template_manager.templates.is_empty());

    let initial_task_count = model.tasks.len();

    update(&mut model, Message::Ui(UiMessage::SelectTemplate(0)));

    // Template picker should be hidden
    assert!(!model.template_picker.visible);

    // A new task should be created
    assert_eq!(model.tasks.len(), initial_task_count + 1);

    // Should enter edit mode for the new task
    assert_eq!(model.input.mode, InputMode::Editing);
}

#[test]
fn test_select_template_invalid_index() {
    let mut model = Model::new();
    model.template_picker.visible = true;
    // Clear templates
    model.template_manager.templates.clear();

    let initial_task_count = model.tasks.len();

    update(&mut model, Message::Ui(UiMessage::SelectTemplate(0)));

    // No task should be created
    assert_eq!(model.tasks.len(), initial_task_count);
}
