//! Entity handlers for the pipe interface.
//!
//! Each handler module implements CRUD operations for a specific entity type.

pub mod export;
pub mod goal;
pub mod habit;
pub mod import;
pub mod project;
pub mod tag;
pub mod task;
pub mod time_entry;
pub mod work_log;

use taskflow::app::Model;

use super::types::{EntityType, Operation, PipeError, PipeRequest};

/// Result type for handlers.
pub type HandlerResult = Result<serde_json::Value, PipeError>;

/// Dispatch a request to the appropriate handler.
pub fn dispatch(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    // Handle special operations first
    match request.operation {
        Operation::Export => return export::handle_export(model, request),
        Operation::Import => return import::handle_import(model, request),
        _ => {}
    }

    // Dispatch to entity handler
    match request.entity {
        EntityType::Task => task::handle(model, request),
        EntityType::Project => project::handle(model, request),
        EntityType::TimeEntry => time_entry::handle(model, request),
        EntityType::WorkLog => work_log::handle(model, request),
        EntityType::Habit => habit::handle(model, request),
        EntityType::Goal => goal::handle(model, request),
        EntityType::KeyResult => goal::handle_key_result(model, request),
        EntityType::Tag => tag::handle(model, request),
        EntityType::SavedFilter => Err(PipeError::new(
            "NOT_IMPLEMENTED",
            "SavedFilter operations are not yet implemented",
        )),
    }
}
