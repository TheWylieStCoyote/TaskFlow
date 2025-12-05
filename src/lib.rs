//! # TaskFlow
//!
//! A terminal-based task and project management application built with Rust.
//!
//! TaskFlow provides a fast, keyboard-driven TUI interface for managing tasks,
//! projects, and time tracking. It follows The Elm Architecture (TEA) pattern
//! for predictable state management.
//!
//! ## Architecture
//!
//! The application is structured into several modules:
//!
//! - [`app`] - Application state management (Model, Message, Update)
//! - [`domain`] - Core domain entities (Task, Project, TimeEntry)
//! - [`storage`] - Persistence backends (JSON, YAML, SQLite, Markdown)
//! - [`config`] - Configuration management (settings, themes, keybindings)
//! - [`ui`] - Terminal UI rendering with Ratatui
//!
//! ## The Elm Architecture (TEA)
//!
//! TaskFlow uses TEA for state management:
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
//! - JSON (default) - Fast and compact
//! - YAML - Human-readable and editable
//! - SQLite - Efficient for large datasets
//! - Markdown - Integration with other tools
//!
//! ## Example
//!
//! ```no_run
//! use taskflow::app::Model;
//! use taskflow::domain::Task;
//!
//! let mut model = Model::new();
//! let task = Task::new("My first task");
//! model.tasks.insert(task.id.clone(), task);
//! ```

pub mod app;
pub mod config;
pub mod domain;
pub mod storage;
pub mod ui;
