//! Habit CRUD operations for the pipe interface.

use chrono::NaiveDate;
use uuid::Uuid;

use taskflow::app::Model;
use taskflow::domain::{Habit, HabitFrequency, HabitId};

use super::HandlerResult;
use crate::commands::pipe::types::{FilterParams, HabitInput, Operation, PipeError, PipeRequest};

/// Handle habit operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_habits(model, &request.filters),
        Operation::Get => get_habit(model, request.id.as_deref()),
        Operation::Create => create_habit(model, request.data.as_ref()),
        Operation::Update => update_habit(model, request.id.as_deref(), request.data.as_ref()),
        Operation::Delete => delete_habit(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

fn list_habits(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    let mut habits: Vec<&Habit> = model.habits.values().collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by search text
        if let Some(ref search) = f.search {
            let search_lower = search.to_lowercase();
            habits.retain(|h| {
                h.name.to_lowercase().contains(&search_lower)
                    || h.description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&search_lower))
            });
        }

        // Filter by tags
        if let Some(ref filter_tags) = f.tags {
            let mode = f.tags_mode.as_deref().unwrap_or("all");
            habits.retain(|h| {
                let habit_tags_lower: Vec<String> =
                    h.tags.iter().map(|s| s.to_lowercase()).collect();
                if mode == "any" {
                    filter_tags
                        .iter()
                        .any(|ft| habit_tags_lower.contains(&ft.to_lowercase()))
                } else {
                    filter_tags
                        .iter()
                        .all(|ft| habit_tags_lower.contains(&ft.to_lowercase()))
                }
            });
        }

        // Filter by archived status (default: exclude archived)
        if !f.include_completed.unwrap_or(false) {
            habits.retain(|h| !h.archived);
        }

        // Sort by name
        habits.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    let total = habits.len();
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let habits: Vec<Habit> = habits
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(serde_json::json!({
        "habits": habits,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_habit(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Habit ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let habit_id = HabitId(uuid);
    model
        .habits
        .get(&habit_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Habit not found: {id}")))
        .and_then(|h| serde_json::to_value(h).map_err(PipeError::serialization))
}

fn create_habit(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Habit data is required"))?;

    let input: HabitInput = serde_json::from_value(data.clone())
        .map_err(|e| PipeError::new("INVALID_DATA", format!("Failed to parse habit data: {e}")))?;

    let name = input
        .name
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Habit name is required"))?;

    let mut habit = Habit::new(&name);

    if let Some(ref desc) = input.description {
        habit.description = Some(desc.clone());
    }
    if let Some(ref freq_str) = input.frequency {
        if let Some(freq) = parse_frequency(freq_str) {
            habit.frequency = freq;
        }
    }
    if let Some(target) = input.target_count {
        // Target count stored in check-ins, not directly on habit
        // This is informational only for now
        let _ = target;
    }

    let habit_id = habit.id;
    model.habits.insert(habit_id, habit.clone());
    model.sync_habit(&habit);

    serde_json::to_value(&habit).map_err(PipeError::serialization)
}

fn update_habit(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Habit ID is required"))?;
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Habit data is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let habit_id = HabitId(uuid);

    let habit = model
        .habits
        .get_mut(&habit_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Habit not found: {id}")))?;

    if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
        habit.name = name.to_string();
    }
    if let Some(desc) = data.get("description") {
        habit.description = if desc.is_null() {
            None
        } else {
            desc.as_str().map(|s| s.to_string())
        };
    }
    if let Some(freq_str) = data.get("frequency").and_then(|v| v.as_str()) {
        if let Some(freq) = parse_frequency(freq_str) {
            habit.frequency = freq;
        }
    }
    if let Some(archived) = data.get("archived").and_then(|v| v.as_bool()) {
        habit.archived = archived;
    }
    if let Some(start) = data.get("start_date").and_then(|v| v.as_str()) {
        if let Ok(date) = NaiveDate::parse_from_str(start, "%Y-%m-%d") {
            habit.start_date = date;
        }
    }
    if let Some(end) = data.get("end_date") {
        habit.end_date = if end.is_null() {
            None
        } else {
            end.as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        };
    }
    if let Some(color) = data.get("color") {
        habit.color = if color.is_null() {
            None
        } else {
            color.as_str().map(|s| s.to_string())
        };
    }
    if let Some(tags) = data.get("tags").and_then(|v| v.as_array()) {
        habit.tags = tags
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }

    let updated_habit = habit.clone();
    model.sync_habit(&updated_habit);

    serde_json::to_value(&updated_habit).map_err(PipeError::serialization)
}

fn delete_habit(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Habit ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let habit_id = HabitId(uuid);

    model
        .habits
        .remove(&habit_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Habit not found: {id}")))?;

    model.delete_habit_from_storage(&habit_id);

    Ok(serde_json::json!({ "deleted": id }))
}

fn parse_frequency(s: &str) -> Option<HabitFrequency> {
    match s.to_lowercase().as_str() {
        "daily" => Some(HabitFrequency::Daily),
        "weekly" => Some(HabitFrequency::Weekly {
            days: vec![
                chrono::Weekday::Mon,
                chrono::Weekday::Wed,
                chrono::Weekday::Fri,
            ],
        }),
        _ => {
            // Try to parse "every_N_days" format
            if s.to_lowercase().starts_with("every_") && s.to_lowercase().ends_with("_days") {
                let n_str = s.to_lowercase().replace("every_", "").replace("_days", "");
                if let Ok(n) = n_str.parse::<u32>() {
                    return Some(HabitFrequency::EveryNDays { n });
                }
            }
            None
        }
    }
}
