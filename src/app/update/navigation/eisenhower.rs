//! Eisenhower matrix view navigation handlers.

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
                // Adjust scroll offset to keep selection visible
                let quad = model.view_selection.eisenhower_quadrant;
                let idx = model.view_selection.eisenhower_task_index;
                if idx < model.view_selection.eisenhower_scroll_offsets[quad] {
                    model.view_selection.eisenhower_scroll_offsets[quad] = idx;
                }
            } else if model.view_selection.eisenhower_quadrant >= 2 {
                // At top of task list, move to upper quadrant
                model.view_selection.eisenhower_quadrant -= 2;
                model.view_selection.eisenhower_task_index = 0;
                // Reset scroll offset for new quadrant
                model.view_selection.eisenhower_scroll_offsets
                    [model.view_selection.eisenhower_quadrant] = 0;
            }
        }
        NavigationMessage::EisenhowerDown => {
            let quadrant_tasks =
                model.eisenhower_quadrant_tasks(model.view_selection.eisenhower_quadrant);
            // First try to navigate tasks within the quadrant
            if model.view_selection.eisenhower_task_index + 1 < quadrant_tasks.len() {
                model.view_selection.eisenhower_task_index += 1;
                // Adjust scroll offset to keep selection visible (estimate viewport ~8 rows)
                let quad = model.view_selection.eisenhower_quadrant;
                let idx = model.view_selection.eisenhower_task_index;
                let scroll = model.view_selection.eisenhower_scroll_offsets[quad];
                const ESTIMATED_VIEWPORT: usize = 8;
                if idx >= scroll + ESTIMATED_VIEWPORT {
                    model.view_selection.eisenhower_scroll_offsets[quad] =
                        idx.saturating_sub(ESTIMATED_VIEWPORT - 1);
                }
            } else if model.view_selection.eisenhower_quadrant < 2 {
                // At bottom of task list, move to lower quadrant
                model.view_selection.eisenhower_quadrant += 2;
                model.view_selection.eisenhower_task_index = 0;
                // Reset scroll offset for new quadrant
                model.view_selection.eisenhower_scroll_offsets
                    [model.view_selection.eisenhower_quadrant] = 0;
            }
        }
        NavigationMessage::EisenhowerLeft => {
            if model.view_selection.eisenhower_quadrant % 2 == 1 {
                model.view_selection.eisenhower_quadrant -= 1;
                model.view_selection.eisenhower_task_index = 0; // Reset task selection
                                                                // Reset scroll offset for new quadrant
                model.view_selection.eisenhower_scroll_offsets
                    [model.view_selection.eisenhower_quadrant] = 0;
            }
        }
        NavigationMessage::EisenhowerRight => {
            if model.view_selection.eisenhower_quadrant.is_multiple_of(2) {
                model.view_selection.eisenhower_quadrant += 1;
                model.view_selection.eisenhower_task_index = 0; // Reset task selection
                                                                // Reset scroll offset for new quadrant
                model.view_selection.eisenhower_scroll_offsets
                    [model.view_selection.eisenhower_quadrant] = 0;
            }
        }
        NavigationMessage::EisenhowerSelectQuadrant(quadrant) => {
            if quadrant < 4 {
                model.view_selection.eisenhower_quadrant = quadrant;
                model.view_selection.eisenhower_task_index = 0; // Reset task selection
                model.view_selection.eisenhower_scroll_offsets[quadrant] = 0; // Reset scroll
                model.selected_index = 0;
            }
        }
        _ => {}
    }
}
