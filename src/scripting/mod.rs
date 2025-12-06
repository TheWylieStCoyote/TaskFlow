//! Scripting system for TaskFlow.
//!
//! This module provides a Rhai-based scripting engine that allows users to:
//! - Define hooks that run on task events (create, complete, status change, etc.)
//! - Create custom commands invokable via keyboard shortcuts
//! - Automate workflows with script logic
//!
//! ## Configuration
//!
//! Scripts are configured via `~/.config/taskflow/hooks.toml`:
//!
//! ```toml
//! [settings]
//! enabled = true
//! timeout = 5
//!
//! [hooks.on_task_completed]
//! enabled = true
//! script = """
//!     if task.tags.contains("recurring") {
//!         create_task("Follow-up: " + task.title);
//!     }
//! """
//! ```
//!
//! ## Available Hooks
//!
//! - `on_task_created` - Triggered when a new task is created
//! - `on_task_completed` - Triggered when a task is marked complete
//! - `on_task_status_changed` - Triggered when task status changes
//! - `on_task_priority_changed` - Triggered when task priority changes
//! - `on_time_tracking_started` - Triggered when time tracking begins
//! - `on_time_tracking_stopped` - Triggered when time tracking ends
//!
//! ## Script API
//!
//! Scripts have access to these functions:
//!
//! - `create_task(title)` - Create a new task
//! - `complete_task(id)` - Mark a task complete
//! - `add_tag(id, tag)` - Add a tag to a task
//! - `log(message)` - Log a message
//! - `today()` / `tomorrow()` - Get date values

mod actions;
mod api;
mod config;
mod engine;
mod error;
mod event;

pub use actions::ScriptAction;
pub use config::{CommandConfig, HookConfig, ScriptConfig};
pub use engine::ScriptEngine;
pub use error::{ScriptError, ScriptResult};
pub use event::HookEvent;
