# TaskFlow Features Roadmap

**Status:** Updated Dec 2024

---

## Implementation Status Summary

| Category | Complete | Partial | Not Started |
|----------|----------|---------|-------------|
| Already Implemented (undocumented) | 15 | - | - |
| Workflow Automation | 0 | 2 | 2 |
| Sync & Integration | 1 | 1 | 3 |
| UI Enhancements | 2 | 2 | 1 |
| Smart Features | 0 | 0 | 4 |

---

# Already Implemented Features

These major features are complete but were not previously tracked:

## Core Task Management
- **Task Hierarchy & Subtasks** - Full parent-child relationships with cascade completion
- **Task Chains** - `next_task_id` field with auto-scheduling on completion
- **Task Dependencies** - `dependencies: Vec<TaskId>` field (enforcement not complete)
- **Snooze System** - `snooze_until` field to hide tasks until a date
- **Undo/Redo System** - Full action history with `modify_task_with_undo()` helper

## Views & UI
- **Filter DSL** - Advanced query language with 30+ fields, boolean operators, range syntax
- **Saved Filters/Custom Views** - Persistent filter+sort combinations with icons
- **Kanban Board** - 4-column status-based view with scrolling
- **Eisenhower Matrix** - 2x2 urgency/importance grid
- **Timeline/Gantt** - Date-based task visualization with zoom
- **Heatmap** - Activity visualization
- **Network Graph** - Task dependency visualization
- **Calendar View** - Monthly calendar with task dots
- **Burndown Chart** - Sprint progress visualization
- **Dashboard** - Statistics and overview widgets

## Productivity Features
- **Goals & Key Results** - OKR tracking with quarterly planning
- **Habits** - Recurring habit tracking with streaks
- **Time Tracking** - Estimated vs actual minutes with time entries
- **Work Logs** - Detailed work session logging
- **Pomodoro Timer** - Full state management for focus sessions
- **Analytics** - Velocity, completion trends, productivity metrics

## Data Management
- **Multiple Storage Backends** - JSON, YAML, SQLite, Markdown
- **Import** - CSV tasks, ICS calendar events
- **Export** - CSV, ICS, DOT (Graphviz), Mermaid diagrams, HTML
- **Duplicate Detection** - Jaro-Winkler similarity with project scoping

---

# Workflow Automation

## Phase 1: Advanced Recurrence - ⚠️ 40% COMPLETE
**Complexity:** Medium | **Dependencies:** None

**Implemented:**
- Basic `Recurrence` enum (Daily, Weekly, Monthly, Yearly)
- Recurring task creation on completion

**Not Implemented:**
```rust
pub struct RecurrenceConfig {
    pub pattern: Recurrence,
    pub interval: u32,              // every N occurrences - NOT DONE
    pub end_date: Option<NaiveDate>, // - NOT DONE
    pub max_occurrences: Option<u32>, // - NOT DONE
    pub occurrence_count: u32,       // - NOT DONE
    pub skip_conditions: Vec<SkipCondition>, // - NOT DONE
}
```

---

## Phase 2: Dependency Enforcement - ⚠️ 30% COMPLETE
**Complexity:** Medium-High | **Dependencies:** None

**Implemented:**
- `dependencies: Vec<TaskId>` field on Task
- `TaskStatus::Blocked` variant
- Filter DSL `has:dependencies` query

**Not Implemented:**
- Block task completion if dependencies incomplete
- Auto-transition Blocked → Todo when dependencies complete
- `get_dependents()`, `get_dependency_chain()` helpers

---

## Phase 3: Status Workflows - ❌ NOT STARTED
**Complexity:** High | **Dependencies:** Phase 2

State machine transitions, required fields, auto-transitions.

```rust
pub struct StatusWorkflow {
    pub transitions: HashMap<TaskStatus, Vec<TaskStatus>>,
    pub required_fields: HashMap<TaskStatus, Vec<RequiredField>>,
    pub auto_transitions: Vec<AutoTransition>,
}
```

---

## Phase 4: Rule/Trigger System - ❌ NOT STARTED
**Complexity:** Very High | **Dependencies:** Phase 3, Filter DSL

Event-based automation with filter DSL conditions.

---

# Sync & Integration

## Git Integration - ✅ 100% COMPLETE
**Complexity:** Medium

**Fully Implemented:**
- `GitRef` struct for branch linking (`src/domain/git/mod.rs`)
- `GitCommit` struct for commit history
- Auto-detection of task-branch associations
- Git TODO extraction from code comments
- Merge status tracking (Active, Merged, Deleted)
- CLI `git_todos` command
- GitTodos sidebar view

---

## Cloud Sync - ⚠️ 20% COMPLETE
**Complexity:** High

**Implemented:**
- Storage abstraction layer (multiple backends)
- Repository pattern for data access

**Not Implemented:**
- WebDAV backend
- Conflict resolution
- Remote sync server

---

## CalDAV Integration - ❌ NOT STARTED
**Complexity:** High

Two-way sync with calendar applications.

---

## Webhook System - ❌ NOT STARTED
**Complexity:** Medium

HTTP callbacks on task events.

---

## REST API - ❌ NOT STARTED
**Complexity:** High

HTTP API for external tools.

---

# UI Enhancements

## Custom Views - ✅ 100% COMPLETE
**Complexity:** Medium | **Dependencies:** Filter DSL

**Fully Implemented:**
- `SavedFilter` struct with name, icon, filter, sort
- Saved filter picker UI
- Persistence in storage
- Sidebar integration

---

## Batch Editing - ⚠️ 40% COMPLETE
**Complexity:** Low-Medium

**Implemented:**
- Multi-select mode toggle
- Toggle individual task selection
- Select all / clear selection
- Bulk delete
- Bulk move to project

**Not Implemented:**
- Bulk set priority
- Bulk add/remove tags
- Bulk set due date

---

## Dashboard Widgets - ⚠️ 80% COMPLETE
**Complexity:** Medium

**Implemented:**
- Upcoming tasks widget
- Overdue count
- Completion stats
- Project progress

**Not Implemented:**
- Widget customization/rearrangement
- Completion streak display
- Time tracked today widget

---

## Command Palette - ❌ NOT STARTED
**Complexity:** Medium

Fuzzy-searchable command launcher (like VS Code Ctrl+Shift+P).

---

## Split View - ❌ NOT STARTED
**Complexity:** Medium

Two panels side-by-side.

---

# Smart Features

## Natural Language Input - ❌ NOT STARTED
**Complexity:** Medium

Parse task titles for dates, priorities, tags.

Examples:
- "Call mom tomorrow at 3pm" → due: tomorrow, time: 15:00
- "Buy groceries #shopping !high" → tag: shopping, priority: high

---

## Smart Scheduling - ❌ NOT STARTED
**Complexity:** High

Suggest optimal times for tasks based on patterns.

---

## Workload Balancing - ❌ NOT STARTED
**Complexity:** Medium

Alert when overcommitted.

---

# Implementation Priority

## Recommended Next Steps

**Quick Wins (Low Effort, High Value):**
1. Batch Editing - Add bulk priority/tag/date operations
2. Natural Language Input - Parse dates from titles

**Medium Effort:**
3. Dependency Enforcement - Add completion blocking
4. Command Palette - Fuzzy command search
5. Advanced Recurrence - Add intervals and limits

**High Effort (Future):**
6. Status Workflows
7. Cloud Sync
8. REST API

---

# Key Files Reference

| Area | Files |
|------|-------|
| Task Domain | `src/domain/task/mod.rs`, `src/domain/task/recurrence.rs` |
| Git Integration | `src/domain/git/mod.rs`, `operations.rs`, `matching.rs` |
| Filter DSL | `src/domain/filter_dsl/` |
| Saved Filters | `src/domain/filter.rs`, `src/ui/components/saved_filter_picker.rs` |
| Task Handlers | `src/app/update/task.rs` |
| Multi-Select | `src/app/update/ui/multi_select.rs` |
| Model/State | `src/app/model/mod.rs`, `src/app/model/hierarchy.rs` |
| Views | `src/ui/view/`, `src/ui/components/` |
| Storage | `src/storage/backends/` |
