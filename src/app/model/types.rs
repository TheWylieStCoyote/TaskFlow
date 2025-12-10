//! Core types for the application model.

use chrono::{Datelike, Duration, NaiveDate, Utc};

/// State for the calendar view.
///
/// Tracks the currently displayed month and selected day.
///
/// # Examples
///
/// ```
/// use taskflow::app::CalendarState;
///
/// // Default is current month with today selected
/// let state = CalendarState::default();
/// assert!(state.selected_day.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct CalendarState {
    /// The year being displayed
    pub year: i32,
    /// The month being displayed (1-12)
    pub month: u32,
    /// The selected day within the month (if any)
    pub selected_day: Option<u32>,
    /// Whether focus is on the task list (true) or calendar grid (false)
    pub focus_task_list: bool,
}

impl Default for CalendarState {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            year: today.year(),
            month: today.month(),
            selected_day: Some(today.day()),
            focus_task_list: false,
        }
    }
}

impl CalendarState {
    /// Returns the number of days in the current month.
    #[must_use]
    pub fn days_in_month(&self) -> u32 {
        // Get first day of next month, subtract one day
        let (next_year, next_month) = if self.month == 12 {
            (self.year + 1, 1)
        } else {
            (self.year, self.month + 1)
        };
        NaiveDate::from_ymd_opt(next_year, next_month, 1)
            .and_then(|d| d.checked_sub_signed(Duration::days(1)))
            .map_or(31, |d| d.day())
    }

    /// Validates and clamps the selected day to be within the valid range for the month.
    ///
    /// Returns the clamped day value (1 to days_in_month).
    #[must_use]
    pub fn validated_day(&self) -> Option<u32> {
        self.selected_day
            .map(|day| day.clamp(1, self.days_in_month()))
    }

    /// Returns true if the current state represents a valid calendar date.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !(1..=12).contains(&self.month) {
            return false;
        }
        if let Some(day) = self.selected_day {
            if day < 1 || day > self.days_in_month() {
                return false;
            }
        }
        true
    }

    /// Creates a NaiveDate from the current state, if valid.
    #[must_use]
    pub fn to_date(&self) -> Option<NaiveDate> {
        self.selected_day
            .and_then(|day| NaiveDate::from_ymd_opt(self.year, self.month, day))
    }
}

/// Application lifecycle state.
///
/// Indicates whether the application is running or in the process
/// of shutting down.
///
/// # Examples
///
/// ```
/// use taskflow::app::RunningState;
///
/// let state = RunningState::default();
/// assert_eq!(state, RunningState::Running);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunningState {
    /// Application is running normally
    #[default]
    Running,
    /// Application is shutting down
    Quitting,
}

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

/// State for alert dialogs and error messages.
///
/// Groups related fields for managing alert visibility and messages.
#[derive(Debug, Clone, Default)]
pub struct AlertState {
    /// Whether overdue tasks alert is visible (shown at startup)
    pub show_overdue: bool,
    /// Storage load error message (if any)
    pub storage_error: Option<String>,
    /// Whether storage error alert is visible
    pub show_storage_error: bool,
    /// Error message to display in footer (shown in red)
    pub error_message: Option<String>,
}

/// State for view-specific selections.
///
/// Tracks selection state in specialized views (Kanban, Eisenhower, etc.).
#[derive(Debug, Clone, Default)]
pub struct ViewSelectionState {
    /// Selected column in Kanban view (0-3: Todo, InProgress, Blocked, Done)
    pub kanban_column: usize,
    /// Selected task index within the current Kanban column
    pub kanban_task_index: usize,
    /// Selected quadrant in Eisenhower view (0-3: TL, TR, BL, BR)
    pub eisenhower_quadrant: usize,
    /// Selected task index within the current Eisenhower quadrant
    pub eisenhower_task_index: usize,
    /// Selected day in WeeklyPlanner view (0-6: Mon-Sun)
    pub weekly_planner_day: usize,
    /// Selected task index within the current WeeklyPlanner day
    pub weekly_planner_task_index: usize,
    /// Selected task index in Network view
    pub network_task_index: usize,
}

/// State for daily review mode.
#[derive(Debug, Clone, Default)]
pub struct DailyReviewState {
    /// Whether daily review mode is active
    pub visible: bool,
    /// Current phase of the daily review
    pub phase: crate::ui::DailyReviewPhase,
    /// Selected index within current review phase
    pub selected: usize,
}

/// State for weekly review mode.
#[derive(Debug, Clone, Default)]
pub struct WeeklyReviewState {
    /// Whether weekly review mode is active
    pub visible: bool,
    /// Current phase of the weekly review
    pub phase: crate::ui::WeeklyReviewPhase,
    /// Selected index within current review phase
    pub selected: usize,
}

/// State for template picker modal.
#[derive(Debug, Clone, Default)]
pub struct TemplatePickerState {
    /// Whether template picker is visible
    pub visible: bool,
    /// Index of selected template in picker
    pub selected: usize,
}

/// State for saved filter picker modal.
#[derive(Debug, Clone, Default)]
pub struct SavedFilterPickerState {
    /// Whether saved filter picker is visible
    pub visible: bool,
    /// Selected index in saved filter picker
    pub selected: usize,
}

/// State for keybindings editor modal.
#[derive(Debug, Clone, Default)]
pub struct KeybindingsEditorState {
    /// Whether keybindings editor is visible
    pub visible: bool,
    /// Selected keybinding index in editor
    pub selected: usize,
    /// Whether currently capturing a new key
    pub capturing: bool,
}

/// State for time log editor modal.
#[derive(Debug, Clone, Default)]
pub struct TimeLogEditorState {
    /// Whether time log editor is visible
    pub visible: bool,
    /// Selected time entry index in log
    pub selected: usize,
    /// Current mode in time log editor
    pub mode: crate::ui::TimeLogMode,
    /// Text buffer for editing time entries
    pub buffer: String,
}

/// State for multi-line description editor.
#[derive(Debug, Clone)]
pub struct DescriptionEditorState {
    /// Whether description editor is visible
    pub visible: bool,
    /// Text buffer for editing description (multi-line)
    pub buffer: Vec<String>,
    /// Cursor line position in description buffer
    pub cursor_line: usize,
    /// Cursor column position in description buffer
    pub cursor_col: usize,
}

impl Default for DescriptionEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            buffer: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        }
    }
}

/// State for work log editor modal.
#[derive(Debug, Clone)]
pub struct WorkLogEditorState {
    /// Whether work log editor is visible
    pub visible: bool,
    /// Selected work log entry index
    pub selected: usize,
    /// Current mode in work log editor
    pub mode: crate::ui::WorkLogMode,
    /// Text buffer for editing work log entries (multi-line)
    pub buffer: Vec<String>,
    /// Cursor line position in work log buffer
    pub cursor_line: usize,
    /// Cursor column position in work log buffer
    pub cursor_col: usize,
    /// Search query for filtering work log entries
    pub search_query: String,
}

impl Default for WorkLogEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            selected: 0,
            mode: crate::ui::WorkLogMode::default(),
            buffer: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            search_query: String::new(),
        }
    }
}

/// State for habit tracking view.
#[derive(Debug, Clone, Default)]
pub struct HabitViewState {
    /// Index of selected habit in list
    pub selected: usize,
    /// Whether habit analytics popup is visible
    pub show_analytics: bool,
    /// Whether to show archived habits
    pub show_archived: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_state_days_in_month() {
        // Test various months
        let state = CalendarState {
            year: 2024,
            month: 1, // January
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 31);

        let state = CalendarState {
            year: 2024,
            month: 2, // February (leap year 2024)
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 29);

        let state = CalendarState {
            year: 2023,
            month: 2, // February (non-leap year)
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 28);

        let state = CalendarState {
            year: 2023,
            month: 4, // April
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 30);
    }

    #[test]
    fn test_calendar_state_is_valid() {
        let mut state = CalendarState::default();

        // Valid default state
        assert!(state.is_valid());

        // Invalid month
        state.month = 0;
        assert!(!state.is_valid());

        state.month = 13;
        assert!(!state.is_valid());

        // Invalid day for month
        state.month = 2;
        state.year = 2023;
        state.selected_day = Some(30); // Feb doesn't have 30 days
        assert!(!state.is_valid());

        // Valid day
        state.selected_day = Some(28);
        assert!(state.is_valid());

        // No day selected is valid
        state.selected_day = None;
        assert!(state.is_valid());
    }

    #[test]
    fn test_calendar_state_validated_day() {
        // Day within range
        let state = CalendarState {
            year: 2023,
            month: 2, // February with 28 days
            selected_day: Some(15),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(15));

        // Day too high gets clamped
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: Some(31),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(28));

        // Day too low gets clamped
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: Some(0),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(1));

        // None stays None
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: None,
            ..Default::default()
        };
        assert_eq!(state.validated_day(), None);
    }

    #[test]
    fn test_calendar_state_to_date() {
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: Some(15),
            ..Default::default()
        };

        let date = state.to_date();
        assert!(date.is_some());
        let date = date.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 3);
        assert_eq!(date.day(), 15);

        // Invalid date returns None
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: Some(32),
            ..Default::default()
        };
        assert!(state.to_date().is_none());

        // No day returns None
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: None,
            ..Default::default()
        };
        assert!(state.to_date().is_none());
    }
}
