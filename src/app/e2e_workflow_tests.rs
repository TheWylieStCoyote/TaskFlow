//! End-to-end workflow tests.
//!
//! These tests simulate complete user workflows from start to finish,
//! testing the integration of all application layers: domain, app logic,
//! and storage.

use crate::app::{
    parse_quick_add, update, Message, Model, NavigationMessage, SystemMessage, TaskMessage,
    TimeMessage, UndoAction, ViewId,
};
use crate::domain::{Priority, Task, TaskStatus};
use crate::storage::BackendType;
use chrono::Utc;
use tempfile::TempDir;

// ============================================================================
// E2E Test Harness
// ============================================================================

/// Test harness for simulating complete user workflows.
///
/// Provides helpers for:
/// - Message sequence simulation
/// - State assertions
/// - Storage verification
struct E2ETestHarness {
    model: Model,
    temp_dir: TempDir,
}

impl E2ETestHarness {
    /// Create a new test harness with temporary storage.
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("create temp dir");
        let data_path = temp_dir.path().join("test_data.json");

        let model = Model::new()
            .with_storage(BackendType::Json, data_path)
            .expect("initialize storage");

        Self { model, temp_dir }
    }

    /// Send a message to the model and update state.
    fn send(&mut self, message: Message) -> &mut Self {
        update(&mut self.model, message);
        // Ensure selection is valid after update
        if self.model.selected_index >= self.model.visible_tasks.len()
            && !self.model.visible_tasks.is_empty()
        {
            self.model.selected_index = self.model.visible_tasks.len() - 1;
        }
        self
    }

    /// Navigate to a specific view.
    fn navigate_to(&mut self, view: ViewId) -> &mut Self {
        self.send(Message::Navigation(NavigationMessage::GoToView(view)))
    }

    /// Create a task with quick-add syntax.
    fn quick_add(&mut self, input: &str) -> &mut Self {
        // Parse the quick-add input
        let parsed = parse_quick_add(input);

        // Use parsed title, or original input if title is empty
        let title = if parsed.title.is_empty() {
            input.to_string()
        } else {
            parsed.title
        };

        // Create task and apply metadata
        let mut task = Task::new(title);

        // Apply parsed priority, or default priority if none specified
        if let Some(priority) = parsed.priority {
            task.priority = priority;
        } else {
            task.priority = self.model.default_priority;
        }

        // Apply tags
        if !parsed.tags.is_empty() {
            task = task.with_tags(parsed.tags);
        }

        // Apply due date
        if let Some(due) = parsed.due_date {
            task = task.with_due_date(due);
        }

        // Apply scheduled date
        if let Some(scheduled) = parsed.scheduled_date {
            task.scheduled_date = Some(scheduled);
        }

        // Apply project
        if let Some(project_name) = parsed.project_name {
            // Find project by name
            if let Some(project) = self
                .model
                .projects
                .values()
                .find(|p| p.name == project_name)
            {
                task.project_id = Some(project.id);
            }
        }

        // Apply time block
        if let Some(start) = parsed.scheduled_start_time {
            task.scheduled_start_time = Some(start);
        }
        if let Some(end) = parsed.scheduled_end_time {
            task.scheduled_end_time = Some(end);
        }

        // Add task to model (similar to what update handler does)
        self.model.sync_task(&task);
        self.model
            .undo_stack
            .push(UndoAction::TaskCreated(Box::new(task.clone())));
        let task_id = task.id;
        self.model.tasks.insert(task.id, task);
        self.model.refresh_visible_tasks();

        // Select the newly created task if it's in visible tasks
        if let Some(index) = self
            .model
            .visible_tasks
            .iter()
            .position(|id| *id == task_id)
        {
            self.model.selected_index = index;
        }

        self
    }

    /// Get the currently selected task, if any.
    fn selected_task(&self) -> Option<&Task> {
        self.model.selected_task()
    }

    /// Get all visible tasks in the current view.
    fn visible_tasks(&self) -> Vec<&Task> {
        self.model
            .visible_tasks
            .iter()
            .filter_map(|id| self.model.tasks.get(id))
            .collect()
    }

    /// Get total task count.
    fn task_count(&self) -> usize {
        self.model.tasks.len()
    }

    /// Get count of tasks with specific status.
    fn task_count_with_status(&self, status: TaskStatus) -> usize {
        self.model
            .tasks
            .values()
            .filter(|t| t.status == status)
            .count()
    }

    /// Navigate down in the list.
    fn nav_down(&mut self) -> &mut Self {
        self.send(Message::Navigation(NavigationMessage::Down))
    }

    /// Navigate up in the list.
    fn nav_up(&mut self) -> &mut Self {
        self.send(Message::Navigation(NavigationMessage::Up))
    }

    /// Toggle the completion status of selected task.
    fn toggle_complete(&mut self) -> &mut Self {
        self.send(Message::Task(TaskMessage::ToggleComplete))
    }

    /// Cycle priority of selected task.
    fn cycle_priority(&mut self) -> &mut Self {
        self.send(Message::Task(TaskMessage::CyclePriority))
    }

    /// Duplicate the selected task.
    fn duplicate(&mut self) -> &mut Self {
        self.send(Message::Task(TaskMessage::Duplicate))
    }

    /// Start time tracking on selected task.
    fn start_tracking(&mut self) -> &mut Self {
        self.send(Message::Time(TimeMessage::StartTracking))
    }

    /// Stop time tracking.
    fn stop_tracking(&mut self) -> &mut Self {
        self.send(Message::Time(TimeMessage::StopTracking))
    }

    /// Toggle time tracking.
    fn toggle_tracking(&mut self) -> &mut Self {
        self.send(Message::Time(TimeMessage::ToggleTracking))
    }

    /// Save state to storage.
    fn save(&mut self) -> &mut Self {
        self.send(Message::System(SystemMessage::Save))
    }

    /// Reload state from storage.
    fn reload(&mut self) -> Result<(), String> {
        // Save first
        self.save();

        // Create new model and load from same file
        let data_path = self.temp_dir.path().join("test_data.json");
        self.model = Model::new()
            .with_storage(BackendType::Json, data_path)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Assert task count equals expected value.
    fn assert_task_count(&self, expected: usize) {
        assert_eq!(
            self.task_count(),
            expected,
            "Expected {expected} tasks, found {}",
            self.task_count()
        );
    }

    /// Assert a task with given title exists.
    fn assert_task_exists(&self, title: &str) {
        assert!(
            self.model.tasks.values().any(|t| t.title == title),
            "Task '{title}' not found"
        );
    }

    /// Assert selected task has given title.
    fn assert_selected_title(&self, expected_title: &str) {
        let task = self.selected_task().expect("no task selected");
        assert_eq!(task.title, expected_title);
    }

    /// Assert selected task has given status.
    fn assert_selected_status(&self, expected_status: TaskStatus) {
        let task = self.selected_task().expect("no task selected");
        assert_eq!(task.status, expected_status);
    }

    /// Assert selected task has given priority.
    fn assert_selected_priority(&self, expected_priority: Priority) {
        let task = self.selected_task().expect("no task selected");
        assert_eq!(task.priority, expected_priority);
    }

    /// Assert number of visible tasks.
    fn assert_visible_count(&self, expected: usize) {
        let count = self.visible_tasks().len();
        assert_eq!(
            count, expected,
            "Expected {expected} visible tasks, found {count}"
        );
    }

    /// Assert task count with specific status.
    fn assert_task_count_with_status(&self, status: TaskStatus, expected: usize) {
        let actual = self.task_count_with_status(status);
        assert_eq!(
            actual, expected,
            "Expected {expected} tasks with status {status:?}, found {actual}"
        );
    }
}

// ============================================================================
// Workflow 1: Daily Task Management
// ============================================================================

#[test]
fn test_workflow_daily_task_management() {
    let mut harness = E2ETestHarness::new();

    // 1. Launch app → default view (TaskList)
    assert_eq!(harness.model.current_view, ViewId::TaskList);

    // 2. Navigate to Today view
    harness.navigate_to(ViewId::Today);
    assert_eq!(harness.model.current_view, ViewId::Today);

    // 3. Create new task with quick-add
    harness.quick_add("Fix bug #123 !high @work due:today");

    // 4. Task appears in list with correct properties
    harness.assert_task_count(1);
    harness.assert_task_exists("Fix bug");

    let task = harness.selected_task().expect("task should be selected");
    assert_eq!(task.priority, Priority::High);
    assert!(task.tags.contains(&"123".to_string()));
    assert_eq!(task.due_date, Some(Utc::now().date_naive()));

    // 5. Start time tracking
    harness.start_tracking();
    assert!(harness.model.active_time_entry().is_some());

    // 6. Stop time tracking
    harness.stop_tracking();
    assert!(harness.model.active_time_entry().is_none());

    // 7. Mark task complete
    let task_id = harness.selected_task().expect("should have task").id;
    harness.toggle_complete();

    // 8. Verify task completed (check by ID since it may no longer be selected/visible)
    harness.assert_task_count_with_status(TaskStatus::Done, 1);
    assert_eq!(
        harness.model.tasks.get(&task_id).unwrap().status,
        TaskStatus::Done
    );

    // 9. Verify undo restores task
    harness.send(Message::System(SystemMessage::Undo));
    harness.assert_task_count_with_status(TaskStatus::Todo, 1);
    assert_eq!(
        harness.model.tasks.get(&task_id).unwrap().status,
        TaskStatus::Todo
    );

    // 10. Exit app → verify persistence
    harness.save();
    harness.reload().expect("reload should succeed");
    harness.assert_task_count(1);
    harness.assert_task_exists("Fix bug");
}

// ============================================================================
// Workflow 2: Task Creation and Navigation
// ============================================================================

#[test]
fn test_workflow_task_creation_and_navigation() {
    let mut harness = E2ETestHarness::new();

    // Create multiple tasks
    harness.quick_add("Task 1");
    harness.quick_add("Task 2");
    harness.quick_add("Task 3");

    harness.assert_task_count(3);

    // Navigate to first task to establish selection
    harness.send(Message::Navigation(NavigationMessage::First));
    let first_title = harness
        .selected_task()
        .expect("should have selection")
        .title
        .clone();

    // Navigate down and verify we move to a different task
    harness.nav_down();
    let second_title = harness
        .selected_task()
        .expect("should have selection")
        .title
        .clone();
    assert_ne!(
        first_title, second_title,
        "Down navigation should change selection"
    );

    // Navigate down again
    harness.nav_down();
    let third_title = harness
        .selected_task()
        .expect("should have selection")
        .title
        .clone();
    assert_ne!(
        second_title, third_title,
        "Down navigation should change selection"
    );

    // Navigate up and verify we go back
    harness.nav_up();
    harness.assert_selected_title(&second_title);

    // Navigate to first
    harness.send(Message::Navigation(NavigationMessage::First));
    harness.assert_selected_title(&first_title);

    // Navigate to last
    harness.send(Message::Navigation(NavigationMessage::Last));
    let last_title = harness
        .selected_task()
        .expect("should have selection")
        .title
        .clone();
    assert_ne!(
        first_title, last_title,
        "Last task should be different from first"
    );

    // Verify persistence
    harness.save();
    harness.reload().expect("reload");
    harness.assert_task_count(3);
}

// ============================================================================
// Workflow 3: Time Tracking Session
// ============================================================================

#[test]
fn test_workflow_time_tracking() {
    let mut harness = E2ETestHarness::new();

    // Create Task A and start timer
    harness.quick_add("Task A");
    harness.start_tracking();
    assert!(harness.model.active_time_entry().is_some());

    // Stop timer
    harness.stop_tracking();
    assert!(harness.model.active_time_entry().is_none());

    // Create Task B and toggle timer (start)
    harness.quick_add("Task B");
    harness.toggle_tracking();
    assert!(harness.model.active_time_entry().is_some());

    // Toggle again (stop)
    harness.toggle_tracking();
    assert!(harness.model.active_time_entry().is_none());

    // Save and verify tasks persist
    harness.save();
    harness.reload().expect("reload");
    harness.assert_task_count(2);
}

// ============================================================================
// Workflow 4: Priority Cycling
// ============================================================================

#[test]
fn test_workflow_priority_cycling() {
    let mut harness = E2ETestHarness::new();

    // Create task with default priority (None)
    harness.quick_add("Task with cycling priority");
    harness.assert_selected_priority(Priority::None);

    // Cycle: None → Low
    harness.cycle_priority();
    harness.assert_selected_priority(Priority::Low);

    // Cycle: Low → Medium
    harness.cycle_priority();
    harness.assert_selected_priority(Priority::Medium);

    // Cycle: Medium → High
    harness.cycle_priority();
    harness.assert_selected_priority(Priority::High);

    // Cycle: High → Urgent
    harness.cycle_priority();
    harness.assert_selected_priority(Priority::Urgent);

    // Cycle: Urgent → None (wraps around)
    harness.cycle_priority();
    harness.assert_selected_priority(Priority::None);

    // Verify persistence
    harness.save();
    harness.reload().expect("reload");
    harness.assert_task_count(1);
}

// ============================================================================
// Workflow 5: Task Duplication
// ============================================================================

#[test]
fn test_workflow_task_duplication() {
    let mut harness = E2ETestHarness::new();

    // Create original task
    harness.quick_add("Original task !high #work");

    let original = harness.selected_task().unwrap().clone();
    assert_eq!(original.priority, Priority::High);
    assert!(original.tags.contains(&"work".to_string()));

    // Duplicate task
    harness.duplicate();

    // Should now have 2 tasks
    harness.assert_task_count(2);

    // Verify duplicate exists
    harness.assert_task_exists("Copy of Original task");

    // Verify original still exists
    harness.assert_task_exists("Original task");

    harness.save();
}

// ============================================================================
// Workflow 6: Delete and Undo
// ============================================================================

#[test]
fn test_workflow_delete_and_undo() {
    let mut harness = E2ETestHarness::new();

    // Create a task
    harness.quick_add("Task to delete");
    let task_id = harness.selected_task().unwrap().id;
    harness.assert_task_count(1);

    // Delete it
    harness.send(Message::Task(TaskMessage::Delete(task_id)));
    harness.assert_task_count(0);

    // Undo delete
    harness.send(Message::System(SystemMessage::Undo));
    harness.assert_task_count(1);
    harness.assert_task_exists("Task to delete");

    // Redo delete
    harness.send(Message::System(SystemMessage::Redo));
    harness.assert_task_count(0);

    // Undo again
    harness.send(Message::System(SystemMessage::Undo));
    harness.assert_task_count(1);

    harness.save();
}

// ============================================================================
// Workflow 7: View Switching
// ============================================================================

#[test]
fn test_workflow_view_switching() {
    let mut harness = E2ETestHarness::new();

    // Create some tasks
    harness.quick_add("Task 1");
    harness.quick_add("Task 2");
    harness.quick_add("Task 3");

    // Switch between views
    harness.navigate_to(ViewId::Today);
    assert_eq!(harness.model.current_view, ViewId::Today);

    harness.navigate_to(ViewId::Upcoming);
    assert_eq!(harness.model.current_view, ViewId::Upcoming);

    harness.navigate_to(ViewId::Projects);
    assert_eq!(harness.model.current_view, ViewId::Projects);

    harness.navigate_to(ViewId::Calendar);
    assert_eq!(harness.model.current_view, ViewId::Calendar);

    // Return to task list
    harness.navigate_to(ViewId::TaskList);
    assert_eq!(harness.model.current_view, ViewId::TaskList);

    harness.assert_task_count(3);
}

// ============================================================================
// Workflow 8: Multiple Task Operations
// ============================================================================

#[test]
fn test_workflow_multiple_task_operations() {
    let mut harness = E2ETestHarness::new();

    // Create 5 tasks
    for i in 1..=5 {
        harness.quick_add(&format!("Task {i}"));
    }

    harness.assert_task_count(5);

    // Complete every other task
    harness.send(Message::Navigation(NavigationMessage::First));

    for _ in 0..3 {
        harness.toggle_complete();
        harness.nav_down();
        harness.nav_down();
    }

    // Should have some completed tasks
    assert!(harness.task_count_with_status(TaskStatus::Done) > 0);

    // Save and reload
    harness.save();
    harness.reload().expect("reload");

    // Verify all tasks still exist
    harness.assert_task_count(5);
}

// ============================================================================
// Workflow 9: Task Creation with Quick-Add Parsing
// ============================================================================

#[test]
fn test_workflow_quick_add_parsing() {
    let mut harness = E2ETestHarness::new();

    // Test various quick-add formats
    harness.quick_add("Simple task");
    harness.assert_task_exists("Simple task");

    harness.quick_add("High priority task !high");
    let task = harness
        .model
        .tasks
        .values()
        .find(|t| t.title == "High priority task");
    assert_eq!(task.unwrap().priority, Priority::High);

    harness.quick_add("Tagged task #work #urgent");
    let task = harness
        .model
        .tasks
        .values()
        .find(|t| t.title == "Tagged task");
    assert!(task.unwrap().tags.contains(&"work".to_string()));
    assert!(task.unwrap().tags.contains(&"urgent".to_string()));

    harness.quick_add("Task with due date due:today");
    let task = harness
        .model
        .tasks
        .values()
        .find(|t| t.title == "Task with due date");
    assert!(task.unwrap().due_date.is_some());

    harness.assert_task_count(4);
    harness.save();
}

// ============================================================================
// Workflow 10: Page Navigation
// ============================================================================

#[test]
fn test_workflow_page_navigation() {
    let mut harness = E2ETestHarness::new();

    // Create enough tasks to test paging (15 tasks)
    for i in 1..=15 {
        harness.quick_add(&format!("Task {i:02}"));
    }

    harness.assert_task_count(15);

    // Ensure we have a valid first selection by navigating to first
    harness.send(Message::Navigation(NavigationMessage::First));
    let first_task = harness.selected_task().expect("should have selection");
    let first_title = first_task.title.clone();

    // Page down (moves 10 items)
    harness.send(Message::Navigation(NavigationMessage::PageDown));

    // Page up
    harness.send(Message::Navigation(NavigationMessage::PageUp));

    // Should be back at first task
    harness.assert_selected_title(&first_title);

    // Navigate to last
    harness.send(Message::Navigation(NavigationMessage::Last));

    // Verify we have some task selected (order may vary)
    assert!(harness.selected_task().is_some());

    harness.save();
}
