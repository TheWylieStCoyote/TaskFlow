//! Project CRUD operations for the pipe interface.

use uuid::Uuid;

use taskflow::app::Model;
use taskflow::domain::{Project, ProjectId, ProjectStatus};

use super::HandlerResult;
use crate::commands::pipe::types::{FilterParams, Operation, PipeError, PipeRequest, ProjectInput};

/// Handle project operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_projects(model, &request.filters),
        Operation::Get => get_project(model, request.id.as_deref()),
        Operation::Create => create_project(model, request.data.as_ref()),
        Operation::Update => update_project(model, request.id.as_deref(), request.data.as_ref()),
        Operation::Delete => delete_project(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

fn list_projects(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    let mut projects: Vec<&Project> = model.projects.values().collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by search text
        if let Some(ref search) = f.search {
            let search_lower = search.to_lowercase();
            projects.retain(|p| {
                p.name.to_lowercase().contains(&search_lower)
                    || p.description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&search_lower))
            });
        }

        // Filter by status
        if let Some(ref statuses) = f.status {
            let status_set: Vec<ProjectStatus> = statuses
                .iter()
                .filter_map(|s| parse_project_status(s))
                .collect();
            if !status_set.is_empty() {
                projects.retain(|p| status_set.contains(&p.status));
            }
        }

        // Sort by name
        projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    let total = projects.len();
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let projects: Vec<Project> = projects
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(serde_json::json!({
        "projects": projects,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_project(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Project ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let project_id = ProjectId(uuid);
    model
        .projects
        .get(&project_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Project not found: {id}")))
        .and_then(|p| serde_json::to_value(p).map_err(PipeError::serialization))
}

fn create_project(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Project data is required"))?;

    let input: ProjectInput = serde_json::from_value(data.clone()).map_err(|e| {
        PipeError::new("INVALID_DATA", format!("Failed to parse project data: {e}"))
    })?;

    let name = input
        .name
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Project name is required"))?;

    let mut project = Project::new(&name);

    if let Some(ref desc) = input.description {
        project.description = Some(desc.clone());
    }
    if let Some(ref status_str) = input.status {
        if let Some(status) = parse_project_status(status_str) {
            project.status = status;
        }
    }
    if let Some(ref parent_id_str) = input.parent_id {
        if let Ok(uuid) = Uuid::parse_str(parent_id_str) {
            project.parent_id = Some(ProjectId(uuid));
        }
    }
    if let Some(ref color) = input.color {
        project.color = Some(color.clone());
    }

    let project_id = project.id;
    model.projects.insert(project_id, project.clone());
    model.sync_project(&project);

    serde_json::to_value(&project).map_err(PipeError::serialization)
}

fn update_project(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Project ID is required"))?;
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Project data is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let project_id = ProjectId(uuid);

    let project = model
        .projects
        .get_mut(&project_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Project not found: {id}")))?;

    if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
        project.name = name.to_string();
    }
    if let Some(desc) = data.get("description") {
        project.description = if desc.is_null() {
            None
        } else {
            desc.as_str().map(|s| s.to_string())
        };
    }
    if let Some(status_str) = data.get("status").and_then(|v| v.as_str()) {
        if let Some(status) = parse_project_status(status_str) {
            project.status = status;
        }
    }
    if let Some(parent) = data.get("parent_id") {
        project.parent_id = if parent.is_null() {
            None
        } else {
            parent
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok().map(ProjectId))
        };
    }
    if let Some(color) = data.get("color") {
        project.color = if color.is_null() {
            None
        } else {
            color.as_str().map(|s| s.to_string())
        };
    }

    project.updated_at = chrono::Utc::now();
    let updated_project = project.clone();
    model.sync_project(&updated_project);

    serde_json::to_value(&updated_project).map_err(PipeError::serialization)
}

fn delete_project(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Project ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let project_id = ProjectId(uuid);

    model
        .projects
        .remove(&project_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Project not found: {id}")))?;

    // Note: project deletion will be persisted on model.save()

    Ok(serde_json::json!({ "deleted": id }))
}

fn parse_project_status(s: &str) -> Option<ProjectStatus> {
    match s.to_lowercase().replace(['-', '_'], "").as_str() {
        "active" => Some(ProjectStatus::Active),
        "onhold" | "hold" => Some(ProjectStatus::OnHold),
        "completed" | "done" => Some(ProjectStatus::Completed),
        "archived" => Some(ProjectStatus::Archived),
        _ => None,
    }
}
