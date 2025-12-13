# Workflow & Automation Features - Analysis

## Overview

Analysis of workflow and automation features for TaskFlow, including template workflows, advanced recurrence, dependency enforcement, status workflows, and webhook triggers.

---

## Current Implementation Status

| Feature | Status | Location |
|---------|--------|----------|
| Task Templates | Basic (single task) | `src/app/templates.rs` |
| Recurrence | ~40% implemented | `src/domain/task/recurrence.rs` |
| Dependencies | ~30% implemented | `src/domain/task/mod.rs`, `src/app/model/hierarchy.rs` |
| Status | Fixed enum (5 states) | `src/domain/task/status.rs` |
| Webhooks | None | N/A |

---

## Feature 1: Template Workflows

**Goal**: Create task templates that spawn multiple linked tasks

### What Exists
- `TaskTemplate` struct with priority, tags, description, relative due dates
- `TemplateManager` for storing templates
- Builder pattern for configuration

### What Needs to Be Added

```rust
pub struct WorkflowTemplate {
    pub name: String,
    pub tasks: Vec<TaskSpec>,
}

pub struct TaskSpec {
    pub title: String,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub parent_index: Option<usize>,    // Index within template
    pub dependencies: Vec<usize>,        // Indices of blocking tasks
    pub estimated_minutes: Option<u32>,
}
```

### New Message Type
```rust
TaskMessage::SpawnWorkflow(WorkflowTemplate)
```

### Complexity: Medium (20-30 hours)
- Extend template system: 4-6 hours
- Add message handler: 6-8 hours
- UI for template picker/builder: 8-10 hours

---

## Feature 2: Advanced Recurrence

**Goal**: Skip conditions, end dates, intervals ("every 2 weeks")

### What Exists
- `Recurrence` enum: Daily, Weekly(days), Monthly(day), Yearly
- `create_next_recurring_task()` with 40+ test cases
- Edge case handling (Feb 29, month boundaries)

### What Needs to Be Added

```rust
pub struct RecurrenceConfig {
    pub rule: RecurrenceRule,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,        // NEW
    pub max_occurrences: Option<u32>,       // NEW
    pub skip_condition: Option<SkipCondition>, // NEW
}

pub enum RecurrenceRule {
    Daily { skip_weekends: bool },
    Weekly { days: Vec<Weekday>, interval: u32 },  // every N weeks
    Monthly { day: u32, interval: u32 },           // every N months
    Yearly { month: u32, day: u32 },
}

pub enum SkipCondition {
    Weekends,
    Holidays(Vec<NaiveDate>),
    Custom(String),  // Future: expression-based
}
```

### Complexity: Medium-High (25-35 hours)
- Extend types and Task field: 4-6 hours
- Implement interval math: 6-8 hours
- End date/max occurrences: 3-4 hours
- Skip condition evaluation: 5-7 hours
- Tests (expand from 40+): 3-5 hours

---

## Feature 3: Dependency Enforcement

**Goal**: Block completion when dependencies incomplete, auto-transition Blocked → Todo

### What Exists
- `Task.dependencies: Vec<TaskId>` field
- `is_task_blocked(task_id)` in `src/app/model/hierarchy.rs`
- `incomplete_dependency_count(task_id)`
- `TaskStatus::Blocked` variant
- UI shows `[!]` for blocked tasks

### What's Missing
- No enforcement in toggle completion
- No auto-transition when dependencies resolve
- No circular dependency prevention

### What Needs to Be Added

```rust
// In task completion handler:
fn handle_toggle_complete(model: &mut Model, task_id: TaskId) -> Result<()> {
    if model.is_task_blocked(&task_id) {
        return Err("Cannot complete: task has incomplete dependencies");
    }
    // ... complete task

    // Auto-unblock dependent tasks
    for dependent_id in model.find_dependents(&task_id) {
        if !model.is_task_blocked(&dependent_id) {
            model.set_status(dependent_id, TaskStatus::Todo);
        }
    }
}

// Cycle detection
fn validate_dependency(from: TaskId, to: TaskId) -> Result<()> {
    // Prevent A -> B -> A cycles
}
```

### New Messages
```rust
TaskMessage::AddDependency(task_id, blocker_id)
TaskMessage::RemoveDependency(task_id, blocker_id)
```

### Complexity: Medium (15-20 hours)
- Completion blocking logic: 3-4 hours
- Cycle detection: 3-4 hours
- Auto-transition on dependency completion: 3-4 hours
- UI/messaging: 2-3 hours

---

## Feature 4: Status Workflows

**Goal**: Configurable status workflows with required fields

### What Exists
- Fixed `TaskStatus` enum: Todo, InProgress, Blocked, Done, Cancelled
- `TaskMessage::SetStatus` for manual changes
- No transition rules or required fields

### What Needs to Be Added

```rust
pub struct Workflow {
    pub name: String,
    pub states: Vec<WorkflowState>,
    pub initial_state: String,
    pub transitions: Vec<WorkflowTransition>,
}

pub struct WorkflowState {
    pub name: String,
    pub symbol: String,
    pub color: Option<String>,
    pub is_terminal: bool,
    pub required_fields: Vec<RequiredField>,
}

pub struct WorkflowTransition {
    pub from: String,
    pub to: String,
    pub condition: Option<TransitionCondition>,
}

pub struct RequiredField {
    pub field: String,  // "due_date", "estimate", "assignee"
    pub message: String,
}
```

### Configuration (TOML)
```toml
[[workflows.kanban]]
name = "Kanban"
states = [
    { name = "Backlog", symbol = "○" },
    { name = "Ready", symbol = "◐" },
    { name = "In Progress", symbol = "●", required_fields = ["estimate"] },
    { name = "Review", symbol = "◑", required_fields = ["reviewer"] },
    { name = "Done", symbol = "✓", is_terminal = true },
]
```

### Complexity: High (40-50 hours)
- Core workflow data structures: 4-6 hours
- Workflow parser/loader: 6-8 hours
- State transition validation: 8-10 hours
- Integration with Task/Model: 6-8 hours
- UI components: 8-10 hours
- Built-in workflow templates: 4-6 hours

---

## Feature 5: Webhook Triggers

**Goal**: Fire webhooks on task events (completed, created, overdue)

### What Exists
- TEA message system for all task operations
- Update function routes all messages
- No HTTP client in dependencies

### What Needs to Be Added

```rust
pub struct Webhook {
    pub id: WebhookId,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub secret: Option<String>,  // HMAC signing
    pub active: bool,
    pub headers: HashMap<String, String>,
}

pub enum WebhookEvent {
    TaskCreated,
    TaskCompleted,
    TaskDeleted,
    TaskStatusChanged,
    TaskOverdue,
}

pub struct WebhookPayload {
    pub event: WebhookEvent,
    pub timestamp: DateTime<Utc>,
    pub task_id: TaskId,
    pub task: Task,
    pub previous_state: Option<Task>,
}
```

### New Dependencies
```toml
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }
```

### Complexity: High-Very High (50-70 hours)
- Core webhook types: 6-8 hours
- HTTP client with retry logic: 8-10 hours
- Event dispatcher: 6-8 hours
- Integration with update handlers: 8-10 hours
- Webhook configuration UI: 6-8 hours
- Payload signing: 4-6 hours

### Alternative: Script Execution
Instead of HTTP webhooks, execute local scripts on events:
```bash
# ~/.config/taskflow/hooks/on_task_complete.sh
#!/bin/bash
echo "$TASK_TITLE completed" | notify-send
```
**Effort**: 25-30 hours (significantly simpler)

---

## Implementation Priority

| Feature | Effort | Value | Risk | Priority |
|---------|--------|-------|------|----------|
| Dependency Enforcement | 15-20h | High | Low | 1st |
| Advanced Recurrence | 25-35h | High | Low | 2nd |
| Template Workflows | 20-30h | Medium-High | Low | 3rd |
| Status Workflows | 40-50h | Medium | Medium | 4th |
| Webhooks | 50-70h | Low (TUI app) | High | 5th (optional) |

**Total Estimated Effort**: 150-205 hours (full implementation)

**Recommended MVP**: Dependency Enforcement + Advanced Recurrence (40-55 hours)

---

## Key Files to Modify

```
src/domain/task/recurrence.rs    # Extend recurrence types
src/domain/task/mod.rs           # Add workflow fields
src/app/update/task.rs           # Dependency enforcement
src/app/templates.rs             # Workflow templates
src/config/workflows.rs          # NEW: Workflow config
src/webhooks/                    # NEW: Webhook module
```

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
- [DATA_BACKUP_FEATURES.md](DATA_BACKUP_FEATURES.md) - Related: Audit log for tracking
