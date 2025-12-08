//! ProjectRepository implementation for markdown backend.

use crate::domain::{Project, ProjectId};
use crate::storage::{ProjectRepository, StorageError, StorageResult};

use super::MarkdownBackend;

impl ProjectRepository for MarkdownBackend {
    fn create_project(&mut self, project: &Project) -> StorageResult<()> {
        if self.projects_cache.contains_key(&project.id) {
            return Err(StorageError::already_exists(
                "Project",
                project.id.to_string(),
            ));
        }
        self.write_project_file(project)?;
        self.projects_cache.insert(project.id, project.clone());
        Ok(())
    }

    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>> {
        Ok(self.projects_cache.get(id).cloned())
    }

    fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        if !self.projects_cache.contains_key(&project.id) {
            return Err(StorageError::not_found("Project", project.id.to_string()));
        }
        self.write_project_file(project)?;
        self.projects_cache.insert(project.id, project.clone());
        Ok(())
    }

    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()> {
        if !self.projects_cache.contains_key(id) {
            return Err(StorageError::not_found("Project", id.to_string()));
        }
        self.delete_project_file(id)?;
        self.projects_cache.remove(id);
        Ok(())
    }

    fn list_projects(&self) -> StorageResult<Vec<Project>> {
        Ok(self.projects_cache.values().cloned().collect())
    }

    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>> {
        Ok(self
            .projects_cache
            .values()
            .filter(|p| p.parent_id.as_ref() == Some(parent_id))
            .cloned()
            .collect())
    }
}
