//! Tag operations for the pipe interface.
//!
//! Tags are derived from tasks and habits rather than being standalone entities.
//! This handler provides read-only access to the tags used across the system.

use std::collections::HashMap;

use taskflow::app::Model;

use super::HandlerResult;
use crate::commands::pipe::types::{FilterParams, Operation, PipeError, PipeRequest};

/// Handle tag operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_tags(model, &request.filters),
        Operation::Get => get_tag(model, request.id.as_deref()),
        Operation::Create | Operation::Update | Operation::Delete => Err(PipeError::new(
            "INVALID_OPERATION",
            "Tags are derived from tasks and habits. Use task/habit operations to manage tags.",
        )),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

/// Tag information with usage counts.
#[derive(serde::Serialize)]
struct TagInfo {
    name: String,
    task_count: usize,
    habit_count: usize,
    total_count: usize,
}

fn list_tags(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    // Collect all tags with counts
    let mut tag_counts: HashMap<String, (usize, usize)> = HashMap::new();

    // Count tags from tasks
    for task in model.tasks.values() {
        for tag in &task.tags {
            let lower_tag = tag.to_lowercase();
            let entry = tag_counts.entry(lower_tag).or_insert((0, 0));
            entry.0 += 1;
        }
    }

    // Count tags from habits
    for habit in model.habits.values() {
        for tag in &habit.tags {
            let lower_tag = tag.to_lowercase();
            let entry = tag_counts.entry(lower_tag).or_insert((0, 0));
            entry.1 += 1;
        }
    }

    let mut tags: Vec<TagInfo> = tag_counts
        .into_iter()
        .map(|(name, (task_count, habit_count))| TagInfo {
            name,
            task_count,
            habit_count,
            total_count: task_count + habit_count,
        })
        .collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by search text
        if let Some(ref search) = f.search {
            let search_lower = search.to_lowercase();
            tags.retain(|t| t.name.contains(&search_lower));
        }

        // Sort by name or count
        if let Some(ref sort_by) = f.sort_by {
            let desc = f.sort_order.as_deref() == Some("desc");
            match sort_by.as_str() {
                "count" | "total_count" => {
                    tags.sort_by(|a, b| {
                        let cmp = a.total_count.cmp(&b.total_count);
                        if desc {
                            cmp.reverse()
                        } else {
                            cmp
                        }
                    });
                }
                _ => {
                    tags.sort_by(|a, b| {
                        let cmp = a.name.cmp(&b.name);
                        if desc {
                            cmp.reverse()
                        } else {
                            cmp
                        }
                    });
                }
            }
        } else {
            // Default: sort by count descending
            tags.sort_by(|a, b| b.total_count.cmp(&a.total_count));
        }
    } else {
        // Default: sort by count descending
        tags.sort_by(|a, b| b.total_count.cmp(&a.total_count));
    }

    let total = tags.len();
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let tags: Vec<TagInfo> = tags.into_iter().skip(offset).take(limit).collect();

    Ok(serde_json::json!({
        "tags": tags,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_tag(model: &Model, name: Option<&str>) -> HandlerResult {
    let name = name.ok_or_else(|| PipeError::new("MISSING_ID", "Tag name is required"))?;
    let name_lower = name.to_lowercase();

    // Count occurrences
    let mut task_count = 0;
    let mut habit_count = 0;
    let mut task_ids: Vec<String> = Vec::new();
    let mut habit_ids: Vec<String> = Vec::new();

    for task in model.tasks.values() {
        if task.tags.iter().any(|t| t.to_lowercase() == name_lower) {
            task_count += 1;
            task_ids.push(task.id.0.to_string());
        }
    }

    for habit in model.habits.values() {
        if habit.tags.iter().any(|t| t.to_lowercase() == name_lower) {
            habit_count += 1;
            habit_ids.push(habit.id.0.to_string());
        }
    }

    if task_count == 0 && habit_count == 0 {
        return Err(PipeError::new(
            "NOT_FOUND",
            format!("Tag not found: {name}"),
        ));
    }

    Ok(serde_json::json!({
        "name": name_lower,
        "task_count": task_count,
        "habit_count": habit_count,
        "total_count": task_count + habit_count,
        "task_ids": task_ids,
        "habit_ids": habit_ids,
    }))
}
