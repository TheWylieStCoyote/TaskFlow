//! Timeline view navigation handlers.
//!
//! Only the selection moves here; the view scrolls the task rows so the
//! selection stays visible for whatever height the terminal has.

use crate::app::{Model, NavigationMessage, TimelineZoom, ViewId};
use crate::domain::today;

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
            let today = today();
            model.timeline_state.viewport_start = today - chrono::Duration::days(7);
        }
        NavigationMessage::TimelineUp => {
            if model.timeline_state.selected_task_index > 0 {
                model.timeline_state.selected_task_index -= 1;
            }
        }
        NavigationMessage::TimelineDown => {
            let max_index = model.visible_tasks.len().saturating_sub(1);
            if model.timeline_state.selected_task_index < max_index {
                model.timeline_state.selected_task_index += 1;
            }
        }
        _ => {}
    }
}
