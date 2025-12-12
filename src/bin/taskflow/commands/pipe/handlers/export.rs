//! Export handler for the pipe interface.
//!
//! Exports all data from the model in a format suitable for backup/restore.

use std::collections::HashSet;

use taskflow::app::Model;
use taskflow::domain::Tag;
use taskflow::storage::ExportData;

use super::HandlerResult;
use crate::commands::pipe::types::{PipeError, PipeRequest};

/// Handle export operation.
pub fn handle_export(model: &Model, _request: &PipeRequest) -> HandlerResult {
    // Collect unique tags from tasks
    let tag_names: HashSet<String> = model
        .tasks
        .values()
        .flat_map(|t| t.tags.iter())
        .cloned()
        .collect();

    let tags: Vec<Tag> = tag_names.into_iter().map(Tag::new).collect();

    let export_data = ExportData {
        tasks: model.tasks.values().cloned().collect(),
        projects: model.projects.values().cloned().collect(),
        tags,
        time_entries: model.time_entries.values().cloned().collect(),
        work_logs: model.work_logs.values().cloned().collect(),
        habits: model.habits.values().cloned().collect(),
        goals: model.goals.values().cloned().collect(),
        key_results: model.key_results.values().cloned().collect(),
        version: 1,
        pomodoro_session: model.pomodoro.session.clone(),
        pomodoro_config: Some(model.pomodoro.config.clone()),
        pomodoro_stats: Some(model.pomodoro.stats.clone()),
        saved_filters: model.saved_filters.values().cloned().collect(),
    };

    serde_json::to_value(&export_data).map_err(PipeError::serialization)
}
