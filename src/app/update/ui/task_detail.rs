//! Task detail modal handler.

use crate::app::{Model, UiMessage};

/// Handle task detail modal messages.
pub fn handle_ui_task_detail(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowTaskDetail => {
            if model.selected_task().is_some() {
                model.task_detail.visible = true;
                model.task_detail.scroll = 0;
            }
        }
        UiMessage::HideTaskDetail => {
            model.task_detail.visible = false;
        }
        UiMessage::TaskDetailScrollUp => {
            model.task_detail.scroll = model.task_detail.scroll.saturating_sub(1);
        }
        UiMessage::TaskDetailScrollDown => {
            model.task_detail.scroll = model.task_detail.scroll.saturating_add(1);
        }
        UiMessage::TaskDetailPageUp => {
            model.task_detail.scroll = model.task_detail.scroll.saturating_sub(10);
        }
        UiMessage::TaskDetailPageDown => {
            model.task_detail.scroll = model.task_detail.scroll.saturating_add(10);
        }
        UiMessage::TaskDetailScrollTop => {
            model.task_detail.scroll = 0;
        }
        UiMessage::TaskDetailScrollBottom => {
            // Set to a large value; rendering will clamp it
            model.task_detail.scroll = usize::MAX;
        }
        _ => {}
    }
}
