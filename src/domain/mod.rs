//! Core domain entities for task management.
//!
//! This module contains the fundamental data types that represent
//! the application's business logic:
//!
//! - [`Task`] - A work item with title, status, priority, and metadata
//! - [`Project`] - A container for organizing related tasks
//! - [`Tag`] - A label for categorizing tasks
//! - [`TimeEntry`] - A time tracking record for a task
//! - [`Filter`] - Query parameters for filtering tasks
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
//! - `Urgent` - Highest priority (!!!!）
//! - `High` - Important tasks (!!!)
//! - `Medium` - Standard priority (!!)
//! - `Low` - Less urgent (!)
//! - `None` - No priority set

mod filter;
mod project;
mod tag;
mod task;
mod time_entry;

pub use filter::*;
pub use project::*;
pub use tag::*;
pub use task::*;
pub use time_entry::*;
