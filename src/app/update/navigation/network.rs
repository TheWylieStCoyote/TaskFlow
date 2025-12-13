//! Network view navigation handlers.

use crate::app::{Model, NavigationMessage, ViewId};

/// Handle network-specific navigation messages.
pub fn handle_network_navigation(model: &mut Model, msg: NavigationMessage) {
    if model.current_view != ViewId::Network {
        return;
    }

    match msg {
        NavigationMessage::NetworkUp => {
            if model.view_selection.network_task_index > 0 {
                model.view_selection.network_task_index -= 1;
            }
        }
        NavigationMessage::NetworkDown => {
            let network_tasks = model.network_tasks();
            if model.view_selection.network_task_index + 1 < network_tasks.len() {
                model.view_selection.network_task_index += 1;
            }
        }
        _ => {}
    }
}
