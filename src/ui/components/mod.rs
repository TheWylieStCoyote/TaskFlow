//! UI components for the TaskFlow TUI.
//!
//! This module provides all the visual components used to render the TaskFlow
//! terminal user interface. Each component is a Ratatui widget that can be
//! composed to build the complete UI.
//!
//! # Component Categories
//!
//! ## Core Views
//! - [`TaskList`] - Main task list with selection and status indicators
//! - [`Sidebar`] - Navigation panel with views and projects
//! - [`FocusView`] - Detailed single-task view
//!
//! ## Specialized Views
//! - [`Calendar`] - Monthly calendar with task indicators
//! - [`Kanban`] - Kanban board with status columns
//! - [`Eisenhower`] - Urgency/importance matrix
//! - [`WeeklyPlanner`] - Week-at-a-glance planning view
//! - [`Timeline`] - Chronological task timeline
//! - [`Network`] - Task dependency graph visualization
//!
//! ## Analytics & Reports
//! - [`Dashboard`] - Overview statistics and metrics
//! - [`Reports`] - Detailed analytics panels
//! - [`Burndown`] - Project burndown charts
//! - [`Forecast`] - Workload forecasting
//! - [`Heatmap`] - GitHub-style activity heatmap
//!
//! ## Editors & Pickers
//! - [`InputWidget`] - Text input with cursor
//! - [`DescriptionEditor`] - Multi-line text editor
//! - [`TemplatePicker`] - Task template selection
//! - [`SavedFilterPicker`] - Filter preset selection
//! - [`KeybindingsEditor`] - Keybinding customization
//! - [`TimeLogEditor`] - Time entry management
//! - [`WorkLogEditor`] - Work log entry editor
//!
//! ## Reviews
//! - [`DailyReview`] - Daily task review workflow
//! - [`WeeklyReview`] - Weekly planning review
//!
//! ## Utilities
//! - [`Help`] - Keybinding help popup
//! - [`Habits`] - Habit tracking view
//! - [`charts`] - Shared chart rendering utilities

mod burndown;
mod calendar;
pub mod charts;
mod daily_review;
mod dashboard;
mod description_editor;
mod duplicates;
mod eisenhower;
mod focus_view;
mod forecast;
mod goals;
mod git_todos;
mod habits;
mod heatmap;
mod help;
mod input;
mod kanban;
mod keybindings_editor;
mod network;
mod reports;
mod saved_filter_picker;
mod sidebar;
mod task_list;
mod template_picker;
mod time_log_editor;
mod timeline;
mod weekly_planner;
mod weekly_review;
mod work_log_editor;

pub use burndown::*;
pub use calendar::*;
pub use daily_review::*;
pub use dashboard::*;
pub use description_editor::*;
pub use duplicates::*;
pub use eisenhower::*;
pub use focus_view::*;
pub use forecast::*;
pub use goals::*;
pub use git_todos::*;
pub use habits::*;
pub use heatmap::*;
pub use help::*;
pub use input::*;
pub use kanban::*;
pub use keybindings_editor::*;
pub use network::*;
pub use reports::*;
pub use saved_filter_picker::*;
pub use sidebar::*;
pub use task_list::*;
pub use template_picker::*;
pub use time_log_editor::*;
pub use timeline::*;
pub use weekly_planner::*;
pub use weekly_review::*;
pub use work_log_editor::*;
