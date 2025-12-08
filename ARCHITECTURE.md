# TaskFlow Architecture

This document describes the high-level architecture of TaskFlow, a terminal-based task management application built with Rust.

## Overview

TaskFlow follows **The Elm Architecture (TEA)**, a unidirectional data flow pattern that ensures predictable state management and makes the application easy to test and reason about.

```
┌─────────────────────────────────────────────────────────────────┐
│                         Event Loop                              │
│  ┌─────────┐     ┌──────────┐     ┌────────┐     ┌─────────┐   │
│  │  View   │────▶│  Input   │────▶│ Update │────▶│  Model  │   │
│  │  (UI)   │     │ (Events) │     │  (Msg) │     │ (State) │   │
│  └─────────┘     └──────────┘     └────────┘     └─────────┘   │
│       ▲                                               │         │
│       └───────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── main.rs          # Entry point, event loop, terminal setup
├── lib.rs           # Public API, crate documentation
├── app/             # Application logic (TEA)
│   ├── model/       # Application state
│   ├── update/      # Message handlers
│   ├── message.rs   # Message type definitions
│   ├── undo.rs      # Undo/redo stack
│   ├── macros.rs    # Keyboard macro recording
│   ├── templates.rs # Task templates
│   └── quick_add.rs # Natural language task parsing
├── domain/          # Core business entities
│   ├── task.rs      # Task entity
│   ├── project.rs   # Project entity
│   ├── time_entry.rs # Time tracking
│   ├── work_log.rs  # Work log entries
│   ├── filter.rs    # Filter criteria
│   └── pomodoro.rs  # Pomodoro timer
├── ui/              # Terminal UI (Ratatui)
│   ├── view.rs      # Main render function
│   └── components/  # Reusable widgets
├── storage/         # Persistence layer
│   ├── backends/    # Storage implementations
│   ├── import.rs    # CSV/ICS import
│   └── export.rs    # CSV/ICS/HTML export
└── config/          # Configuration
    ├── settings.rs  # General settings
    ├── keybindings.rs # Key mappings
    └── theme.rs     # Color themes
```

## Core Components

### 1. Model (`app/model/`)

The **Model** holds all application state in a single struct. Key fields include:

| Field | Description |
|-------|-------------|
| `tasks` | HashMap of all tasks by ID |
| `projects` | HashMap of all projects by ID |
| `visible_tasks` | Filtered/sorted task IDs for display |
| `selected_index` | Currently selected task |
| `current_view` | Active view (List, Today, Calendar, etc.) |
| `input_mode` | Whether in editing mode |
| `undo_stack` | History for undo/redo |

**Submodules:**
- `filtering.rs` - Task filtering, sorting, search
- `hierarchy.rs` - Subtask/parent relationships
- `time_tracking.rs` - Time entry management
- `storage.rs` - Backend sync helpers

### 2. Message (`app/message.rs`)

**Messages** are immutable events that describe what happened. They are categorized:

| Category | Examples |
|----------|----------|
| `NavigationMessage` | `Up`, `Down`, `GoToView(ViewId)` |
| `TaskMessage` | `Create(String)`, `Delete`, `ToggleComplete` |
| `UiMessage` | `StartEditing`, `Char(char)`, `Submit` |
| `SystemMessage` | `Save`, `Quit`, `Undo`, `Redo` |
| `TimeMessage` | `StartTracking`, `StopTracking` |
| `PomodoroMessage` | `Start`, `Pause`, `Skip` |

### 3. Update (`app/update/`)

The **Update** function is pure: given current state and a message, it produces new state.

```rust
pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Navigation(msg) => navigation::handle_navigation(model, msg),
        Message::Task(msg) => task::handle_task(model, msg),
        Message::Ui(msg) => ui::handle_ui(model, msg),
        Message::System(msg) => system::handle_system(model, msg),
        Message::Time(msg) => time::handle_time(model, msg),
        Message::Pomodoro(msg) => time::handle_pomodoro(model, msg),
        Message::None => {}
    }
}
```

**Submodules:**
- `navigation.rs` - Movement, view switching
- `task.rs` - Task CRUD, completion, priority
- `time.rs` - Time tracking, Pomodoro
- `ui/` - Input handling, dialogs, multi-select
- `system.rs` - Save, quit, import/export

### 4. View (`ui/`)

The **View** renders the UI based on current state. It never mutates state directly.

```rust
pub fn view(model: &Model, frame: &mut Frame<'_>, theme: &Theme) {
    render_header(frame, chunks[0], theme);
    render_content(model, frame, chunks[1], theme);
    render_footer(model, frame, chunks[2], theme);
    // Render popups if active...
}
```

**UI Components** (`ui/components/`):
- `task_list.rs` - Main task list view
- `sidebar.rs` - Project/view navigation
- `calendar.rs` - Month calendar view
- `dashboard.rs` - Overview with charts
- `kanban.rs` - Kanban board view
- `help.rs` - Keybindings popup
- Various editors and pickers

## Data Flow

### User Input → Message → Update → Render

1. **Input**: User presses a key (e.g., `j`)
2. **Keybinding lookup**: Key maps to action (e.g., `MoveDown`)
3. **Message creation**: Action becomes `Message::Navigation(Down)`
4. **Update**: `handle_navigation()` increments `selected_index`
5. **Render**: `view()` re-renders with new selection highlighted

### Example: Creating a Task

```
User types 'a' (add task)
    ↓
Message::Ui(StartEditing(InputTarget::Task))
    ↓
Model.input_mode = Editing, input_buffer = ""
    ↓
View renders input dialog
    ↓
User types "Buy groceries due:tomorrow #shopping"
    ↓
Message::Ui(Submit)
    ↓
Quick-add parser extracts: title, due date, tags
    ↓
Task created, added to model.tasks
    ↓
Undo action recorded
    ↓
Storage backend saves changes
    ↓
View re-renders task list
```

## Storage Layer

The storage layer uses a trait-based design for pluggable backends:

```rust
pub trait StorageBackend: Send + Sync {
    fn create_task(&mut self, task: &Task) -> StorageResult<()>;
    fn update_task(&mut self, task: &Task) -> StorageResult<()>;
    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()>;
    fn list_tasks(&self) -> StorageResult<Vec<Task>>;
    // ... projects, time entries, etc.
}
```

**Available Backends:**
| Backend | File Format | Use Case |
|---------|------------|----------|
| JSON | `.json` | Default, fast, compact |
| YAML | `.yaml` | Human-readable |
| SQLite | `.db` | Large datasets |
| Markdown | `.md` files | Git-friendly |

## Undo/Redo System

The undo system captures state changes as reversible actions:

```rust
pub enum UndoAction {
    CreateTask(TaskId),
    DeleteTask(Task),
    UpdateTask { before: Task, after: Task },
    // ... more actions
}
```

Each undoable operation:
1. Captures the "before" state
2. Performs the action
3. Pushes an `UndoAction` to the stack
4. Clears the redo stack

## Key Design Decisions

### Why TEA?

- **Predictability**: All state changes go through `update()`
- **Testability**: Pure functions are easy to test
- **Debugging**: Can inspect any state transition
- **Time-travel**: Undo/redo is straightforward

### Why Single Model Struct?

- **Simplicity**: One place for all state
- **Consistency**: No scattered mutable state
- **Serialization**: Easy to save/restore

### Why Trait-based Storage?

- **Flexibility**: Swap backends without changing app logic
- **Testing**: Use in-memory backend for tests
- **Extensibility**: Add new backends easily

## Testing Strategy

- **Unit tests**: Domain logic, parsing, filtering
- **Integration tests**: Message → state transitions
- **UI tests**: Widget rendering (buffer snapshots)
- **Storage tests**: Each backend implementation

Tests are colocated with modules or in `tests/` submodules.

## Configuration

Configuration files live in `~/.config/taskflow/`:

```
~/.config/taskflow/
├── config.toml        # Backend, data dir, preferences
├── keybindings.toml   # Custom key mappings
└── themes/
    └── custom.toml    # Color themes
```

All configuration has sensible defaults.

## Performance Considerations

- **Lazy loading**: Only load visible tasks
- **Incremental updates**: Refresh only what changed
- **Efficient filtering**: Pre-computed visible task list
- **Background saves**: Non-blocking persistence

## Future Considerations

- **Structured logging**: Replace println with tracing
- **Async storage**: Non-blocking backend operations
- **Plugin system**: Custom views and commands
- **Sync**: Cloud/network synchronization
