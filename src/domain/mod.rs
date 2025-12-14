//! Core domain entities for task management.
//!
//! This module contains the fundamental data types that represent
//! the application's business logic:
//!
//! - [`Task`] - A work item with title, status, priority, and metadata
//! - [`Project`] - A container for organizing related tasks
//! - [`Goal`] - An OKR-style objective with timeframe and progress tracking
//! - [`KeyResult`] - A measurable outcome linked to a Goal
//! - [`Tag`] - A label for categorizing tasks
//! - [`TimeEntry`] - A time tracking record for a task
//! - [`Filter`] - Query parameters for filtering tasks
//!
//! ## Quick Examples
//!
//! ### Creating and Managing Tasks
//!
//! ```
//! use taskflow::domain::{Task, Priority, TaskStatus};
//! use chrono::Utc;
//!
//! // Create a task with the builder pattern
//! let task = Task::new("Implement login feature")
//!     .with_priority(Priority::High)
//!     .with_tags(vec!["backend".into(), "auth".into()])
//!     .with_description("Add OAuth2 support".to_string());
//!
//! assert_eq!(task.priority, Priority::High);
//! assert_eq!(task.tags.len(), 2);
//!
//! // Toggle completion
//! let mut task = task;
//! task.toggle_complete();
//! assert_eq!(task.status, TaskStatus::Done);
//! assert!(task.completed_at.is_some());
//! ```
//!
//! ### Working with Projects
//!
//! ```
//! use taskflow::domain::{Project, ProjectStatus, Task};
//!
//! // Create a project hierarchy
//! let parent = Project::new("Engineering");
//! let child = Project::new("Backend Team")
//!     .with_parent(parent.id)
//!     .with_color("#3498db");
//!
//! // Associate tasks with projects
//! let task = Task::new("Setup CI/CD")
//!     .with_project(child.id);
//!
//! assert!(child.is_active());
//! assert_eq!(task.project_id, Some(child.id));
//! ```
//!
//! ### Time Tracking
//!
//! ```
//! use taskflow::domain::{Task, TimeEntry};
//!
//! let task = Task::new("Write tests");
//!
//! // Start a time entry
//! let mut entry = TimeEntry::start(task.id);
//! assert!(entry.is_running());
//!
//! // Stop and get duration
//! entry.stop();
//! println!("Time spent: {}", entry.formatted_duration());
//! ```
//!
//! ## Task Status Flow
//!
//! Tasks progress through various states:
//!
//! ```text
//! Todo → InProgress → Done
//!   │         │
//!   └──→ Blocked ──→ InProgress
//!   │
//!   └──→ Cancelled
//! ```
//!
//! ## Priority Levels
//!
//! Tasks can have one of five priority levels:
//!
//! | Priority | Symbol | Use Case |
//! |----------|--------|----------|
//! | `Urgent` | `!!!!` | Critical issues, production bugs |
//! | `High` | `!!!` | Important features, deadlines |
//! | `Medium` | `!!` | Standard work items |
//! | `Low` | `!` | Nice-to-haves, backlog |
//! | `None` | (none) | Uncategorized tasks |
//!
//! ## Recurring Tasks
//!
//! Tasks can have recurrence patterns for repeating work:
//!
//! ```
//! use taskflow::domain::{Task, Recurrence};
//!
//! // Daily standup
//! let daily = Task::new("Team standup")
//!     .with_recurrence(Some(Recurrence::Daily));
//!
//! // Weekly review on Fridays
//! let weekly = Task::new("Sprint review")
//!     .with_recurrence(Some(Recurrence::Weekly {
//!         days: vec![chrono::Weekday::Fri]
//!     }));
//!
//! // Monthly report on the 1st
//! let monthly = Task::new("Monthly report")
//!     .with_recurrence(Some(Recurrence::Monthly { day: 1 }));
//! ```

pub mod analytics;
mod calendar_event;
pub mod duplicate_detector;
mod filter;
pub mod filter_dsl;
pub mod git;
mod goal;
mod habit;
mod key_result;
mod pomodoro;
mod project;
mod tag;
mod task;
mod time_entry;
mod work_log;

#[cfg(test)]
mod proptest_tests;

pub use calendar_event::*;
pub use filter::*;
pub use goal::*;
pub use habit::*;
pub use key_result::*;
pub use pomodoro::*;
pub use project::*;
pub use tag::*;
pub use task::*;
pub use time_entry::*;
pub use work_log::*;

/// Returns "s" for plural counts, "" for singular.
///
/// Useful for constructing messages like "1 task" vs "2 tasks".
///
/// # Examples
///
/// ```
/// use taskflow::domain::plural_s;
///
/// assert_eq!(format!("1 task{}", plural_s(1)), "1 task");
/// assert_eq!(format!("2 task{}", plural_s(2)), "2 tasks");
/// assert_eq!(format!("0 task{}", plural_s(0)), "0 tasks");
/// ```
#[must_use]
pub const fn plural_s(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}
