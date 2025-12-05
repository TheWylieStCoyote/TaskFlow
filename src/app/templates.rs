use crate::domain::{Priority, Task};

/// A task template for quickly creating common task types
#[derive(Debug, Clone)]
pub struct TaskTemplate {
    /// Template name (for display)
    pub name: String,
    /// Default title (user can edit)
    pub title: String,
    /// Default priority
    pub priority: Priority,
    /// Default tags
    pub tags: Vec<String>,
    /// Default description
    pub description: Option<String>,
    /// Days from now for due date (None = no due date)
    pub due_days: Option<i64>,
}

impl TaskTemplate {
    #[must_use]
    pub fn new(name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            priority: Priority::None,
            tags: Vec::new(),
            description: None,
            due_days: None,
        }
    }

    #[must_use]
    pub const fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub const fn with_due_days(mut self, days: i64) -> Self {
        self.due_days = Some(days);
        self
    }

    /// Create a task from this template
    #[must_use]
    pub fn create_task(&self) -> Task {
        use chrono::{Duration, Utc};

        let mut task = Task::new(&self.title);
        task.priority = self.priority;
        task.tags.clone_from(&self.tags);
        task.description.clone_from(&self.description);

        if let Some(days) = self.due_days {
            let due = (Utc::now() + Duration::days(days)).date_naive();
            task.due_date = Some(due);
        }

        task
    }
}

/// Template manager with predefined and custom templates
#[derive(Debug, Clone, Default)]
pub struct TemplateManager {
    /// Available templates
    pub templates: Vec<TaskTemplate>,
}

impl TemplateManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            templates: Self::default_templates(),
        }
    }

    /// Get default built-in templates
    fn default_templates() -> Vec<TaskTemplate> {
        vec![
            TaskTemplate::new("Bug Fix", "Fix: ")
                .with_priority(Priority::High)
                .with_tags(vec!["bug".to_string()])
                .with_description("Steps to reproduce:\n\nExpected behavior:\n\nActual behavior:"),
            TaskTemplate::new("Feature", "Implement: ")
                .with_priority(Priority::Medium)
                .with_tags(vec!["feature".to_string()])
                .with_description("Description:\n\nAcceptance criteria:"),
            TaskTemplate::new("Review", "Review: ")
                .with_priority(Priority::Medium)
                .with_tags(vec!["review".to_string()])
                .with_due_days(1),
            TaskTemplate::new("Meeting Notes", "Meeting: ")
                .with_priority(Priority::Low)
                .with_tags(vec!["meeting".to_string()])
                .with_description("Attendees:\n\nAgenda:\n\nAction items:"),
            TaskTemplate::new("Daily Task", "Daily: ")
                .with_priority(Priority::Low)
                .with_due_days(0),
            TaskTemplate::new("Weekly Task", "Weekly: ")
                .with_priority(Priority::Low)
                .with_due_days(7),
            TaskTemplate::new("Urgent", "URGENT: ")
                .with_priority(Priority::Urgent)
                .with_due_days(0),
            TaskTemplate::new("Research", "Research: ")
                .with_priority(Priority::Low)
                .with_tags(vec!["research".to_string()])
                .with_description("Goal:\n\nFindings:\n\nConclusion:"),
        ]
    }

    /// Get template by index
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&TaskTemplate> {
        self.templates.get(index)
    }

    /// Get number of templates
    #[must_use]
    pub const fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Add a custom template
    pub fn add_template(&mut self, template: TaskTemplate) {
        self.templates.push(template);
    }

    /// Get template names for display
    #[must_use]
    pub fn template_names(&self) -> Vec<&str> {
        self.templates.iter().map(|t| t.name.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_new() {
        let template = TaskTemplate::new("Test", "Test Task");
        assert_eq!(template.name, "Test");
        assert_eq!(template.title, "Test Task");
        assert_eq!(template.priority, Priority::None);
        assert!(template.tags.is_empty());
        assert!(template.description.is_none());
        assert!(template.due_days.is_none());
    }

    #[test]
    fn test_template_with_priority() {
        let template = TaskTemplate::new("Test", "Task").with_priority(Priority::High);
        assert_eq!(template.priority, Priority::High);
    }

    #[test]
    fn test_template_with_tags() {
        let template = TaskTemplate::new("Test", "Task")
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(template.tags.len(), 2);
        assert_eq!(template.tags[0], "tag1");
    }

    #[test]
    fn test_template_with_description() {
        let template = TaskTemplate::new("Test", "Task").with_description("Description");
        assert_eq!(template.description, Some("Description".to_string()));
    }

    #[test]
    fn test_template_with_due_days() {
        let template = TaskTemplate::new("Test", "Task").with_due_days(7);
        assert_eq!(template.due_days, Some(7));
    }

    #[test]
    fn test_template_create_task() {
        let template = TaskTemplate::new("Test", "Test Task")
            .with_priority(Priority::Medium)
            .with_tags(vec!["test".to_string()]);

        let task = template.create_task();
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.tags, vec!["test".to_string()]);
    }

    #[test]
    fn test_template_create_task_with_due_date() {
        use chrono::{Duration, Utc};

        let template = TaskTemplate::new("Test", "Task").with_due_days(3);
        let task = template.create_task();

        let expected_date = (Utc::now() + Duration::days(3)).date_naive();
        assert_eq!(task.due_date, Some(expected_date));
    }

    #[test]
    fn test_template_manager_new() {
        let manager = TemplateManager::new();
        assert!(!manager.is_empty());
        assert!(manager.len() > 0);
    }

    #[test]
    fn test_template_manager_get() {
        let manager = TemplateManager::new();
        let template = manager.get(0);
        assert!(template.is_some());
    }

    #[test]
    fn test_template_manager_get_out_of_bounds() {
        let manager = TemplateManager::new();
        let template = manager.get(100);
        assert!(template.is_none());
    }

    #[test]
    fn test_template_manager_add() {
        let mut manager = TemplateManager::new();
        let initial_len = manager.len();

        manager.add_template(TaskTemplate::new("Custom", "Custom Task"));
        assert_eq!(manager.len(), initial_len + 1);
    }

    #[test]
    fn test_template_manager_names() {
        let manager = TemplateManager::new();
        let names = manager.template_names();

        assert!(!names.is_empty());
        assert!(names.contains(&"Bug Fix"));
        assert!(names.contains(&"Feature"));
    }

    #[test]
    fn test_default_templates_exist() {
        let manager = TemplateManager::new();

        // Verify we have expected templates
        let names = manager.template_names();
        assert!(names.contains(&"Bug Fix"));
        assert!(names.contains(&"Feature"));
        assert!(names.contains(&"Review"));
        assert!(names.contains(&"Urgent"));
    }
}
