use std::collections::HashMap;

use crate::domain::{Filter, Project, ProjectId, SortSpec, Task, TaskId};

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
}

impl Model {
    pub fn new() -> Self {
        Self {
            running: RunningState::default(),
            tasks: HashMap::new(),
            projects: HashMap::new(),
            current_view: ViewId::default(),
            selected_index: 0,
            visible_tasks: Vec::new(),
            filter: Filter::default(),
            sort: SortSpec::default(),
            show_completed: false,
            show_sidebar: true,
            show_help: false,
            terminal_size: (80, 24),
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
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}
