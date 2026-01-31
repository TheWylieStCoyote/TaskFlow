//! Application state management following The Elm Architecture (TEA).
//!
//! This module contains the core application logic:
//!
//! - [`Model`] - The complete application state
//! - [`Message`] - Events that can modify state
//! - [`update()`] - The update function that handles messages
//!
//! ## The Elm Architecture
//!
//! The application follows TEA (The Elm Architecture), a unidirectional
//! data flow pattern where:
//!
//! 1. **Model**: The [`Model`] holds all application state in a single struct
//! 2. **Message**: User actions and system events generate [`Message`]s
//! 3. **Update**: The [`update()`] function processes messages and modifies state
//! 4. **View**: The UI renders based on current state (see [`crate::ui`])
//!
//! ```text
//! ┌─────────┐     ┌──────────┐     ┌────────┐
//! │  View   │────▶│  Message │────▶│ Update │
//! └─────────┘     └──────────┘     └────────┘
//!      ▲                                │
//!      │          ┌──────────┐          │
//!      └──────────│  Model   │◀─────────┘
//!                 └──────────┘
//! ```
//!
//! This pattern ensures predictable state transitions and makes the
//! application easy to test and reason about.
//!
//! ## Quick Start
//!
//! ### Creating and Updating State
//!
//! ```
//! use taskflow::app::{Model, Message, NavigationMessage, TaskMessage, update};
//! use taskflow::domain::Task;
//!
//! // Initialize application state
//! let mut model = Model::new();
//!
//! // Create a task by sending a message
//! update(&mut model, Message::Task(TaskMessage::Create("My task".to_string())));
//!
//! // Navigate through tasks
//! update(&mut model, Message::Navigation(NavigationMessage::Down));
//!
//! // Toggle task completion
//! update(&mut model, Message::Task(TaskMessage::ToggleComplete));
//! ```
//!
//! ### Working with Views
//!
//! ```
//! use taskflow::app::{Model, Message, NavigationMessage, ViewId, update};
//!
//! let mut model = Model::new().with_sample_data();
//!
//! // Switch to Today view
//! update(&mut model, Message::Navigation(NavigationMessage::GoToView(ViewId::Today)));
//!
//! // Switch to Calendar view
//! update(&mut model, Message::Navigation(NavigationMessage::GoToView(ViewId::Calendar)));
//! ```
//!
//! ### Using Undo/Redo
//!
//! ```
//! use taskflow::app::{Model, Message, SystemMessage, TaskMessage, update};
//!
//! let mut model = Model::new();
//!
//! // Make some changes
//! update(&mut model, Message::Task(TaskMessage::Create("Task 1".to_string())));
//! update(&mut model, Message::Task(TaskMessage::Create("Task 2".to_string())));
//!
//! // Undo the last change
//! update(&mut model, Message::System(SystemMessage::Undo));
//!
//! // Redo if needed
//! update(&mut model, Message::System(SystemMessage::Redo));
//! ```
//!
//! ## Key Types
//!
//! | Type | Description |
//! |------|-------------|
//! | [`Model`] | Complete application state |
//! | [`Message`] | Top-level message enum |
//! | [`NavigationMessage`] | Movement and view switching |
//! | [`TaskMessage`] | Task CRUD operations |
//! | [`UiMessage`] | UI state changes (input, dialogs) |
//! | [`SystemMessage`] | App-level actions (save, quit) |
//! | [`TimeMessage`] | Time tracking operations |
//! | [`ViewId`] | View identifiers |
//! | [`FocusPane`] | Which pane has focus |
//! | [`MacroState`] | Keyboard macro recording |
//! | [`TemplateManager`] | Task templates |
//! | [`UndoStack`] | Undo/redo history |
//!
//! ## Message Categories
//!
//! Messages are organized into categories for clarity:
//!
//! - **Navigation**: Movement (`Up`, `Down`), view changes (`GoToView`)
//! - **Task**: CRUD operations (`Create`, `Delete`, `ToggleComplete`)
//! - **UI**: Input handling, dialogs, multi-select
//! - **System**: Save, quit, undo/redo, export
//! - **Time**: Time tracking start/stop

pub mod analytics;
mod macros;
mod message;
mod model;
pub mod quick_add;
mod templates;
mod undo;
mod update;

pub use macros::*;
pub use message::*;
pub use model::*;
pub use quick_add::{parse_date, parse_date_with_reference, parse_quick_add};
pub use templates::*;
pub use undo::*;
pub use update::*;

#[cfg(test)]
mod e2e_workflow_tests;
