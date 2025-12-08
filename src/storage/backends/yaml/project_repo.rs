//! ProjectRepository implementation for YAML backend.

use crate::domain::{Project, ProjectId};
use crate::storage::{ProjectRepository, StorageError, StorageResult};

use super::YamlBackend;

impl ProjectRepository for YamlBackend {
    fn create_project(&mut self, project: &Project) -> StorageResult<()> {
        if self.data.projects.iter().any(|p| p.id == project.id) {
            return Err(StorageError::already_exists(
                "Project",
                project.id.to_string(),
            ));
        }
        self.data.projects.push(project.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>> {
        Ok(self.data.projects.iter().find(|p| &p.id == id).cloned())
    }

    fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        if let Some(existing) = self.data.projects.iter_mut().find(|p| p.id == project.id) {
            *existing = project.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("Project", project.id.to_string()))
        }
    }

    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()> {
        let len_before = self.data.projects.len();
        self.data.projects.retain(|p| &p.id != id);
        if self.data.projects.len() == len_before {
            return Err(StorageError::not_found("Project", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_projects(&self) -> StorageResult<Vec<Project>> {
        Ok(self.data.projects.clone())
    }

    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>> {
        Ok(self
            .data
            .projects
            .iter()
            .filter(|p| p.parent_id.as_ref() == Some(parent_id))
            .cloned()
            .collect())
    }
}
