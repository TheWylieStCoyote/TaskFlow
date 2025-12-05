use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::{
    Filter, Priority, Project, ProjectId, SortSpec, Task, TaskId, TimeEntry, TimeEntryId,
};
#[allow(unused_imports)]
use crate::storage::{self, BackendType, ProjectRepository, StorageBackend, TaskRepository};
use crate::ui::{InputMode, InputTarget};

use super::{FocusPane, UndoStack, ViewId};

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
    pub focus_pane: FocusPane,
    pub sidebar_selected: usize,
    pub selected_project: Option<ProjectId>,

    // Input state
    pub input_mode: InputMode,
    pub input_target: InputTarget,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub show_confirm_delete: bool,

    // Storage
    storage: Option<Box<dyn StorageBackend>>,
    pub data_path: Option<PathBuf>,
    pub dirty: bool,

    // Configuration
    pub default_priority: Priority,

    // Undo history
    pub undo_stack: UndoStack,
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
            focus_pane: FocusPane::default(),
            sidebar_selected: 0,
            selected_project: None,
            input_mode: InputMode::Normal,
            input_target: InputTarget::default(),
            input_buffer: String::new(),
            cursor_position: 0,
            show_confirm_delete: false,
            storage: None,
            data_path: None,
            dirty: false,
            default_priority: Priority::default(),
            undo_stack: UndoStack::new(),
        }
    }

    /// Get the number of sidebar items (views + separator + projects header + projects)
    pub fn sidebar_item_count(&self) -> usize {
        // 4 views (All Tasks, Today, Upcoming, Overdue) + 1 separator + 1 "Projects" header + projects count
        6 + self.projects.len().max(1) // At least 1 for "No projects" placeholder
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

    /// Sync a project to storage (create or update)
    pub fn sync_project(&mut self, project: &Project) {
        if let Some(ref mut backend) = self.storage {
            // Try update first, if not found, create
            if backend.update_project(project).is_err() {
                let _ = backend.create_project(project);
            }
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
        use crate::domain::{SortField, SortOrder};

        let mut tasks: Vec<_> = self
            .tasks
            .values()
            .filter(|task| self.task_matches_filter(task))
            .collect();

        // Sort based on SortSpec
        let sort_field = self.sort.field;
        let sort_order = self.sort.order;

        tasks.sort_by(|a, b| {
            let cmp = match sort_field {
                SortField::CreatedAt => a.created_at.cmp(&b.created_at),
                SortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                SortField::DueDate => {
                    // Tasks with no due date go last
                    match (a.due_date, b.due_date) {
                        (Some(da), Some(db)) => da.cmp(&db),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                SortField::Priority => {
                    let priority_order = |p: &crate::domain::Priority| match p {
                        crate::domain::Priority::Urgent => 0,
                        crate::domain::Priority::High => 1,
                        crate::domain::Priority::Medium => 2,
                        crate::domain::Priority::Low => 3,
                        crate::domain::Priority::None => 4,
                    };
                    priority_order(&a.priority).cmp(&priority_order(&b.priority))
                }
                SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                SortField::Status => {
                    let status_order = |s: &crate::domain::TaskStatus| match s {
                        crate::domain::TaskStatus::InProgress => 0,
                        crate::domain::TaskStatus::Todo => 1,
                        crate::domain::TaskStatus::Blocked => 2,
                        crate::domain::TaskStatus::Done => 3,
                        crate::domain::TaskStatus::Cancelled => 4,
                    };
                    status_order(&a.status).cmp(&status_order(&b.status))
                }
            };

            match sort_order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
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

        // Filter by search text (case-insensitive, matches title or tags)
        if let Some(ref search) = self.filter.search_text {
            let search_lower = search.to_lowercase();
            let title_matches = task.title.to_lowercase().contains(&search_lower);
            let tags_match = task
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&search_lower));
            if !title_matches && !tags_match {
                return false;
            }
        }

        // Filter by tags (if set)
        if let Some(ref filter_tags) = self.filter.tags {
            use crate::domain::TagFilterMode;
            let has_tags = match self.filter.tags_mode {
                TagFilterMode::Any => {
                    // Task must have at least one of the filter tags
                    filter_tags.iter().any(|ft| {
                        task.tags
                            .iter()
                            .any(|t| t.to_lowercase() == ft.to_lowercase())
                    })
                }
                TagFilterMode::All => {
                    // Task must have all of the filter tags
                    filter_tags.iter().all(|ft| {
                        task.tags
                            .iter()
                            .any(|t| t.to_lowercase() == ft.to_lowercase())
                    })
                }
            };
            if !has_tags {
                return false;
            }
        }

        // Filter by selected project if any
        if let Some(ref project_id) = self.selected_project {
            if task.project_id.as_ref() != Some(project_id) {
                return false;
            }
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
            ViewId::Overdue => {
                // Show tasks with past due dates (before today)
                task.due_date
                    .map(|d| d < chrono::Utc::now().date_naive())
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
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        // Add tasks with different priorities
        let urgent = Task::new("Urgent").with_priority(Priority::Urgent);
        let low = Task::new("Low").with_priority(Priority::Low);
        let high = Task::new("High").with_priority(Priority::High);

        model.tasks.insert(low.id.clone(), low.clone());
        model.tasks.insert(urgent.id.clone(), urgent.clone());
        model.tasks.insert(high.id.clone(), high.clone());

        // Set sort to priority (default is CreatedAt)
        model.sort = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Ascending,
        };
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

    #[test]
    fn test_view_overdue_filters_past_due() {
        let mut model = Model::new();
        model.current_view = ViewId::Overdue;

        let today = chrono::Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let last_week = today - chrono::Duration::days(7);
        let tomorrow = today + chrono::Duration::days(1);

        let task_yesterday = Task::new("Due yesterday").with_due_date(yesterday);
        let task_last_week = Task::new("Due last week").with_due_date(last_week);
        let task_today = Task::new("Due today").with_due_date(today);
        let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
        let task_no_date = Task::new("No due date");

        model
            .tasks
            .insert(task_yesterday.id.clone(), task_yesterday.clone());
        model
            .tasks
            .insert(task_last_week.id.clone(), task_last_week.clone());
        model.tasks.insert(task_today.id.clone(), task_today);
        model.tasks.insert(task_tomorrow.id.clone(), task_tomorrow);
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Only overdue tasks (past due dates) should be visible
        assert_eq!(model.visible_tasks.len(), 2);
        assert!(model.visible_tasks.contains(&task_yesterday.id));
        assert!(model.visible_tasks.contains(&task_last_week.id));
    }

    #[test]
    fn test_view_overdue_excludes_today() {
        let mut model = Model::new();
        model.current_view = ViewId::Overdue;

        let today = chrono::Utc::now().date_naive();
        let task_today = Task::new("Due today").with_due_date(today);

        model.tasks.insert(task_today.id.clone(), task_today);

        model.refresh_visible_tasks();

        // Today's tasks are not overdue
        assert!(model.visible_tasks.is_empty());
    }

    #[test]
    fn test_view_overdue_excludes_no_due_date() {
        let mut model = Model::new();
        model.current_view = ViewId::Overdue;

        let task_no_date = Task::new("No due date");
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Tasks without due dates are not overdue
        assert!(model.visible_tasks.is_empty());
    }

    #[test]
    fn test_search_filter_matches_title() {
        let mut model = Model::new();

        let task_match = Task::new("Build the feature");
        let task_no_match = Task::new("Fix the bug");

        model
            .tasks
            .insert(task_match.id.clone(), task_match.clone());
        model.tasks.insert(task_no_match.id.clone(), task_no_match);

        model.filter.search_text = Some("build".to_string());
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_match.id);
    }

    #[test]
    fn test_search_filter_case_insensitive() {
        let mut model = Model::new();

        let task = Task::new("Build Feature");
        model.tasks.insert(task.id.clone(), task.clone());

        // Search with different cases
        model.filter.search_text = Some("BUILD".to_string());
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        model.filter.search_text = Some("feature".to_string());
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);
    }

    #[test]
    fn test_search_filter_matches_tags() {
        let mut model = Model::new();

        let task_with_tag = Task::new("Some task").with_tags(vec!["urgent".to_string()]);
        let task_no_tag = Task::new("Other task");

        model
            .tasks
            .insert(task_with_tag.id.clone(), task_with_tag.clone());
        model.tasks.insert(task_no_tag.id.clone(), task_no_tag);

        model.filter.search_text = Some("urgent".to_string());
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_with_tag.id);
    }

    #[test]
    fn test_search_filter_partial_match() {
        let mut model = Model::new();

        let task = Task::new("Implement authentication");
        model.tasks.insert(task.id.clone(), task.clone());

        model.filter.search_text = Some("auth".to_string());
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
    }

    #[test]
    fn test_search_filter_empty_clears() {
        let mut model = Model::new();

        let task1 = Task::new("Task one");
        let task2 = Task::new("Task two");

        model.tasks.insert(task1.id.clone(), task1);
        model.tasks.insert(task2.id.clone(), task2);

        // With filter
        model.filter.search_text = Some("one".to_string());
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        // Without filter
        model.filter.search_text = None;
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 2);
    }

    #[test]
    fn test_tag_filter_any_mode() {
        use crate::domain::TagFilterMode;

        let mut model = Model::new();

        let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
        let task_python = Task::new("Task Python").with_tags(vec!["python".to_string()]);
        let task_both =
            Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
        let task_none = Task::new("Task None");

        model.tasks.insert(task_rust.id.clone(), task_rust.clone());
        model
            .tasks
            .insert(task_python.id.clone(), task_python.clone());
        model.tasks.insert(task_both.id.clone(), task_both.clone());
        model.tasks.insert(task_none.id.clone(), task_none);

        // Filter by "rust" tag (Any mode - default)
        model.filter.tags = Some(vec!["rust".to_string()]);
        model.filter.tags_mode = TagFilterMode::Any;
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 2);
        assert!(model.visible_tasks.contains(&task_rust.id));
        assert!(model.visible_tasks.contains(&task_both.id));
    }

    #[test]
    fn test_tag_filter_all_mode() {
        use crate::domain::TagFilterMode;

        let mut model = Model::new();

        let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
        let task_both =
            Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
        let task_none = Task::new("Task None");

        model.tasks.insert(task_rust.id.clone(), task_rust.clone());
        model.tasks.insert(task_both.id.clone(), task_both.clone());
        model.tasks.insert(task_none.id.clone(), task_none);

        // Filter by "rust" AND "python" tags (All mode)
        model.filter.tags = Some(vec!["rust".to_string(), "python".to_string()]);
        model.filter.tags_mode = TagFilterMode::All;
        model.refresh_visible_tasks();

        // Only task_both has both tags
        assert_eq!(model.visible_tasks.len(), 1);
        assert!(model.visible_tasks.contains(&task_both.id));
    }

    #[test]
    fn test_tag_filter_case_insensitive() {
        let mut model = Model::new();

        let task = Task::new("Task").with_tags(vec!["Rust".to_string()]);
        model.tasks.insert(task.id.clone(), task.clone());

        // Filter with different case
        model.filter.tags = Some(vec!["rust".to_string()]);
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
        assert!(model.visible_tasks.contains(&task.id));
    }

    #[test]
    fn test_tag_filter_clear() {
        let mut model = Model::new();

        let task_tagged = Task::new("Tagged").with_tags(vec!["work".to_string()]);
        let task_untagged = Task::new("Untagged");

        model
            .tasks
            .insert(task_tagged.id.clone(), task_tagged.clone());
        model.tasks.insert(task_untagged.id.clone(), task_untagged);

        // With filter
        model.filter.tags = Some(vec!["work".to_string()]);
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        // Clear filter
        model.filter.tags = None;
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 2);
    }

    #[test]
    fn test_tag_filter_with_search() {
        let mut model = Model::new();

        let task_match =
            Task::new("Important Task").with_tags(vec!["work".to_string(), "urgent".to_string()]);
        let task_wrong_tag = Task::new("Important Other").with_tags(vec!["home".to_string()]);
        let task_wrong_title = Task::new("Regular Task").with_tags(vec!["work".to_string()]);

        model
            .tasks
            .insert(task_match.id.clone(), task_match.clone());
        model
            .tasks
            .insert(task_wrong_tag.id.clone(), task_wrong_tag);
        model
            .tasks
            .insert(task_wrong_title.id.clone(), task_wrong_title);

        // Both search and tag filter
        model.filter.search_text = Some("Important".to_string());
        model.filter.tags = Some(vec!["work".to_string()]);
        model.refresh_visible_tasks();

        // Only task_match matches both criteria
        assert_eq!(model.visible_tasks.len(), 1);
        assert!(model.visible_tasks.contains(&task_match.id));
    }

    #[test]
    fn test_sort_by_title() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let task_b = Task::new("Banana");
        let task_a = Task::new("Apple");
        let task_c = Task::new("Cherry");

        model.tasks.insert(task_b.id.clone(), task_b.clone());
        model.tasks.insert(task_a.id.clone(), task_a.clone());
        model.tasks.insert(task_c.id.clone(), task_c.clone());

        model.sort = SortSpec {
            field: SortField::Title,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks[0], task_a.id);
        assert_eq!(model.visible_tasks[1], task_b.id);
        assert_eq!(model.visible_tasks[2], task_c.id);
    }

    #[test]
    fn test_sort_by_title_descending() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let task_b = Task::new("Banana");
        let task_a = Task::new("Apple");
        let task_c = Task::new("Cherry");

        model.tasks.insert(task_b.id.clone(), task_b.clone());
        model.tasks.insert(task_a.id.clone(), task_a.clone());
        model.tasks.insert(task_c.id.clone(), task_c.clone());

        model.sort = SortSpec {
            field: SortField::Title,
            order: SortOrder::Descending,
        };
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks[0], task_c.id);
        assert_eq!(model.visible_tasks[1], task_b.id);
        assert_eq!(model.visible_tasks[2], task_a.id);
    }

    #[test]
    fn test_sort_by_due_date() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let today = chrono::Utc::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);
        let next_week = today + chrono::Duration::days(7);

        let task_soon = Task::new("Soon").with_due_date(tomorrow);
        let task_later = Task::new("Later").with_due_date(next_week);
        let task_no_date = Task::new("No date");

        model
            .tasks
            .insert(task_later.id.clone(), task_later.clone());
        model.tasks.insert(task_soon.id.clone(), task_soon.clone());
        model
            .tasks
            .insert(task_no_date.id.clone(), task_no_date.clone());

        model.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        // Tasks with dates come first, then tasks without dates
        assert_eq!(model.visible_tasks[0], task_soon.id);
        assert_eq!(model.visible_tasks[1], task_later.id);
        assert_eq!(model.visible_tasks[2], task_no_date.id);
    }

    #[test]
    fn test_sort_by_status() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();
        model.show_completed = true; // Show completed for this test

        let task_todo = Task::new("Todo").with_status(TaskStatus::Todo);
        let task_in_progress = Task::new("In Progress").with_status(TaskStatus::InProgress);
        let task_done = Task::new("Done").with_status(TaskStatus::Done);

        model.tasks.insert(task_done.id.clone(), task_done.clone());
        model.tasks.insert(task_todo.id.clone(), task_todo.clone());
        model
            .tasks
            .insert(task_in_progress.id.clone(), task_in_progress.clone());

        model.sort = SortSpec {
            field: SortField::Status,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        // Order: InProgress, Todo, Blocked, Done, Cancelled
        assert_eq!(model.visible_tasks[0], task_in_progress.id);
        assert_eq!(model.visible_tasks[1], task_todo.id);
        assert_eq!(model.visible_tasks[2], task_done.id);
    }

    #[test]
    fn test_sort_order_toggle() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let task_high = Task::new("High").with_priority(Priority::High);
        let task_low = Task::new("Low").with_priority(Priority::Low);

        model.tasks.insert(task_high.id.clone(), task_high.clone());
        model.tasks.insert(task_low.id.clone(), task_low.clone());

        // Ascending: High first (lower priority number)
        model.sort = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks[0], task_high.id);
        assert_eq!(model.visible_tasks[1], task_low.id);

        // Descending: Low first
        model.sort.order = SortOrder::Descending;
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks[0], task_low.id);
        assert_eq!(model.visible_tasks[1], task_high.id);
    }
}
