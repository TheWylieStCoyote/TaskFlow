# ADR-0001: Use The Elm Architecture

## Status

Accepted

## Context

TaskFlow is a terminal-based task management application that needs to:
- Handle complex user interactions (navigation, editing, dialogs)
- Maintain consistent state across multiple views
- Support undo/redo operations
- Be easily testable
- Scale to handle new features without tangled state

Traditional approaches like MVC or direct state mutation can lead to:
- Difficult-to-track state changes
- Complex interaction between components
- Hard-to-reproduce bugs
- Challenging undo/redo implementation

## Decision

We adopt The Elm Architecture (TEA) as the core application architecture:

```
Model → Update → View → Model
```

**Model**: A single struct containing all application state
**Message**: Immutable events describing what happened
**Update**: Pure function that transforms state based on messages
**View**: Pure function that renders UI based on current state

All state changes flow through the `update()` function, ensuring:
- Single source of truth for state
- Predictable state transitions
- Easy debugging (log messages to see all changes)
- Natural fit for undo/redo (capture state or inverse operations)

## Consequences

### Positive

- **Predictability**: Every state change is explicit and traceable
- **Testability**: Update handlers are pure functions, easily unit tested
- **Undo/Redo**: Straightforward to implement (capture messages or state snapshots)
- **Debugging**: Can log all messages to understand behavior
- **Refactoring**: Adding new features follows a consistent pattern

### Negative

- **Boilerplate**: Every new feature requires Message variants and handlers
- **Learning curve**: Developers unfamiliar with TEA need time to adapt
- **Single Model**: As the app grows, the Model struct becomes large
- **Performance**: Full re-render on every message (mitigated by terminal efficiency)

### Mitigations

- Message enum is split into categories (`NavigationMessage`, `TaskMessage`, etc.)
- Model is split into logical submodules (`model/filtering.rs`, `model/hierarchy.rs`)
- Update handlers are organized by domain (`update/task.rs`, `update/ui/`)
