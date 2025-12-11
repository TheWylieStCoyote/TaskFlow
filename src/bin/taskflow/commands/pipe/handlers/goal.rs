//! Goal and KeyResult CRUD operations for the pipe interface.

use chrono::NaiveDate;
use uuid::Uuid;

use taskflow::app::Model;
use taskflow::domain::{Goal, GoalId, GoalStatus, KeyResult, KeyResultId, Quarter};

use super::HandlerResult;
use crate::commands::pipe::types::{
    FilterParams, GoalInput, KeyResultInput, Operation, PipeError, PipeRequest,
};

/// Handle goal operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_goals(model, &request.filters),
        Operation::Get => get_goal(model, request.id.as_deref()),
        Operation::Create => create_goal(model, request.data.as_ref()),
        Operation::Update => update_goal(model, request.id.as_deref(), request.data.as_ref()),
        Operation::Delete => delete_goal(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

/// Handle key result operations.
pub fn handle_key_result(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_key_results(model, &request.filters),
        Operation::Get => get_key_result(model, request.id.as_deref()),
        Operation::Create => create_key_result(model, request.data.as_ref()),
        Operation::Update => update_key_result(model, request.id.as_deref(), request.data.as_ref()),
        Operation::Delete => delete_key_result(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

// ============================================================================
// Goal operations
// ============================================================================

fn list_goals(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    let mut goals: Vec<&Goal> = model.goals.values().collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by search text
        if let Some(ref search) = f.search {
            let search_lower = search.to_lowercase();
            goals.retain(|g| {
                g.name.to_lowercase().contains(&search_lower)
                    || g.description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&search_lower))
            });
        }

        // Filter by status
        if let Some(ref statuses) = f.status {
            let status_set: Vec<GoalStatus> = statuses
                .iter()
                .filter_map(|s| parse_goal_status(s))
                .collect();
            if !status_set.is_empty() {
                goals.retain(|g| status_set.contains(&g.status));
            }
        }

        // Filter by completion (archived/completed status)
        if !f.include_completed.unwrap_or(false) {
            goals.retain(|g| g.status.is_active() || g.status == GoalStatus::OnHold);
        }

        // Sort by name
        goals.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    let total = goals.len();
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let goals: Vec<Goal> = goals
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(serde_json::json!({
        "goals": goals,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_goal(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Goal ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let goal_id = GoalId(uuid);
    model
        .goals
        .get(&goal_id)
        .map(|g| serde_json::to_value(g).unwrap())
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Goal not found: {id}")))
}

fn create_goal(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Goal data is required"))?;

    let input: GoalInput = serde_json::from_value(data.clone())
        .map_err(|e| PipeError::new("INVALID_DATA", format!("Failed to parse goal data: {e}")))?;

    let name = input
        .name
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Goal name is required"))?;

    let mut goal = Goal::new(&name);

    if let Some(ref desc) = input.description {
        goal.description = Some(desc.clone());
    }
    if let Some(ref date_str) = input.target_date {
        goal.due_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok();
    }

    let goal_id = goal.id;
    model.goals.insert(goal_id, goal.clone());
    model.sync_goal(&goal);

    Ok(serde_json::to_value(&goal).unwrap())
}

fn update_goal(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Goal ID is required"))?;
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Goal data is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let goal_id = GoalId(uuid);

    let goal = model
        .goals
        .get_mut(&goal_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Goal not found: {id}")))?;

    if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
        goal.name = name.to_string();
    }
    if let Some(desc) = data.get("description") {
        goal.description = if desc.is_null() {
            None
        } else {
            desc.as_str().map(|s| s.to_string())
        };
    }
    if let Some(status_str) = data.get("status").and_then(|v| v.as_str()) {
        if let Some(status) = parse_goal_status(status_str) {
            goal.status = status;
        }
    }
    if let Some(start) = data.get("start_date") {
        goal.start_date = if start.is_null() {
            None
        } else {
            start
                .as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        };
    }
    if let Some(due) = data.get("due_date") {
        goal.due_date = if due.is_null() {
            None
        } else {
            due.as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        };
    }
    if let Some(quarter_obj) = data.get("quarter") {
        if quarter_obj.is_null() {
            goal.quarter = None;
        } else if let (Some(year), Some(q_str)) = (
            quarter_obj.get("year").and_then(|v| v.as_i64()),
            quarter_obj.get("quarter").and_then(|v| v.as_str()),
        ) {
            if let Some(q) = parse_quarter(q_str) {
                goal.quarter = Some((year as i32, q));
            }
        }
    }
    if let Some(progress) = data.get("manual_progress") {
        goal.manual_progress = if progress.is_null() {
            None
        } else {
            progress.as_u64().map(|v| v.min(100) as u8)
        };
    }
    if let Some(color) = data.get("color") {
        goal.color = if color.is_null() {
            None
        } else {
            color.as_str().map(|s| s.to_string())
        };
    }
    if let Some(icon) = data.get("icon") {
        goal.icon = if icon.is_null() {
            None
        } else {
            icon.as_str().map(|s| s.to_string())
        };
    }

    goal.updated_at = chrono::Utc::now();
    let updated_goal = goal.clone();
    model.sync_goal(&updated_goal);

    Ok(serde_json::to_value(&updated_goal).unwrap())
}

fn delete_goal(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Goal ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let goal_id = GoalId(uuid);

    model
        .goals
        .remove(&goal_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Goal not found: {id}")))?;

    // Also delete associated key results
    let kr_ids: Vec<KeyResultId> = model
        .key_results
        .values()
        .filter(|kr| kr.goal_id == goal_id)
        .map(|kr| kr.id)
        .collect();
    for kr_id in &kr_ids {
        model.key_results.remove(kr_id);
        model.delete_key_result_from_storage(kr_id);
    }

    model.delete_goal_from_storage(&goal_id);

    Ok(serde_json::json!({
        "deleted": id,
        "deleted_key_results": kr_ids.len()
    }))
}

// ============================================================================
// KeyResult operations
// ============================================================================

fn list_key_results(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    let mut key_results: Vec<&KeyResult> = model.key_results.values().collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by goal (using project_id field as goal filter)
        if let Some(ref goal_id_str) = f.project_id {
            if let Ok(uuid) = Uuid::parse_str(goal_id_str) {
                let goal_id = GoalId(uuid);
                key_results.retain(|kr| kr.goal_id == goal_id);
            }
        }

        // Filter by search text
        if let Some(ref search) = f.search {
            let search_lower = search.to_lowercase();
            key_results.retain(|kr| {
                kr.name.to_lowercase().contains(&search_lower)
                    || kr
                        .description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&search_lower))
            });
        }

        // Sort by name
        key_results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    let total = key_results.len();
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let key_results: Vec<KeyResult> = key_results
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(serde_json::json!({
        "key_results": key_results,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_key_result(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Key result ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let kr_id = KeyResultId(uuid);
    model
        .key_results
        .get(&kr_id)
        .map(|kr| serde_json::to_value(kr).unwrap())
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Key result not found: {id}")))
}

fn create_key_result(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Key result data is required"))?;

    let input: KeyResultInput = serde_json::from_value(data.clone()).map_err(|e| {
        PipeError::new(
            "INVALID_DATA",
            format!("Failed to parse key result data: {e}"),
        )
    })?;

    let goal_id_str = input
        .goal_id
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Goal ID is required"))?;

    let goal_id = Uuid::parse_str(&goal_id_str)
        .map(GoalId)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid goal UUID: {goal_id_str}")))?;

    // Verify goal exists
    if !model.goals.contains_key(&goal_id) {
        return Err(PipeError::new(
            "NOT_FOUND",
            format!("Goal not found: {goal_id_str}"),
        ));
    }

    let name = input
        .name
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Key result name is required"))?;

    let target_value = input.target_value.unwrap_or(100.0);

    let mut kr = KeyResult::new(goal_id, &name).with_target(target_value, input.unit.as_deref());

    if let Some(current) = input.current_value {
        kr.current_value = current;
    }

    let kr_id = kr.id;
    model.key_results.insert(kr_id, kr.clone());
    model.sync_key_result(&kr);

    Ok(serde_json::to_value(&kr).unwrap())
}

fn update_key_result(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Key result ID is required"))?;
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Key result data is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let kr_id = KeyResultId(uuid);

    let kr = model
        .key_results
        .get_mut(&kr_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Key result not found: {id}")))?;

    if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
        kr.name = name.to_string();
    }
    if let Some(desc) = data.get("description") {
        kr.description = if desc.is_null() {
            None
        } else {
            desc.as_str().map(|s| s.to_string())
        };
    }
    if let Some(target) = data.get("target_value").and_then(|v| v.as_f64()) {
        kr.target_value = target;
    }
    if let Some(current) = data.get("current_value").and_then(|v| v.as_f64()) {
        kr.current_value = current;
    }
    if let Some(unit) = data.get("unit") {
        kr.unit = if unit.is_null() {
            None
        } else {
            unit.as_str().map(|s| s.to_string())
        };
    }
    if let Some(progress) = data.get("manual_progress") {
        kr.manual_progress = if progress.is_null() {
            None
        } else {
            progress.as_u64().map(|v| v.min(100) as u8)
        };
    }

    kr.updated_at = chrono::Utc::now();
    let updated_kr = kr.clone();
    model.sync_key_result(&updated_kr);

    Ok(serde_json::to_value(&updated_kr).unwrap())
}

fn delete_key_result(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Key result ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let kr_id = KeyResultId(uuid);

    model
        .key_results
        .remove(&kr_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Key result not found: {id}")))?;

    model.delete_key_result_from_storage(&kr_id);

    Ok(serde_json::json!({ "deleted": id }))
}

// ============================================================================
// Helper functions
// ============================================================================

fn parse_goal_status(s: &str) -> Option<GoalStatus> {
    match s.to_lowercase().replace(['-', '_'], "").as_str() {
        "active" => Some(GoalStatus::Active),
        "onhold" | "hold" => Some(GoalStatus::OnHold),
        "completed" | "done" => Some(GoalStatus::Completed),
        "archived" => Some(GoalStatus::Archived),
        _ => None,
    }
}

fn parse_quarter(s: &str) -> Option<Quarter> {
    match s.to_uppercase().as_str() {
        "Q1" => Some(Quarter::Q1),
        "Q2" => Some(Quarter::Q2),
        "Q3" => Some(Quarter::Q3),
        "Q4" => Some(Quarter::Q4),
        _ => None,
    }
}
