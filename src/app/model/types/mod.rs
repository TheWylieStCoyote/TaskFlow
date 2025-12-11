//! Core types for the application model.
//!
//! This module is organized into submodules by functionality:
//! - `burndown` - Burndown chart configuration state
//! - `calendar` - Calendar view state
//! - `timeline` - Timeline/Gantt view state
//! - `reviews` - Daily and weekly review states
//! - `ui_state` - Core UI state (input, selections, filters)
//! - `pomodoro` - Pomodoro timer state
//! - `editors` - Multi-line editor states
//! - `pickers` - Modal picker states
//! - `persistence` - Storage and import states

mod burndown;
mod calendar;
mod editors;
mod persistence;
mod pickers;
mod pomodoro;
mod reviews;
mod timeline;
mod ui_state;

// Re-export all types for backwards compatibility
pub use burndown::{BurndownMode, BurndownState, BurndownTimeWindow};
pub use calendar::CalendarState;
pub use editors::{DescriptionEditorState, TimeLogEditorState, WorkLogEditorState};
pub use persistence::{ImportState, StorageState};
pub use pickers::{KeybindingsEditorState, SavedFilterPickerState, TemplatePickerState};
pub use pomodoro::PomodoroState;
pub use reviews::{DailyReviewState, WeeklyReviewState};
pub use timeline::{TimelineState, TimelineZoom};
pub use ui_state::{
    AlertState, DuplicatesViewState, FilterState, GoalViewState, HabitViewState, InputState,
    MultiSelectState, RunningState, ViewSelectionState,
};
