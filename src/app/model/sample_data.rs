//! Sample data generation for the Model.

use chrono::{NaiveDate, Utc};

use crate::domain::{Priority, Project, Task, TaskStatus};

use super::Model;

impl Model {
    /// Adds sample tasks and projects for testing.
    ///
    /// Creates a set of example tasks across multiple projects with
    /// various priorities, statuses, and due dates. Useful for
    /// development and demonstration.
    ///
    /// # Panics
    ///
    /// Panics if the current date cannot be computed for sample due dates.
    ///
    /// # Examples
    ///
    /// ```
    /// use taskflow::app::Model;
    ///
    /// let model = Model::new().with_sample_data();
    /// assert!(!model.tasks.is_empty());
    /// assert!(!model.projects.is_empty());
    /// ```
    #[must_use]
    pub fn with_sample_data(mut self) -> Self {
        // Create sample projects
        let backend_project = Project::new("Backend API");
        let frontend_project = Project::new("Frontend UI");
        let docs_project = Project::new("Documentation");

        let backend_id = backend_project.id;
        let frontend_id = frontend_project.id;
        let docs_id = docs_project.id;

        self.projects.insert(backend_id, backend_project);
        self.projects.insert(frontend_id, frontend_project);
        self.projects.insert(docs_id, docs_project);

        let today = Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let tomorrow = today + chrono::Duration::days(1);
        let next_week = today + chrono::Duration::days(7);

        let tasks = vec![
            // Backend tasks
            Task::new("Set up database schema")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(backend_id)
                .with_tags(vec!["database".into(), "setup".into()]),
            Task::new("Implement REST endpoints")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(backend_id)
                .with_tags(vec!["api".into(), "rust".into()]),
            Task::new("Add authentication middleware")
                .with_priority(Priority::Urgent)
                .with_due_date(tomorrow)
                .with_project(backend_id)
                .with_tags(vec!["security".into(), "api".into()]),
            Task::new("Write integration tests")
                .with_priority(Priority::Medium)
                .with_due_date(next_week)
                .with_project(backend_id)
                .with_tags(vec!["testing".into()]),
            // Frontend tasks
            Task::new("Design component library")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(frontend_id)
                .with_tags(vec!["design".into(), "ui".into()]),
            Task::new("Build task list widget")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "rust".into()]),
            Task::new("Add keyboard navigation")
                .with_priority(Priority::Medium)
                .with_due_date(today)
                .with_project(frontend_id)
                .with_tags(vec!["ux".into(), "accessibility".into()]),
            Task::new("Implement dark mode")
                .with_priority(Priority::Low)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "design".into()]),
            // Documentation tasks
            Task::new("Write API documentation")
                .with_priority(Priority::Medium)
                .with_due_date(next_week)
                .with_project(docs_id)
                .with_tags(vec!["docs".into(), "api".into()]),
            Task::new("Create user guide")
                .with_priority(Priority::Low)
                .with_project(docs_id)
                .with_tags(vec!["docs".into()]),
            // Standalone tasks (no project)
            Task::new("Fix critical bug in parser")
                .with_priority(Priority::Urgent)
                .with_due_date(yesterday)
                .with_tags(vec!["bug".into(), "urgent".into()]),
            Task::new("Review pull requests")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::Medium)
                .with_due_date(today)
                .with_tags(vec!["review".into()]),
            Task::new("Update dependencies")
                .with_priority(Priority::Low)
                .with_tags(vec!["maintenance".into()]),
            Task::new("Plan next sprint")
                .with_priority(Priority::Medium)
                .with_due_date(NaiveDate::from_ymd_opt(2025, 12, 15).unwrap())
                .with_tags(vec!["planning".into()]),
            Task::new("Team sync meeting")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::None)
                .with_tags(vec!["meeting".into()]),
        ];

        for task in tasks {
            self.tasks.insert(task.id, task);
        }

        self.refresh_visible_tasks();
        self
    }
}
