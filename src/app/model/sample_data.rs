//! Sample data generation for the Model.

use chrono::{Duration, NaiveDate, Utc, Weekday};

use crate::domain::{Habit, HabitFrequency, Priority, Project, Task, TaskStatus, TimeEntry};

use super::Model;

impl Model {
    /// Adds sample tasks and projects for testing.
    ///
    /// Creates a comprehensive set of example tasks across multiple projects with
    /// various priorities, statuses, due dates, subtasks, dependencies, time entries,
    /// and habits. Useful for development and demonstration.
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
    #[allow(clippy::too_many_lines)]
    pub fn with_sample_data(mut self) -> Self {
        // Disable storage to prevent sample data from being persisted
        self.storage.backend = None;
        self.storage.sample_data_mode = true;

        let now = Utc::now();
        let today = now.date_naive();

        // ========== PROJECTS ==========
        let backend_project = Project::new("Backend API");
        let frontend_project = Project::new("Frontend UI");
        let docs_project = Project::new("Documentation");
        let devops_project = Project::new("DevOps & Infrastructure");
        let mobile_project = Project::new("Mobile App");
        let personal_project = Project::new("Personal");
        let home_project = Project::new("Home & Errands");
        let learning_project = Project::new("Learning & Growth");
        let side_project = Project::new("Side Project: TaskFlow");
        let health_project = Project::new("Health & Fitness");

        let backend_id = backend_project.id;
        let frontend_id = frontend_project.id;
        let docs_id = docs_project.id;
        let devops_id = devops_project.id;
        let mobile_id = mobile_project.id;
        let personal_id = personal_project.id;
        let home_id = home_project.id;
        let learning_id = learning_project.id;
        let side_id = side_project.id;
        let health_id = health_project.id;

        self.projects.insert(backend_id, backend_project);
        self.projects.insert(frontend_id, frontend_project);
        self.projects.insert(docs_id, docs_project);
        self.projects.insert(devops_id, devops_project);
        self.projects.insert(mobile_id, mobile_project);
        self.projects.insert(personal_id, personal_project);
        self.projects.insert(home_id, home_project);
        self.projects.insert(learning_id, learning_project);
        self.projects.insert(side_id, side_project);
        self.projects.insert(health_id, health_project);

        // Date helpers
        let days_ago = |d: i64| today - Duration::days(d);
        let days_from_now = |d: i64| today + Duration::days(d);

        // ========== BACKEND API TASKS ==========
        let mut tasks = vec![
            Task::new("Set up database schema")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(backend_id)
                .with_tags(vec!["database".into(), "setup".into()])
                .with_completed_at(now - Duration::days(45)),
            Task::new("Implement user authentication")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Urgent)
                .with_project(backend_id)
                .with_tags(vec!["security".into(), "auth".into()])
                .with_completed_at(now - Duration::days(38)),
            Task::new("Create REST API endpoints")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(backend_id)
                .with_tags(vec!["api".into(), "rust".into()])
                .with_completed_at(now - Duration::days(30)),
            Task::new("Add rate limiting middleware")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(backend_id)
                .with_tags(vec!["security".into(), "api".into()])
                .with_estimated_minutes(120),
            Task::new("Implement caching layer")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(3))
                .with_project(backend_id)
                .with_tags(vec!["performance".into(), "redis".into()])
                .with_estimated_minutes(180),
            Task::new("Write API integration tests")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(5))
                .with_project(backend_id)
                .with_tags(vec!["testing".into(), "api".into()])
                .with_estimated_minutes(240),
            Task::new("Set up database migrations")
                .with_priority(Priority::Medium)
                .with_project(backend_id)
                .with_tags(vec!["database".into()]),
            Task::new("Add request validation")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(7))
                .with_project(backend_id)
                .with_tags(vec!["api".into(), "validation".into()]),
            Task::new("Implement pagination for list endpoints")
                .with_priority(Priority::Low)
                .with_project(backend_id)
                .with_tags(vec!["api".into()]),
            Task::new("Add API versioning")
                .with_priority(Priority::Low)
                .with_due_date(days_from_now(14))
                .with_project(backend_id)
                .with_tags(vec!["api".into(), "architecture".into()]),
        ];

        // ========== FRONTEND UI TASKS ==========
        tasks.extend(vec![
            Task::new("Design component library")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(frontend_id)
                .with_tags(vec!["design".into(), "ui".into()])
                .with_completed_at(now - Duration::days(60)),
            Task::new("Build task list widget")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "rust".into()])
                .with_completed_at(now - Duration::days(42)),
            Task::new("Implement sidebar navigation")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "navigation".into()])
                .with_completed_at(now - Duration::days(28)),
            Task::new("Add keyboard shortcuts")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(frontend_id)
                .with_tags(vec!["ux".into(), "accessibility".into()])
                .with_estimated_minutes(90),
            Task::new("Implement dark mode theme")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(4))
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "theme".into()])
                .with_estimated_minutes(120),
            Task::new("Add drag-and-drop task reordering")
                .with_priority(Priority::Low)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "ux".into()]),
            Task::new("Create calendar view")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(10))
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "feature".into()])
                .with_estimated_minutes(300),
            Task::new("Add task filtering UI")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Medium)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "ux".into()])
                .with_completed_at(now - Duration::days(14)),
            Task::new("Implement search functionality")
                .with_priority(Priority::High)
                .with_due_date(today)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "feature".into()])
                .with_estimated_minutes(150),
            Task::new("Add responsive layout")
                .with_priority(Priority::Low)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "responsive".into()]),
        ]);

        // ========== DOCUMENTATION TASKS ==========
        tasks.extend(vec![
            Task::new("Write API documentation")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(7))
                .with_project(docs_id)
                .with_tags(vec!["docs".into(), "api".into()])
                .with_estimated_minutes(240),
            Task::new("Create user guide")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(14))
                .with_project(docs_id)
                .with_tags(vec!["docs".into(), "guide".into()])
                .with_estimated_minutes(360),
            Task::new("Write installation instructions")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(docs_id)
                .with_tags(vec!["docs".into(), "setup".into()])
                .with_completed_at(now - Duration::days(21)),
            Task::new("Document configuration options")
                .with_priority(Priority::Medium)
                .with_project(docs_id)
                .with_tags(vec!["docs".into(), "config".into()]),
            Task::new("Create contribution guidelines")
                .with_priority(Priority::Low)
                .with_project(docs_id)
                .with_tags(vec!["docs".into(), "community".into()]),
            Task::new("Add code examples")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(10))
                .with_project(docs_id)
                .with_tags(vec!["docs".into(), "examples".into()]),
        ]);

        // ========== DEVOPS TASKS ==========
        tasks.extend(vec![
            Task::new("Set up CI/CD pipeline")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Urgent)
                .with_project(devops_id)
                .with_tags(vec!["devops".into(), "ci".into()])
                .with_completed_at(now - Duration::days(90)),
            Task::new("Configure Docker containers")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(devops_id)
                .with_tags(vec!["devops".into(), "docker".into()])
                .with_completed_at(now - Duration::days(75)),
            Task::new("Set up monitoring and alerts")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(devops_id)
                .with_tags(vec!["devops".into(), "monitoring".into()])
                .with_estimated_minutes(180),
            Task::new("Configure auto-scaling")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(12))
                .with_project(devops_id)
                .with_tags(vec!["devops".into(), "infrastructure".into()]),
            Task::new("Set up staging environment")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(5))
                .with_project(devops_id)
                .with_tags(vec!["devops".into(), "environment".into()])
                .with_estimated_minutes(120),
            Task::new("Implement backup strategy")
                .with_priority(Priority::Urgent)
                .with_due_date(days_from_now(2))
                .with_project(devops_id)
                .with_tags(vec!["devops".into(), "backup".into()])
                .with_estimated_minutes(90),
            Task::new("Security audit")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(21))
                .with_project(devops_id)
                .with_tags(vec!["security".into(), "audit".into()]),
        ]);

        // ========== MOBILE APP TASKS ==========
        tasks.extend(vec![
            Task::new("Design mobile UI mockups")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(mobile_id)
                .with_tags(vec!["mobile".into(), "design".into()])
                .with_completed_at(now - Duration::days(35)),
            Task::new("Set up React Native project")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(mobile_id)
                .with_tags(vec!["mobile".into(), "setup".into()])
                .with_estimated_minutes(60),
            Task::new("Implement offline sync")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(14))
                .with_project(mobile_id)
                .with_tags(vec!["mobile".into(), "sync".into()])
                .with_estimated_minutes(480),
            Task::new("Add push notifications")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(21))
                .with_project(mobile_id)
                .with_tags(vec!["mobile".into(), "notifications".into()]),
            Task::new("Implement biometric authentication")
                .with_priority(Priority::Medium)
                .with_project(mobile_id)
                .with_tags(vec!["mobile".into(), "security".into()]),
        ]);

        // ========== PERSONAL TASKS ==========
        tasks.extend(vec![
            Task::new("Renew passport")
                .with_priority(Priority::Urgent)
                .with_due_date(days_from_now(30))
                .with_project(personal_id)
                .with_tags(vec!["personal".into(), "documents".into()]),
            Task::new("Schedule dentist appointment")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(7))
                .with_project(personal_id)
                .with_tags(vec!["health".into(), "appointment".into()]),
            Task::new("Plan weekend trip")
                .with_priority(Priority::Low)
                .with_project(personal_id)
                .with_tags(vec!["personal".into(), "travel".into()]),
            Task::new("Call mom")
                .with_priority(Priority::Medium)
                .with_due_date(today)
                .with_project(personal_id)
                .with_tags(vec!["family".into()]),
            Task::new("Review monthly budget")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(3))
                .with_project(personal_id)
                .with_tags(vec!["finance".into()]),
            Task::new("Pay credit card bill")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Urgent)
                .with_project(personal_id)
                .with_tags(vec!["finance".into(), "bills".into()])
                .with_completed_at(now - Duration::days(5)),
        ]);

        // ========== HOME & ERRANDS TASKS ==========
        tasks.extend(vec![
            Task::new("Grocery shopping")
                .with_priority(Priority::Medium)
                .with_due_date(today)
                .with_project(home_id)
                .with_tags(vec!["errands".into(), "shopping".into()]),
            Task::new("Clean garage")
                .with_priority(Priority::Low)
                .with_due_date(days_from_now(14))
                .with_project(home_id)
                .with_tags(vec!["home".into(), "cleaning".into()])
                .with_estimated_minutes(180),
            Task::new("Fix leaky faucet")
                .with_priority(Priority::Medium)
                .with_project(home_id)
                .with_tags(vec!["home".into(), "repairs".into()]),
            Task::new("Mow the lawn")
                .with_priority(Priority::Low)
                .with_due_date(days_from_now(5))
                .with_project(home_id)
                .with_tags(vec!["home".into(), "yard".into()]),
            Task::new("Schedule HVAC maintenance")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(21))
                .with_project(home_id)
                .with_tags(vec!["home".into(), "maintenance".into()]),
            Task::new("Organize closet")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Low)
                .with_project(home_id)
                .with_tags(vec!["home".into(), "organizing".into()])
                .with_completed_at(now - Duration::days(8)),
            Task::new("Return Amazon package")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(2))
                .with_project(home_id)
                .with_tags(vec!["errands".into()]),
        ]);

        // ========== LEARNING TASKS ==========
        tasks.extend(vec![
            Task::new("Complete Rust async chapter")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(learning_id)
                .with_tags(vec!["learning".into(), "rust".into()])
                .with_estimated_minutes(120),
            Task::new("Watch system design videos")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(7))
                .with_project(learning_id)
                .with_tags(vec!["learning".into(), "architecture".into()]),
            Task::new("Practice LeetCode problems")
                .with_priority(Priority::Medium)
                .with_project(learning_id)
                .with_tags(vec!["learning".into(), "algorithms".into()])
                .with_estimated_minutes(60),
            Task::new("Read 'Designing Data-Intensive Applications'")
                .with_priority(Priority::Low)
                .with_project(learning_id)
                .with_tags(vec!["learning".into(), "reading".into()]),
            Task::new("Take AWS certification exam")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(45))
                .with_project(learning_id)
                .with_tags(vec!["learning".into(), "certification".into()]),
            Task::new("Learn Kubernetes basics")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Medium)
                .with_project(learning_id)
                .with_tags(vec!["learning".into(), "devops".into()])
                .with_completed_at(now - Duration::days(18)),
        ]);

        // ========== SIDE PROJECT TASKS ==========
        tasks.extend(vec![
            Task::new("Add mouse support")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Medium)
                .with_project(side_id)
                .with_tags(vec!["feature".into(), "ux".into()])
                .with_completed_at(now - Duration::days(3)),
            Task::new("Implement heatmap view")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::Medium)
                .with_project(side_id)
                .with_tags(vec!["feature".into(), "analytics".into()])
                .with_completed_at(now - Duration::days(2)),
            Task::new("Add quick capture mode")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(side_id)
                .with_tags(vec!["feature".into(), "ux".into()])
                .with_completed_at(now - Duration::days(1)),
            Task::new("Write unit tests for new features")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(2))
                .with_project(side_id)
                .with_tags(vec!["testing".into()])
                .with_estimated_minutes(180),
            Task::new("Optimize rendering performance")
                .with_priority(Priority::Medium)
                .with_project(side_id)
                .with_tags(vec!["performance".into()]),
            Task::new("Add vim-style navigation")
                .with_priority(Priority::Low)
                .with_project(side_id)
                .with_tags(vec!["feature".into(), "vim".into()]),
            Task::new("Implement undo/redo")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(side_id)
                .with_tags(vec!["feature".into()])
                .with_completed_at(now - Duration::days(25)),
            Task::new("Add time tracking")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(side_id)
                .with_tags(vec!["feature".into()])
                .with_completed_at(now - Duration::days(20)),
        ]);

        // ========== HEALTH & FITNESS TASKS ==========
        tasks.extend(vec![
            Task::new("Schedule annual checkup")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(14))
                .with_project(health_id)
                .with_tags(vec!["health".into(), "appointment".into()]),
            Task::new("Research gym memberships")
                .with_priority(Priority::Low)
                .with_project(health_id)
                .with_tags(vec!["fitness".into()]),
            Task::new("Buy running shoes")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(7))
                .with_project(health_id)
                .with_tags(vec!["fitness".into(), "shopping".into()]),
            Task::new("Meal prep for the week")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(2))
                .with_project(health_id)
                .with_tags(vec!["health".into(), "nutrition".into()])
                .with_estimated_minutes(120),
        ]);

        // ========== STANDALONE TASKS (No Project) ==========
        tasks.extend(vec![
            Task::new("Fix critical bug in parser")
                .with_priority(Priority::Urgent)
                .with_due_date(days_ago(1))
                .with_tags(vec!["bug".into(), "urgent".into()]),
            Task::new("Review pull requests")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::Medium)
                .with_due_date(today)
                .with_tags(vec!["review".into(), "code".into()]),
            Task::new("Update dependencies")
                .with_priority(Priority::Low)
                .with_tags(vec!["maintenance".into()]),
            Task::new("Plan next sprint")
                .with_priority(Priority::Medium)
                .with_due_date(NaiveDate::from_ymd_opt(2025, 12, 15).unwrap())
                .with_tags(vec!["planning".into(), "team".into()]),
            Task::new("Team sync meeting")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::None)
                .with_tags(vec!["meeting".into()])
                .with_completed_at(now - Duration::days(1)),
            Task::new("Send weekly status report")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(1))
                .with_tags(vec!["reporting".into()]),
            Task::new("Prepare presentation for stakeholders")
                .with_priority(Priority::High)
                .with_due_date(days_from_now(6))
                .with_tags(vec!["presentation".into(), "stakeholders".into()])
                .with_estimated_minutes(240),
            Task::new("1:1 with manager")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(3))
                .with_tags(vec!["meeting".into(), "1on1".into()]),
            Task::new("Submit expense report")
                .with_priority(Priority::Medium)
                .with_due_date(days_from_now(5))
                .with_tags(vec!["admin".into(), "finance".into()]),
            Task::new("Book conference tickets")
                .with_priority(Priority::Low)
                .with_due_date(days_from_now(30))
                .with_tags(vec!["conference".into(), "travel".into()]),
        ]);

        // ========== BLOCKED TASKS ==========
        tasks.extend(vec![
            Task::new("Deploy to production")
                .with_status(TaskStatus::Blocked)
                .with_priority(Priority::Urgent)
                .with_project(devops_id)
                .with_tags(vec!["deployment".into(), "blocked".into()]),
            Task::new("Client demo")
                .with_status(TaskStatus::Blocked)
                .with_priority(Priority::High)
                .with_due_date(days_from_now(8))
                .with_tags(vec!["demo".into(), "client".into()]),
        ]);

        // ========== ADDITIONAL HISTORICAL COMPLETED TASKS FOR HEATMAP ==========
        // Spread completed tasks across the past few months to populate the heatmap
        let historical_titles = [
            "Fix login redirect bug",
            "Update API rate limits",
            "Refactor database queries",
            "Add error logging",
            "Write unit tests",
            "Code review feedback",
            "Update README",
            "Fix CSS styling",
            "Optimize image loading",
            "Add form validation",
            "Fix memory leak",
            "Update npm packages",
            "Add loading spinner",
            "Fix timezone bug",
            "Improve search results",
            "Add pagination",
            "Fix mobile layout",
            "Update API docs",
            "Add caching headers",
            "Fix authentication bug",
            "Optimize SQL queries",
            "Add dark mode toggle",
            "Fix scroll behavior",
            "Update error messages",
            "Add keyboard navigation",
            "Fix date formatting",
            "Improve performance",
            "Add analytics tracking",
            "Fix email templates",
            "Update dependencies",
        ];

        for (i, title) in historical_titles.iter().enumerate() {
            // Spread across past 120 days with varying intervals
            let days_back = (i as i64 * 4) + (i as i64 % 3);
            tasks.push(
                Task::new(*title)
                    .with_status(TaskStatus::Done)
                    .with_priority(if i % 3 == 0 {
                        Priority::High
                    } else {
                        Priority::Medium
                    })
                    .with_tags(vec!["historical".into()])
                    .with_completed_at(now - Duration::days(days_back)),
            );
        }

        // Add some clusters of completions (productive days)
        let productive_days = [7, 14, 21, 28, 35, 42, 56, 70];
        for day in productive_days {
            for j in 0..3 {
                tasks.push(
                    Task::new(format!("Batch task {} from day {}", j + 1, day))
                        .with_status(TaskStatus::Done)
                        .with_priority(Priority::Medium)
                        .with_tags(vec!["batch".into()])
                        .with_completed_at(now - Duration::days(day)),
                );
            }
        }

        // Insert all tasks and collect IDs for dependencies/subtasks
        let mut task_ids: Vec<_> = Vec::new();
        for task in tasks {
            let id = task.id;
            self.tasks.insert(id, task);
            task_ids.push(id);
        }

        // ========== SUBTASKS ==========
        // Find "Write API integration tests" and add subtasks
        if let Some(parent_task) = self
            .tasks
            .values()
            .find(|t| t.title == "Write API integration tests")
        {
            let parent_id = parent_task.id;
            let subtasks = vec![
                Task::new("Test authentication endpoints")
                    .with_priority(Priority::High)
                    .with_project(backend_id)
                    .with_parent(parent_id)
                    .with_tags(vec!["testing".into()]),
                Task::new("Test CRUD operations")
                    .with_priority(Priority::High)
                    .with_project(backend_id)
                    .with_parent(parent_id)
                    .with_tags(vec!["testing".into()]),
                Task::new("Test error handling")
                    .with_priority(Priority::Medium)
                    .with_project(backend_id)
                    .with_parent(parent_id)
                    .with_tags(vec!["testing".into()]),
            ];
            for st in subtasks {
                self.tasks.insert(st.id, st);
            }
        }

        // Find "Create user guide" and add subtasks
        if let Some(parent_task) = self.tasks.values().find(|t| t.title == "Create user guide") {
            let parent_id = parent_task.id;
            let subtasks = vec![
                Task::new("Write getting started section")
                    .with_status(TaskStatus::Done)
                    .with_priority(Priority::High)
                    .with_project(docs_id)
                    .with_parent(parent_id)
                    .with_completed_at(now - Duration::days(10)),
                Task::new("Document keyboard shortcuts")
                    .with_status(TaskStatus::InProgress)
                    .with_priority(Priority::Medium)
                    .with_project(docs_id)
                    .with_parent(parent_id),
                Task::new("Add screenshots")
                    .with_priority(Priority::Low)
                    .with_project(docs_id)
                    .with_parent(parent_id),
                Task::new("Write FAQ section")
                    .with_priority(Priority::Low)
                    .with_project(docs_id)
                    .with_parent(parent_id),
            ];
            for st in subtasks {
                self.tasks.insert(st.id, st);
            }
        }

        // ========== DEPENDENCIES ==========
        // "Deploy to production" depends on "Write API integration tests" and "Security audit"
        let deploy_id = self
            .tasks
            .values()
            .find(|t| t.title == "Deploy to production")
            .map(|t| t.id);
        let tests_id = self
            .tasks
            .values()
            .find(|t| t.title == "Write API integration tests")
            .map(|t| t.id);
        let audit_id = self
            .tasks
            .values()
            .find(|t| t.title == "Security audit")
            .map(|t| t.id);

        if let (Some(deploy), Some(tests), Some(audit)) = (deploy_id, tests_id, audit_id) {
            if let Some(task) = self.tasks.get_mut(&deploy) {
                task.dependencies = vec![tests, audit];
            }
        }

        // ========== TIME ENTRIES ==========
        // Add some time entries for completed tasks
        for task in self.tasks.values() {
            if task.status.is_complete() {
                if let Some(completed_at) = task.completed_at {
                    // Add 1-3 time entries per completed task
                    let entry_count = (task.id.0.as_u128() % 3 + 1) as i64;
                    for i in 0..entry_count {
                        let started = completed_at - Duration::hours(i + 1);
                        let duration = (30 + (task.id.0.as_u128() % 90)) as i64;
                        let mut entry = TimeEntry::start(task.id);
                        entry.started_at = started;
                        entry.ended_at = Some(started + Duration::minutes(duration));
                        entry.duration_minutes = Some(duration as u32);
                        self.time_entries.insert(entry.id, entry);
                    }
                }
            }
        }

        // Add time entries for in-progress tasks
        for task in self
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::InProgress)
        {
            let started = now - Duration::hours((task.id.0.as_u128() % 48) as i64);
            let duration = (15 + (task.id.0.as_u128() % 60)) as i64;
            let mut entry = TimeEntry::start(task.id);
            entry.started_at = started;
            entry.ended_at = Some(started + Duration::minutes(duration));
            entry.duration_minutes = Some(duration as u32);
            self.time_entries.insert(entry.id, entry);
        }

        // ========== HABITS ==========
        let mut habits = vec![
            Habit::new("Morning exercise")
                .with_frequency(HabitFrequency::Daily)
                .with_description("30 minutes of cardio or strength training"),
            Habit::new("Read for 30 minutes")
                .with_frequency(HabitFrequency::Daily)
                .with_description("Read technical or personal development books"),
            Habit::new("Review weekly goals")
                .with_frequency(HabitFrequency::Weekly {
                    days: vec![Weekday::Mon],
                })
                .with_description("Review and adjust weekly priorities"),
            Habit::new("Code review")
                .with_frequency(HabitFrequency::Daily)
                .with_description("Review at least one PR from teammates"),
            Habit::new("Meditate")
                .with_frequency(HabitFrequency::Daily)
                .with_description("10 minutes of mindfulness"),
            Habit::new("Learn something new")
                .with_frequency(HabitFrequency::Weekly {
                    days: vec![Weekday::Wed, Weekday::Sat],
                })
                .with_description("Watch a tech talk or read an article"),
        ];

        for habit in &mut habits {
            // Add some check-ins for each habit
            let check_in_count = (habit.id.0.as_u128() % 10 + 5) as i64;
            for i in 0..check_in_count {
                let check_date = days_ago(i * 2 + (habit.id.0.as_u128() % 3) as i64);
                habit.check_in(check_date, true, None);
            }
            self.habits.insert(habit.id, habit.clone());
        }

        self.refresh_visible_tasks();
        self
    }
}
