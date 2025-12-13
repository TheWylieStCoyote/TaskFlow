//! Timeline view navigation handlers.

use crate::app::{Model, NavigationMessage, TimelineZoom, ViewId};

/// Handle timeline-specific navigation messages.
pub fn handle_timeline_navigation(model: &mut Model, msg: NavigationMessage) {
    if model.current_view != ViewId::Timeline {
        return;
    }

    match msg {
        NavigationMessage::TimelineScrollLeft => {
            model.timeline_state.viewport_start -= chrono::Duration::days(7);
        }
        NavigationMessage::TimelineScrollRight => {
            model.timeline_state.viewport_start += chrono::Duration::days(7);
        }
        NavigationMessage::TimelineZoomIn => {
            if model.timeline_state.zoom_level == TimelineZoom::Week {
                model.timeline_state.zoom_level = TimelineZoom::Day;
            }
        }
        NavigationMessage::TimelineZoomOut => {
            if model.timeline_state.zoom_level == TimelineZoom::Day {
                model.timeline_state.zoom_level = TimelineZoom::Week;
            }
        }
        NavigationMessage::TimelineGoToday => {
            let today = chrono::Utc::now().date_naive();
            model.timeline_state.viewport_start = today - chrono::Duration::days(7);
        }
        NavigationMessage::TimelineUp => {
            if model.timeline_state.selected_task_index > 0 {
                model.timeline_state.selected_task_index -= 1;
                // Adjust scroll offset to keep selection visible
                let idx = model.timeline_state.selected_task_index;
                if idx < model.timeline_state.task_scroll_offset {
                    model.timeline_state.task_scroll_offset = idx;
                }
            }
        }
        NavigationMessage::TimelineDown => {
            let max_index = model.visible_tasks.len().saturating_sub(1);
            if model.timeline_state.selected_task_index < max_index {
                model.timeline_state.selected_task_index += 1;
                // Adjust scroll offset to keep selection visible (estimate viewport ~15 rows)
                let idx = model.timeline_state.selected_task_index;
                let scroll = model.timeline_state.task_scroll_offset;
                const ESTIMATED_VIEWPORT: usize = 15;
                if idx >= scroll + ESTIMATED_VIEWPORT {
                    model.timeline_state.task_scroll_offset =
                        idx.saturating_sub(ESTIMATED_VIEWPORT - 1);
                }
            }
        }
        _ => {}
    }
}
