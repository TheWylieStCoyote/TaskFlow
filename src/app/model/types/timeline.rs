//! Timeline view state types.

use chrono::{Duration, NaiveDate, Utc};

/// Zoom level for the timeline view.
///
/// Controls how time is displayed on the horizontal axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineZoom {
    /// Each column represents one day
    #[default]
    Day,
    /// Each column represents one week
    Week,
}

/// State for the timeline/Gantt view.
///
/// Tracks the viewport position, selection, and display options.
#[derive(Debug, Clone)]
pub struct TimelineState {
    /// Leftmost visible date in the viewport
    pub viewport_start: NaiveDate,
    /// Number of days visible in the viewport
    pub viewport_days: u32,
    /// Index of the selected task in the timeline list
    pub selected_task_index: usize,
    /// Whether to show dependency lines between tasks
    pub show_dependencies: bool,
    /// Current zoom level
    pub zoom_level: TimelineZoom,
    /// Vertical scroll offset for task list
    pub task_scroll_offset: usize,
}

impl Default for TimelineState {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            // Start viewport 7 days before today
            viewport_start: today - Duration::days(7),
            viewport_days: 21,
            selected_task_index: 0,
            show_dependencies: false,
            zoom_level: TimelineZoom::default(),
            task_scroll_offset: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_zoom_default() {
        let zoom = TimelineZoom::default();
        assert_eq!(zoom, TimelineZoom::Day);
    }

    #[test]
    fn test_timeline_state_default() {
        let state = TimelineState::default();
        assert_eq!(state.viewport_days, 21);
        assert_eq!(state.selected_task_index, 0);
        assert!(!state.show_dependencies);
        assert_eq!(state.zoom_level, TimelineZoom::Day);
    }
}
