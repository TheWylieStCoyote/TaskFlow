//! Import handler for the pipe interface.
//!
//! Imports data from an export, merging with existing data.

#![allow(clippy::map_entry)] // We need to sync after inserting, can't use entry API

use taskflow::app::Model;
use taskflow::storage::ExportData;

use super::HandlerResult;
use crate::commands::pipe::types::{PipeError, PipeRequest};

/// Handle import operation.
pub fn handle_import(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    let data = request
        .data
        .as_ref()
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Import data is required"))?;

    let export_data: ExportData = serde_json::from_value(data.clone())
        .map_err(|e| PipeError::new("INVALID_DATA", format!("Failed to parse import data: {e}")))?;

    let mut imported = ImportStats::default();

    // Import tasks
    for task in export_data.tasks {
        if !model.tasks.contains_key(&task.id) {
            model.tasks.insert(task.id, task.clone());
            model.sync_task(&task);
            imported.tasks += 1;
        }
    }

    // Import projects
    for project in export_data.projects {
        if !model.projects.contains_key(&project.id) {
            model.projects.insert(project.id, project.clone());
            model.sync_project(&project);
            imported.projects += 1;
        }
    }

    // Import time entries
    for entry in export_data.time_entries {
        if !model.time_entries.contains_key(&entry.id) {
            model.time_entries.insert(entry.id, entry.clone());
            model.sync_time_entry(&entry);
            imported.time_entries += 1;
        }
    }

    // Import work logs
    for log in export_data.work_logs {
        if !model.work_logs.contains_key(&log.id) {
            model.work_logs.insert(log.id, log.clone());
            model.sync_work_log(&log);
            imported.work_logs += 1;
        }
    }

    // Import habits
    for habit in export_data.habits {
        if !model.habits.contains_key(&habit.id) {
            model.habits.insert(habit.id, habit.clone());
            model.sync_habit(&habit);
            imported.habits += 1;
        }
    }

    // Import goals
    for goal in export_data.goals {
        if !model.goals.contains_key(&goal.id) {
            model.goals.insert(goal.id, goal.clone());
            model.sync_goal(&goal);
            imported.goals += 1;
        }
    }

    // Import key results
    for kr in export_data.key_results {
        if !model.key_results.contains_key(&kr.id) {
            model.key_results.insert(kr.id, kr.clone());
            model.sync_key_result(&kr);
            imported.key_results += 1;
        }
    }

    // Import saved filters (no sync method, just add to model)
    for filter in export_data.saved_filters {
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(e) = model.saved_filters.entry(filter.id.clone()) {
            e.insert(filter);
            imported.saved_filters += 1;
        }
    }

    Ok(serde_json::json!({
        "success": true,
        "imported": {
            "tasks": imported.tasks,
            "projects": imported.projects,
            "time_entries": imported.time_entries,
            "work_logs": imported.work_logs,
            "habits": imported.habits,
            "goals": imported.goals,
            "key_results": imported.key_results,
            "saved_filters": imported.saved_filters,
        }
    }))
}

#[derive(Default)]
struct ImportStats {
    tasks: usize,
    projects: usize,
    time_entries: usize,
    work_logs: usize,
    habits: usize,
    goals: usize,
    key_results: usize,
    saved_filters: usize,
}
