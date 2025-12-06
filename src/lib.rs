//! # `TaskFlow`
//!
//! A terminal-based task and project management application built with Rust.
//!
//! `TaskFlow` provides a fast, keyboard-driven TUI interface for managing tasks,
//! projects, and time tracking. It follows The Elm Architecture (TEA) pattern
//! for predictable state management.
//!
//! ## Features
//!
//! - **Task Management**: Create, edit, complete, and organize tasks
//! - **Project Organization**: Group tasks into projects
//! - **Time Tracking**: Track time spent on tasks with start/stop timers
//! - **Multiple Views**: Task list, Today, Upcoming, Overdue, Calendar, Dashboard
//! - **Keyboard-Driven**: Full vim-style navigation and shortcuts
//! - **Multiple Storage Backends**: JSON, YAML, `SQLite`, Markdown
//! - **Customizable**: Themes, keybindings, and settings
//!
//! ## Quick Start
//!
//! ### Creating Tasks
//!
//! ```
//! use taskflow::domain::{Task, Priority, TaskStatus};
//! use chrono::Utc;
//!
//! // Create a simple task
//! let task = Task::new("Write documentation");
//!
//! // Create a task with priority and due date
//! let today = Utc::now().date_naive();
//! let urgent_task = Task::new("Fix critical bug")
//!     .with_priority(Priority::Urgent)
//!     .with_due_date(today)
//!     .with_tags(vec!["bug".into(), "critical".into()]);
//!
//! // Check task state
//! assert!(!urgent_task.status.is_complete());
//! assert!(urgent_task.is_due_today());
//! ```
//!
//! ### Working with Projects
//!
//! ```
//! use taskflow::domain::{Project, Task};
//!
//! // Create a project
//! let project = Project::new("Backend API")
//!     .with_color("#3498db");
//!
//! // Create a task in the project
//! let task = Task::new("Implement REST endpoints")
//!     .with_project(project.id.clone());
//!
//! assert!(task.project_id.is_some());
//! ```
//!
//! ### Time Tracking
//!
//! ```
//! use taskflow::domain::{Task, TimeEntry};
//!
//! let task = Task::new("Code review");
//!
//! // Start tracking time
//! let mut entry = TimeEntry::start(task.id.clone());
//! assert!(entry.is_running());
//!
//! // Stop tracking
//! entry.stop();
//! assert!(!entry.is_running());
//!
//! // Get formatted duration
//! let duration = entry.formatted_duration(); // e.g., "45m" or "1h 30m"
//! ```
//!
//! ### Using the Application Model
//!
//! ```
//! use taskflow::app::Model;
//! use taskflow::domain::{Task, Priority};
//!
//! // Create application state
//! let mut model = Model::new();
//!
//! // Add a task
//! let task = Task::new("My first task")
//!     .with_priority(Priority::High);
//! let task_id = task.id.clone();
//! model.tasks.insert(task_id.clone(), task);
//!
//! // Refresh the visible task list
//! model.refresh_visible_tasks();
//!
//! // Task is now visible
//! assert!(model.visible_tasks.contains(&task_id));
//! ```
//!
//! ## Architecture
//!
//! The application is structured into several modules:
//!
//! - [`app`] - Application state management (Model, Message, Update)
//! - [`domain`] - Core domain entities (Task, Project, `TimeEntry`)
//! - [`storage`] - Persistence backends (JSON, YAML, `SQLite`, Markdown)
//! - [`config`] - Configuration management (settings, themes, keybindings)
//! - [`ui`] - Terminal UI rendering with Ratatui
//!
//! ## The Elm Architecture (TEA)
//!
//! `TaskFlow` uses TEA for state management:
//!
//! ```text
//! ┌─────────┐    ┌──────────┐    ┌─────────┐
//! │  Model  │───▶│  Update  │───▶│  View   │
//! │ (State) │    │(Messages)│    │  (UI)   │
//! └─────────┘    └──────────┘    └─────────┘
//!      ▲                              │
//!      └──────────────────────────────┘
//!                 User Input
//! ```
//!
//! - **Model**: Central application state in [`app::Model`]
//! - **Message**: Events that can change state in [`app::Message`]
//! - **Update**: Pure function that produces new state from current state + message
//! - **View**: Renders the UI based on current state
//!
//! ## Storage Backends
//!
//! Multiple storage backends are supported via the [`storage::StorageBackend`] trait:
//!
//! ```
//! use taskflow::storage::BackendType;
//!
//! // Available backends
//! let json = BackendType::Json;      // Fast and compact (default)
//! let yaml = BackendType::Yaml;      // Human-readable and editable
//! let sqlite = BackendType::Sqlite;  // Efficient for large datasets
//! let markdown = BackendType::Markdown; // Integration with other tools
//! ```
//!
//! ## Keyboard Shortcuts
//!
//! `TaskFlow` uses vim-style navigation:
//!
//! | Key | Action |
//! |-----|--------|
//! | `j`/`k` | Navigate down/up |
//! | `a` | Add new task |
//! | `e` | Edit task |
//! | `x`/`Space` | Toggle complete |
//! | `p` | Cycle priority |
//! | `d` | Delete task |
//! | `?` | Show help |
//! | `q` | Quit |
//!
//! ## Priority Levels
//!
//! Tasks can have one of five priority levels:
//!
//! | Priority | Symbol | Color |
//! |----------|--------|-------|
//! | Urgent | `!!!!` | Red |
//! | High | `!!!` | Yellow |
//! | Medium | `!!` | Cyan |
//! | Low | `!` | Green |
//! | None | (none) | Gray |
//!
//! ## Task Status
//!
//! Tasks can be in one of five states:
//!
//! | Status | Symbol | Description |
//! |--------|--------|-------------|
//! | Todo | `[ ]` | Not started |
//! | InProgress | `[~]` | Currently working on |
//! | Blocked | `[!]` | Waiting on something |
//! | Done | `[x]` | Completed |
//! | Cancelled | `[-]` | Cancelled |

pub mod app;
pub mod config;
pub mod domain;
pub mod scripting;
pub mod storage;
pub mod ui;
