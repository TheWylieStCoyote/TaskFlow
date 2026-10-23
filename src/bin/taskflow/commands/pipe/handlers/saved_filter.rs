//! Saved filter CRUD operations for the pipe interface.
//!
//! A saved filter is a named `Filter` plus a `SortSpec`, shown in the TUI
//! sidebar. Its `id` is an opaque string, not a UUID. The `filter` and `sort`
//! payloads use the same JSON shape `export` writes under `saved_filters`.

use taskflow::app::Model;
use taskflow::domain::{Filter, SavedFilter, SavedFilterId, SortSpec};

use super::HandlerResult;
use crate::commands::pipe::types::{
    FilterParams, Operation, PipeError, PipeRequest, SavedFilterInput,
};

/// Handle saved filter operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_saved_filters(model, request.filters.as_ref()),
        Operation::Get => get_saved_filter(model, request.id.as_deref()),
        Operation::Create => create_saved_filter(model, request.data.as_ref()),
        Operation::Update => {
            update_saved_filter(model, request.id.as_deref(), request.data.as_ref())
        }
        Operation::Delete => delete_saved_filter(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

/// List saved filters sorted by name (case-insensitive, then id).
///
/// `search` matches the name; `limit` (default 100) and `offset` page the
/// sorted list.
fn list_saved_filters(model: &Model, filters: Option<&FilterParams>) -> HandlerResult {
    let mut saved: Vec<&SavedFilter> = model.saved_filters.values().collect();

    if let Some(search) = filters.and_then(|f| f.search.as_deref()) {
        let needle = search.to_lowercase();
        saved.retain(|s| s.name.to_lowercase().contains(&needle));
    }

    saved.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.0.cmp(&b.id.0))
    });

    let total = saved.len();
    let offset = filters.and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.and_then(|f| f.limit).unwrap_or(100);
    let saved: Vec<SavedFilter> = saved
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(serde_json::json!({
        "saved_filters": saved,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn require_id(id: Option<&str>) -> Result<SavedFilterId, PipeError> {
    let id = id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| PipeError::new("MISSING_ID", "Saved filter ID is required"))?;
    Ok(SavedFilterId(id.to_string()))
}

fn not_found(id: &SavedFilterId) -> PipeError {
    PipeError::new("NOT_FOUND", format!("Saved filter not found: {}", id.0))
}

fn get_saved_filter(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = require_id(id)?;
    model
        .saved_filters
        .get(&id)
        .ok_or_else(|| not_found(&id))
        .and_then(|s| serde_json::to_value(s).map_err(PipeError::serialization))
}

fn create_saved_filter(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data =
        data.ok_or_else(|| PipeError::new("MISSING_DATA", "Saved filter data is required"))?;
    let input: SavedFilterInput = serde_json::from_value(data.clone()).map_err(|e| {
        PipeError::new(
            "INVALID_DATA",
            format!("Failed to parse saved filter data: {e}"),
        )
    })?;

    let name = input
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Saved filter name is required"))?;

    let mut saved = SavedFilter::new(
        name,
        input.filter.unwrap_or_default(),
        input.sort.unwrap_or_default(),
    );
    saved.icon = input.icon.filter(|i| !i.is_empty());

    model.sync_saved_filter(&saved);
    model.saved_filters.insert(saved.id.clone(), saved.clone());

    serde_json::to_value(&saved).map_err(PipeError::serialization)
}

/// Patch `name`, `filter`, `sort` and `icon`; `null` clears the icon, and a
/// `filter` or `sort` object replaces the stored one whole.
fn update_saved_filter(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = require_id(id)?;
    let data =
        data.ok_or_else(|| PipeError::new("MISSING_DATA", "Saved filter data is required"))?;
    let saved = model
        .saved_filters
        .get_mut(&id)
        .ok_or_else(|| not_found(&id))?;

    if let Some(name) = data.get("name") {
        let name = name
            .as_str()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .ok_or_else(|| {
                PipeError::new(
                    "INVALID_DATA",
                    "Saved filter name must be a non-empty string",
                )
            })?;
        saved.name = name.to_string();
    }
    if let Some(filter) = data.get("filter") {
        saved.filter = serde_json::from_value::<Filter>(filter.clone())
            .map_err(|e| PipeError::new("INVALID_DATA", format!("Invalid filter: {e}")))?;
    }
    if let Some(sort) = data.get("sort") {
        saved.sort = serde_json::from_value::<SortSpec>(sort.clone())
            .map_err(|e| PipeError::new("INVALID_DATA", format!("Invalid sort: {e}")))?;
    }
    if let Some(icon) = data.get("icon") {
        saved.icon = match icon {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) if s.is_empty() => None,
            serde_json::Value::String(s) => Some(s.clone()),
            _ => {
                return Err(PipeError::new(
                    "INVALID_DATA",
                    "Saved filter icon must be a string or null",
                ))
            }
        };
    }

    let updated = saved.clone();
    model.sync_saved_filter(&updated);
    serde_json::to_value(&updated).map_err(PipeError::serialization)
}

fn delete_saved_filter(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = require_id(id)?;
    model
        .saved_filters
        .remove(&id)
        .ok_or_else(|| not_found(&id))?;
    model.delete_saved_filter_from_storage(&id);
    Ok(serde_json::json!({ "deleted": id.0 }))
}
