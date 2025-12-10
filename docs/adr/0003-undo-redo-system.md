# ADR-0003: Undo/Redo System Design

## Status

Accepted

## Context

TaskFlow needs to support undo/redo operations for user-friendliness. Users should be able to:
- Undo accidental deletions
- Undo bulk operations
- Redo previously undone actions
- Have a reasonable undo history depth

Approaches considered:
1. **State snapshots**: Store complete state before each change
2. **Command pattern**: Store operations with inverse operations
3. **Event sourcing**: Store all events, replay to reconstruct state

## Decision

We use a hybrid approach combining command pattern with selective state capture:

```rust
pub enum UndoAction {
    // Task operations - store affected data
    CreateTask(TaskId),                        // Just need ID to delete
    DeleteTask(Task),                          // Store full task to restore
    UpdateTask { before: Task, after: Task },  // Store both states

    // Bulk operations
    BulkDelete(Vec<Task>),
    BulkUpdate { before: Vec<Task>, after: Vec<Task> },

    // Project operations
    CreateProject(ProjectId),
    DeleteProject(Project),
    UpdateProject { before: Project, after: Project },

    // Time tracking
    CreateTimeEntry(TimeEntryId),
    DeleteTimeEntry(TimeEntry),

    // Composite actions
    Composite(Vec<UndoAction>),
}
```

The undo stack stores `UndoAction`s, and redo stack stores previously undone actions:

```rust
pub struct UndoStack {
    undo: Vec<UndoAction>,
    redo: Vec<UndoAction>,
    max_size: usize,
}
```

When an action is undone:
1. Pop from undo stack
2. Apply inverse operation
3. Push to redo stack

When an action is redone:
1. Pop from redo stack
2. Apply original operation
3. Push to undo stack

New actions clear the redo stack.

## Consequences

### Positive

- **Memory efficient**: Only stores changed data, not full snapshots
- **Fast**: O(1) undo/redo operations
- **Granular**: Each logical operation is one undo step
- **Composable**: Bulk operations group multiple changes
- **Intuitive**: Matches user mental model

### Negative

- **Complexity**: Each operation needs undo logic
- **Maintenance**: New features must implement undo support
- **Storage sync**: Must coordinate with storage backend
- **Edge cases**: Some operations (time tracking) need special handling

### Design Decisions

1. **Max stack size**: Default 100 actions to limit memory
2. **Composite actions**: Bulk operations group as single undo step
3. **Time tracking**: Stop active timer before undo that affects it
4. **Storage**: Undo triggers storage update (eventual consistency)
5. **View state**: View/navigation changes are NOT undoable
