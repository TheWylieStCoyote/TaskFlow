# TaskFlow Future Features Roadmap

**Status:** Feature Catalog

**Goal:** Comprehensive roadmap of potential enhancements for TaskFlow

---

## Feature Categories

| Category | Features | Complexity Range |
|----------|----------|------------------|
| Workflow Automation | 4 phases | Medium → Very High |
| Sync & Integration | 5 features | Medium → High |
| UI Enhancements | 5 features | Low → Medium |
| Smart Features | 4 features | Medium → Very High |

---

# Workflow Automation

## Phase 1: Advanced Recurrence
**Complexity:** Medium | **Dependencies:** None

Extend recurrence with intervals, end dates, limits, and skip conditions.

**New:** `src/domain/task/recurrence_config.rs`
```rust
pub struct RecurrenceConfig {
    pub pattern: Recurrence,
    pub interval: u32,              // every N occurrences
    pub end_date: Option<NaiveDate>,
    pub max_occurrences: Option<u32>,
    pub occurrence_count: u32,
    pub skip_conditions: Vec<SkipCondition>,
}

pub enum SkipCondition {
    SkipWeekends,
    SkipWeekdays(Vec<Weekday>),
    SkipIfTag(String),
}
```

**Modify:** `src/domain/task/mod.rs`, `src/app/update/task.rs`

---

## Phase 2: Dependency Enforcement
**Complexity:** Medium-High | **Dependencies:** None

Block completion until dependencies done, auto-unblock dependents.

**Changes:**
- Block task completion if dependencies incomplete (show error)
- Auto-transition Blocked → Todo when dependencies complete
- Add `get_dependents()`, `get_dependency_chain()` helpers

**Modify:** `src/app/update/task.rs`, `src/app/model/hierarchy.rs`

---

## Phase 3: Status Workflows
**Complexity:** High | **Dependencies:** Phase 2

State machine transitions, required fields, auto-transitions.

**New:** `src/domain/workflow/mod.rs`
```rust
pub struct StatusWorkflow {
    pub transitions: HashMap<TaskStatus, Vec<TaskStatus>>,
    pub required_fields: HashMap<TaskStatus, Vec<RequiredField>>,
    pub auto_transitions: Vec<AutoTransition>,
}
```

---

## Phase 4: Rule/Trigger System
**Complexity:** Very High | **Dependencies:** Phase 3, Filter DSL

Event-based automation with filter DSL conditions.

**New:** `src/domain/automation/mod.rs`
- Triggers: TaskCreated, TaskCompleted, StatusChanged, DependenciesComplete
- Actions: SetStatus, SetPriority, AddTag, SetDueDate, CreateTask

---

# Sync & Integration

## Feature: Cloud Sync
**Complexity:** High | **Dependencies:** None

Sync tasks across devices via cloud storage.

**Options:**
- File-based sync (Dropbox, iCloud, Syncthing folder)
- WebDAV backend
- Custom sync server

**New Files:**
- `src/storage/backends/webdav.rs`
- `src/storage/sync/mod.rs` - conflict resolution

---

## Feature: CalDAV Integration
**Complexity:** High | **Dependencies:** None

Two-way sync with calendar applications.

**Capabilities:**
- Push tasks as VTODO items
- Pull calendar events (already have ICS import)
- Sync due dates, reminders

**New:** `src/storage/caldav/mod.rs`

---

## Feature: Git Integration
**Complexity:** Medium | **Dependencies:** None

Link tasks to commits, branches, PRs.

**Capabilities:**
- Extract TODOs from code (already have GitTodos view)
- Link tasks to branches
- Auto-complete on merge
- Show commit history in task detail

**Modify:** `src/domain/task/mod.rs` (add git_ref field)

---

## Feature: Webhook System
**Complexity:** Medium | **Dependencies:** None

HTTP callbacks on task events.

**Triggers:** Task created, completed, status changed, due soon
**Payload:** JSON with task details

**New:** `src/integration/webhooks.rs`

---

## Feature: REST API
**Complexity:** High | **Dependencies:** None

HTTP API for external tools.

**Endpoints:**
- `GET/POST/PUT/DELETE /tasks`
- `GET/POST /projects`
- `POST /tasks/:id/complete`

**New:** `src/bin/taskflow-server/` or feature flag in main binary

---

# UI Enhancements

## Feature: Custom Views
**Complexity:** Medium | **Dependencies:** Filter DSL

User-defined filter+sort+column combinations.

**Capabilities:**
- Save any filter as a custom view
- Choose visible columns
- Set default sort order
- Pin to sidebar

**New:** `src/domain/custom_view.rs`
**Modify:** `src/ui/sidebar.rs`

---

## Feature: Split View
**Complexity:** Medium | **Dependencies:** None

Two panels side-by-side.

**Use cases:**
- Task list + task detail
- Two different filtered views
- Calendar + task list

**Modify:** `src/ui/layout.rs`, `src/app/model/mod.rs`

---

## Feature: Batch Editing
**Complexity:** Low-Medium | **Dependencies:** None

Edit multiple selected tasks at once.

**Operations:**
- Set priority for all selected
- Move to project
- Add/remove tags
- Set due date

**Modify:** `src/app/update/task.rs` (extend multi-select)

---

## Feature: Command Palette
**Complexity:** Medium | **Dependencies:** None

Fuzzy-searchable command launcher (like VS Code Ctrl+Shift+P).

**Features:**
- Search all commands by name
- Show keyboard shortcut
- Recent commands
- Filter by category

**New:** `src/ui/command_palette.rs`

---

## Feature: Dashboard Widgets
**Complexity:** Medium | **Dependencies:** None

Configurable dashboard layout.

**Widgets:**
- Upcoming tasks
- Overdue count
- Completion streak
- Time tracked today
- Project progress bars

**Modify:** `src/ui/view/dashboard.rs`, add widget config

---

# Smart Features

## Feature: Natural Language Input
**Complexity:** Medium | **Dependencies:** None

Parse task titles for dates, priorities, tags.

**Examples:**
- "Call mom tomorrow at 3pm" → due: tomorrow, time: 15:00
- "Buy groceries #shopping !high" → tag: shopping, priority: high
- "Review PR next monday" → due: next Monday

**New:** `src/domain/natural_language.rs`

---

## Feature: Smart Scheduling
**Complexity:** High | **Dependencies:** Analytics

Suggest optimal times for tasks based on patterns.

**Inputs:**
- Historical completion times by tag/type
- Calendar availability
- Energy patterns (morning vs evening tasks)

**New:** `src/domain/scheduler.rs`

---

## Feature: Workload Balancing
**Complexity:** Medium | **Dependencies:** None

Alert when overcommitted.

**Checks:**
- Total estimated time vs available hours
- Due date clustering
- Blocked task chains

**New:** `src/domain/workload.rs`
**Modify:** `src/ui/view/dashboard.rs`

---

## Feature: AI Task Suggestions
**Complexity:** Very High | **Dependencies:** External API

LLM-powered task assistance.

**Capabilities:**
- Break down large tasks into subtasks
- Suggest next actions
- Summarize project status
- Generate task descriptions

**New:** `src/integration/ai.rs` (OpenAI/Claude API)

---

# Implementation Priority

Recommended order based on value/effort ratio:

```
HIGH VALUE, LOWER EFFORT:
1. Batch Editing (UI)
2. Natural Language Input (Smart)
3. Dependency Enforcement (Workflow)
4. Custom Views (UI)

HIGH VALUE, HIGHER EFFORT:
5. Advanced Recurrence (Workflow)
6. Command Palette (UI)
7. Status Workflows (Workflow)
8. Cloud Sync (Integration)

FUTURE:
9. Rule/Trigger System (Workflow)
10. REST API (Integration)
11. Smart Scheduling (Smart)
12. AI Suggestions (Smart)
```

---

# Key Files Reference

| Area | Files |
|------|-------|
| Task Domain | `src/domain/task/mod.rs`, `src/domain/task/recurrence.rs` |
| Task Handlers | `src/app/update/task.rs` |
| Model/State | `src/app/model/mod.rs`, `src/app/model/hierarchy.rs` |
| Filter DSL | `src/domain/filter_dsl/` |
| Views | `src/ui/view/` |
| Storage | `src/storage/backends/` |
