use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::{Filter, Project, ProjectId, SortSpec, Task, TaskId, TimeEntry, TimeEntryId};
#[allow(unused_imports)]
use crate::storage::{self, BackendType, ProjectRepository, StorageBackend, TaskRepository};
use crate::ui::InputMode;

use super::ViewId;

/// Application running state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunningState {
    #[default]
    Running,
    Quitting,
}

/// The complete application state (Model in TEA)
pub struct Model {
    // Running state
    pub running: RunningState,

    // Data
    pub tasks: HashMap<TaskId, Task>,
    pub projects: HashMap<ProjectId, Project>,
    pub time_entries: HashMap<TimeEntryId, TimeEntry>,
    pub active_time_entry: Option<TimeEntryId>,

    // Navigation
    pub current_view: ViewId,
    pub selected_index: usize,

    // Visible items (filtered and sorted)
    pub visible_tasks: Vec<TaskId>,

    // Filter/Sort
    pub filter: Filter,
    pub sort: SortSpec,
    pub show_completed: bool,

    // UI state
    pub show_sidebar: bool,
    pub show_help: bool,
    pub terminal_size: (u16, u16),

    // Input state
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub show_confirm_delete: bool,

    // Storage
    storage: Option<Box<dyn StorageBackend>>,
    pub data_path: Option<PathBuf>,
    pub dirty: bool,
}

impl Model {
    pub fn new() -> Self {
        Self {
            running: RunningState::default(),
            tasks: HashMap::new(),
            projects: HashMap::new(),
            time_entries: HashMap::new(),
            active_time_entry: None,
            current_view: ViewId::default(),
            selected_index: 0,
            visible_tasks: Vec::new(),
            filter: Filter::default(),
            sort: SortSpec::default(),
            show_completed: false,
            show_sidebar: true,
            show_help: false,
            terminal_size: (80, 24),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            show_confirm_delete: false,
            storage: None,
            data_path: None,
            dirty: false,
        }
    }

    /// Load data from a storage backend
    pub fn with_storage(mut self, backend_type: BackendType, path: PathBuf) -> anyhow::Result<Self> {
        let mut backend = storage::create_backend(backend_type, &path)?;

        // Load tasks from storage
        let tasks = backend.list_tasks()?;
        for task in tasks {
            self.tasks.insert(task.id.clone(), task);
        }

        // Load projects from storage
        let projects = storage::ProjectRepository::list_projects(backend.as_mut())?;
        for project in projects {
            self.projects.insert(project.id.clone(), project);
        }

        self.storage = Some(backend);
        self.data_path = Some(path);
        self.refresh_visible_tasks();

        Ok(self)
    }

    /// Save current state to storage
    pub fn save(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut backend) = self.storage {
            backend.flush()?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Sync a task change to storage
    pub fn sync_task(&mut self, task: &Task) {
        if let Some(ref mut backend) = self.storage {
            // Try update first, if not found, create
            if backend.update_task(task).is_err() {
                let _ = backend.create_task(task);
            }
            self.dirty = true;
        }
    }

    /// Delete a task from storage
    pub fn delete_task_from_storage(&mut self, id: &TaskId) {
        if let Some(ref mut backend) = self.storage {
            let _ = backend.delete_task(id);
            self.dirty = true;
        }
    }

    /// Add sample tasks for development
    pub fn with_sample_data(mut self) -> Self {
        use crate::domain::{Priority, TaskStatus};
        use chrono::NaiveDate;

        let tasks = vec![
            Task::new("Set up project structure")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High),
            Task::new("Implement domain types")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High),
            Task::new("Create TEA architecture")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High),
            Task::new("Build task list UI")
                .with_priority(Priority::Medium),
            Task::new("Add storage backends")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Medium),
            Task::new("Implement keybinding config")
                .with_priority(Priority::Low),
            Task::new("Add theme support")
                .with_priority(Priority::Low),
            Task::new("Write documentation")
                .with_priority(Priority::None),
            Task::new("Review and fix bugs")
                .with_due_date(NaiveDate::from_ymd_opt(2025, 12, 10).unwrap())
                .with_priority(Priority::Urgent),
        ];

        for task in tasks {
            self.tasks.insert(task.id.clone(), task);
        }

        self.refresh_visible_tasks();
        self
    }

    /// Recalculate visible tasks based on filter and sort
    pub fn refresh_visible_tasks(&mut self) {
        let mut tasks: Vec<_> = self
            .tasks
            .values()
            .filter(|task| self.task_matches_filter(task))
            .collect();

        // Sort by priority (descending) then by created_at
        tasks.sort_by(|a, b| {
            let priority_order = |p: &crate::domain::Priority| match p {
                crate::domain::Priority::Urgent => 0,
                crate::domain::Priority::High => 1,
                crate::domain::Priority::Medium => 2,
                crate::domain::Priority::Low => 3,
                crate::domain::Priority::None => 4,
            };
            priority_order(&a.priority)
                .cmp(&priority_order(&b.priority))
                .then_with(|| b.created_at.cmp(&a.created_at))
        });

        self.visible_tasks = tasks.into_iter().map(|t| t.id.clone()).collect();

        // Adjust selection if needed
        if self.selected_index >= self.visible_tasks.len() && !self.visible_tasks.is_empty() {
            self.selected_index = self.visible_tasks.len() - 1;
        }
    }

    fn task_matches_filter(&self, task: &Task) -> bool {
        // Filter out completed tasks unless show_completed is true
        if !self.show_completed && task.status.is_complete() {
            return false;
        }

        // Add more filter logic as needed
        true
    }

    /// Get the currently selected task
    pub fn selected_task(&self) -> Option<&Task> {
        self.visible_tasks
            .get(self.selected_index)
            .and_then(|id| self.tasks.get(id))
    }

    /// Get the currently selected task mutably
    pub fn selected_task_mut(&mut self) -> Option<&mut Task> {
        let id = self.visible_tasks.get(self.selected_index)?.clone();
        self.tasks.get_mut(&id)
    }

    /// Check if storage is configured
    pub fn has_storage(&self) -> bool {
        self.storage.is_some()
    }

    /// Start time tracking for a task
    pub fn start_time_tracking(&mut self, task_id: TaskId) {
        // Stop any currently running timer
        self.stop_time_tracking();

        // Start new timer
        let entry = TimeEntry::start(task_id);
        let entry_id = entry.id.clone();
        self.time_entries.insert(entry_id.clone(), entry);
        self.active_time_entry = Some(entry_id);
        self.dirty = true;
    }

    /// Stop the currently active time tracking
    pub fn stop_time_tracking(&mut self) {
        if let Some(ref entry_id) = self.active_time_entry.clone() {
            if let Some(entry) = self.time_entries.get_mut(entry_id) {
                entry.stop();
                self.dirty = true;
            }
            self.active_time_entry = None;
        }
    }

    /// Get the active time entry
    pub fn active_time_entry(&self) -> Option<&TimeEntry> {
        self.active_time_entry
            .as_ref()
            .and_then(|id| self.time_entries.get(id))
    }

    /// Check if time is being tracked for a specific task
    pub fn is_tracking_task(&self, task_id: &TaskId) -> bool {
        self.active_time_entry()
            .map(|e| &e.task_id == task_id)
            .unwrap_or(false)
    }

    /// Get total time tracked for a task
    pub fn total_time_for_task(&self, task_id: &TaskId) -> u32 {
        self.time_entries
            .values()
            .filter(|e| &e.task_id == task_id)
            .map(|e| e.calculated_duration_minutes())
            .sum()
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}
