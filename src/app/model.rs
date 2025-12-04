use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::{
    Filter, Priority, Project, ProjectId, SortSpec, Task, TaskId, TimeEntry, TimeEntryId,
};
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

    // Configuration
    pub default_priority: Priority,
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
            default_priority: Priority::default(),
        }
    }

    /// Load data from a storage backend
    pub fn with_storage(
        mut self,
        backend_type: BackendType,
        path: PathBuf,
    ) -> anyhow::Result<Self> {
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
            Task::new("Build task list UI").with_priority(Priority::Medium),
            Task::new("Add storage backends")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Medium),
            Task::new("Implement keybinding config").with_priority(Priority::Low),
            Task::new("Add theme support").with_priority(Priority::Low),
            Task::new("Write documentation").with_priority(Priority::None),
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

        // Filter by current view
        match self.current_view {
            ViewId::TaskList => true, // Show all tasks
            ViewId::Today => {
                // Show tasks due today
                task.due_date
                    .map(|d| d == chrono::Utc::now().date_naive())
                    .unwrap_or(false)
            }
            ViewId::Upcoming => {
                // Show tasks with future due dates
                task.due_date
                    .map(|d| d > chrono::Utc::now().date_naive())
                    .unwrap_or(false)
            }
            ViewId::Projects => {
                // Show tasks that belong to a project
                task.project_id.is_some()
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, TaskStatus};

    #[test]
    fn test_model_new_defaults() {
        let model = Model::new();

        assert_eq!(model.running, RunningState::Running);
        assert!(model.tasks.is_empty());
        assert!(model.projects.is_empty());
        assert!(model.time_entries.is_empty());
        assert!(model.active_time_entry.is_none());
        assert_eq!(model.selected_index, 0);
        assert!(model.visible_tasks.is_empty());
        assert!(!model.show_completed);
        assert!(model.show_sidebar);
        assert!(!model.show_help);
        assert_eq!(model.input_mode, InputMode::Normal);
        assert!(model.input_buffer.is_empty());
        assert!(!model.dirty);
    }

    #[test]
    fn test_model_with_sample_data() {
        let model = Model::new().with_sample_data();

        // Sample data creates 9 tasks
        assert_eq!(model.tasks.len(), 9);
        // Some are completed, so visible should be less
        assert!(model.visible_tasks.len() < 9);
    }

    #[test]
    fn test_model_refresh_visible_tasks_sorts_by_priority() {
        let mut model = Model::new();

        // Add tasks with different priorities
        let urgent = Task::new("Urgent").with_priority(Priority::Urgent);
        let low = Task::new("Low").with_priority(Priority::Low);
        let high = Task::new("High").with_priority(Priority::High);

        model.tasks.insert(low.id.clone(), low.clone());
        model.tasks.insert(urgent.id.clone(), urgent.clone());
        model.tasks.insert(high.id.clone(), high.clone());

        model.refresh_visible_tasks();

        // Order should be: Urgent, High, Low
        assert_eq!(model.visible_tasks.len(), 3);
        assert_eq!(model.visible_tasks[0], urgent.id);
        assert_eq!(model.visible_tasks[1], high.id);
        assert_eq!(model.visible_tasks[2], low.id);
    }

    #[test]
    fn test_model_refresh_visible_tasks_hides_completed() {
        let mut model = Model::new();
        model.show_completed = false;

        let todo = Task::new("Todo");
        let done = Task::new("Done").with_status(TaskStatus::Done);
        let cancelled = Task::new("Cancelled").with_status(TaskStatus::Cancelled);

        model.tasks.insert(todo.id.clone(), todo);
        model.tasks.insert(done.id.clone(), done);
        model.tasks.insert(cancelled.id.clone(), cancelled);

        model.refresh_visible_tasks();

        // Only non-completed tasks should be visible
        assert_eq!(model.visible_tasks.len(), 1);
    }

    #[test]
    fn test_model_refresh_visible_tasks_shows_completed() {
        let mut model = Model::new();
        model.show_completed = true;

        let todo = Task::new("Todo");
        let done = Task::new("Done").with_status(TaskStatus::Done);
        let cancelled = Task::new("Cancelled").with_status(TaskStatus::Cancelled);

        model.tasks.insert(todo.id.clone(), todo);
        model.tasks.insert(done.id.clone(), done);
        model.tasks.insert(cancelled.id.clone(), cancelled);

        model.refresh_visible_tasks();

        // All tasks should be visible
        assert_eq!(model.visible_tasks.len(), 3);
    }

    #[test]
    fn test_model_selected_task_returns_correct() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");

        model.tasks.insert(task1.id.clone(), task1.clone());
        model.tasks.insert(task2.id.clone(), task2.clone());
        model.refresh_visible_tasks();

        // Select first task
        model.selected_index = 0;
        let selected = model.selected_task().unwrap();
        assert_eq!(selected.id, model.visible_tasks[0]);

        // Select second task
        model.selected_index = 1;
        let selected = model.selected_task().unwrap();
        assert_eq!(selected.id, model.visible_tasks[1]);
    }

    #[test]
    fn test_model_selected_task_empty_list() {
        let model = Model::new();

        assert!(model.selected_task().is_none());
    }

    #[test]
    fn test_model_selected_index_adjustment() {
        let mut model = Model::new();

        // Add 3 tasks
        for i in 0..3 {
            let task = Task::new(format!("Task {}", i));
            model.tasks.insert(task.id.clone(), task);
        }
        model.refresh_visible_tasks();

        // Select last item
        model.selected_index = 2;

        // Remove all tasks except one
        let ids: Vec<_> = model.tasks.keys().skip(1).cloned().collect();
        for id in ids {
            model.tasks.remove(&id);
        }

        model.refresh_visible_tasks();

        // Selection should be adjusted to valid range
        assert!(model.selected_index < model.visible_tasks.len());
    }

    #[test]
    fn test_model_start_time_tracking() {
        let mut model = Model::new();

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        model.start_time_tracking(task.id.clone());

        assert!(model.active_time_entry.is_some());
        assert!(model.time_entries.len() == 1);
        assert!(model.dirty);

        let entry = model.active_time_entry().unwrap();
        assert_eq!(entry.task_id, task.id);
        assert!(entry.is_running());
    }

    #[test]
    fn test_model_stop_time_tracking() {
        let mut model = Model::new();

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        model.start_time_tracking(task.id.clone());
        model.stop_time_tracking();

        assert!(model.active_time_entry.is_none());

        // Entry should still exist but be stopped
        let entry = model.time_entries.values().next().unwrap();
        assert!(!entry.is_running());
    }

    #[test]
    fn test_model_start_stops_previous() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        model.tasks.insert(task1.id.clone(), task1.clone());
        model.tasks.insert(task2.id.clone(), task2.clone());

        // Start tracking task1
        model.start_time_tracking(task1.id.clone());
        let first_entry_id = model.active_time_entry.clone().unwrap();

        // Start tracking task2 (should stop task1)
        model.start_time_tracking(task2.id.clone());

        // Two entries total
        assert_eq!(model.time_entries.len(), 2);

        // First entry should be stopped
        let first_entry = model.time_entries.get(&first_entry_id).unwrap();
        assert!(!first_entry.is_running());

        // Active entry should be for task2
        let active = model.active_time_entry().unwrap();
        assert_eq!(active.task_id, task2.id);
    }

    #[test]
    fn test_model_is_tracking_task() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        model.tasks.insert(task1.id.clone(), task1.clone());
        model.tasks.insert(task2.id.clone(), task2.clone());

        // Not tracking anything initially
        assert!(!model.is_tracking_task(&task1.id));
        assert!(!model.is_tracking_task(&task2.id));

        // Start tracking task1
        model.start_time_tracking(task1.id.clone());

        assert!(model.is_tracking_task(&task1.id));
        assert!(!model.is_tracking_task(&task2.id));
    }

    #[test]
    fn test_model_total_time_for_task() {
        let mut model = Model::new();

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        // Add multiple completed time entries
        let mut entry1 = TimeEntry::start(task.id.clone());
        entry1.duration_minutes = Some(30);
        entry1.ended_at = Some(chrono::Utc::now());

        let mut entry2 = TimeEntry::start(task.id.clone());
        entry2.duration_minutes = Some(45);
        entry2.ended_at = Some(chrono::Utc::now());

        model.time_entries.insert(entry1.id.clone(), entry1);
        model.time_entries.insert(entry2.id.clone(), entry2);

        let total = model.total_time_for_task(&task.id);
        assert_eq!(total, 75); // 30 + 45
    }

    #[test]
    fn test_model_dirty_flag() {
        let mut model = Model::new();
        assert!(!model.dirty);

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        model.start_time_tracking(task.id.clone());
        assert!(model.dirty);
    }

    #[test]
    fn test_model_has_storage() {
        let model = Model::new();
        assert!(!model.has_storage());
    }

    #[test]
    fn test_running_state_default() {
        let state = RunningState::default();
        assert_eq!(state, RunningState::Running);
    }

    #[test]
    fn test_view_tasklist_shows_all() {
        let mut model = Model::new();
        model.current_view = ViewId::TaskList;

        // Create tasks with various due dates and project associations
        let task_no_date = Task::new("No due date");
        let task_with_date = Task::new("Has date")
            .with_due_date(chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap());
        let task_with_project =
            Task::new("Has project").with_project(crate::domain::ProjectId::new());

        model.tasks.insert(task_no_date.id.clone(), task_no_date);
        model
            .tasks
            .insert(task_with_date.id.clone(), task_with_date);
        model
            .tasks
            .insert(task_with_project.id.clone(), task_with_project);

        model.refresh_visible_tasks();

        // TaskList view should show all tasks
        assert_eq!(model.visible_tasks.len(), 3);
    }

    #[test]
    fn test_view_today_filters_due_today() {
        let mut model = Model::new();
        model.current_view = ViewId::Today;

        let today = chrono::Utc::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);

        let task_today = Task::new("Due today").with_due_date(today);
        let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
        let task_no_date = Task::new("No due date");

        model
            .tasks
            .insert(task_today.id.clone(), task_today.clone());
        model.tasks.insert(task_tomorrow.id.clone(), task_tomorrow);
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Only today's task should be visible
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_today.id);
    }

    #[test]
    fn test_view_upcoming_filters_future() {
        let mut model = Model::new();
        model.current_view = ViewId::Upcoming;

        let today = chrono::Utc::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);
        let next_week = today + chrono::Duration::days(7);

        let task_today = Task::new("Due today").with_due_date(today);
        let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
        let task_next_week = Task::new("Due next week").with_due_date(next_week);
        let task_no_date = Task::new("No due date");

        model.tasks.insert(task_today.id.clone(), task_today);
        model
            .tasks
            .insert(task_tomorrow.id.clone(), task_tomorrow.clone());
        model
            .tasks
            .insert(task_next_week.id.clone(), task_next_week.clone());
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Only future tasks should be visible (not today, not tasks without dates)
        assert_eq!(model.visible_tasks.len(), 2);
        assert!(model.visible_tasks.contains(&task_tomorrow.id));
        assert!(model.visible_tasks.contains(&task_next_week.id));
    }

    #[test]
    fn test_view_projects_filters_with_project() {
        let mut model = Model::new();
        model.current_view = ViewId::Projects;

        let project_id = crate::domain::ProjectId::new();

        let task_with_project = Task::new("Has project").with_project(project_id);
        let task_no_project = Task::new("No project");

        model
            .tasks
            .insert(task_with_project.id.clone(), task_with_project.clone());
        model
            .tasks
            .insert(task_no_project.id.clone(), task_no_project);

        model.refresh_visible_tasks();

        // Only tasks with projects should be visible
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_with_project.id);
    }
}
