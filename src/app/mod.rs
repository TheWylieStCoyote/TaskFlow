//! Application state management following The Elm Architecture (TEA).
//!
//! This module contains the core application logic:
//!
//! - [`Model`] - The complete application state
//! - [`Message`] - Events that can modify state
//! - [`update`] - The update function that handles messages
//!
//! ## The Elm Architecture
//!
//! The application follows TEA, where:
//!
//! 1. The [`Model`] holds all application state
//! 2. User actions and system events generate [`Message`]s
//! 3. The [`update`] function processes messages and returns new state
//! 4. The UI renders based on current state (see [`crate::ui`])
//!
//! This pattern ensures predictable state transitions and makes the
//! application easy to test and reason about.
//!
//! ## Key Types
//!
//! - [`MacroState`] - Keyboard macro recording and playback
//! - [`Message`] - Message types for state changes
//! - [`Model`] - Application state struct
//! - [`TemplateManager`] - Task templates for quick creation
//! - [`UndoStack`] - Undo/redo action history
//! - [`update`] - Message handling and state transitions

mod macros;
mod message;
mod model;
mod templates;
mod undo;
mod update;

pub use macros::*;
pub use message::*;
pub use model::*;
pub use templates::*;
pub use undo::*;
pub use update::*;
