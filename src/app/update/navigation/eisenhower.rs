//! Eisenhower matrix view navigation handlers.
//!
//! Only the selection moves here; the view scrolls the selected quadrant so
//! the selection stays visible for whatever height the terminal has.

use crate::app::{Model, NavigationMessage, ViewId};

/// Handle eisenhower-specific navigation messages.
pub fn handle_eisenhower_navigation(model: &mut Model, msg: NavigationMessage) {
    if model.current_view != ViewId::Eisenhower {
        return;
    }

    match msg {
        NavigationMessage::EisenhowerUp => {
            // First try to navigate tasks within the quadrant
            if model.view_selection.eisenhower_task_index > 0 {
                model.view_selection.eisenhower_task_index -= 1;
            } else if model.view_selection.eisenhower_quadrant >= 2 {
                // At top of task list, move to upper quadrant
                model.view_selection.eisenhower_quadrant -= 2;
                model.view_selection.eisenhower_task_index = 0;
            }
        }
        NavigationMessage::EisenhowerDown => {
            let quadrant_tasks =
                model.eisenhower_quadrant_tasks(model.view_selection.eisenhower_quadrant);
            // First try to navigate tasks within the quadrant
            if model.view_selection.eisenhower_task_index + 1 < quadrant_tasks.len() {
                model.view_selection.eisenhower_task_index += 1;
            } else if model.view_selection.eisenhower_quadrant < 2 {
                // At bottom of task list, move to lower quadrant
                model.view_selection.eisenhower_quadrant += 2;
                model.view_selection.eisenhower_task_index = 0;
            }
        }
        NavigationMessage::EisenhowerLeft => {
            if model.view_selection.eisenhower_quadrant % 2 == 1 {
                model.view_selection.eisenhower_quadrant -= 1;
                model.view_selection.eisenhower_task_index = 0; // Reset task selection
            }
        }
        NavigationMessage::EisenhowerRight => {
            if model.view_selection.eisenhower_quadrant.is_multiple_of(2) {
                model.view_selection.eisenhower_quadrant += 1;
                model.view_selection.eisenhower_task_index = 0; // Reset task selection
            }
        }
        NavigationMessage::EisenhowerSelectQuadrant(quadrant) if quadrant < 4 => {
            model.view_selection.eisenhower_quadrant = quadrant;
            model.view_selection.eisenhower_task_index = 0; // Reset task selection
            model.selected_index = 0;
        }
        _ => {}
    }
}
