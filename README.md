# TaskFlow

A terminal-based project management application built with Rust.

TaskFlow provides a fast, keyboard-driven interface for managing tasks, projects, and time tracking—all from your terminal.

## Features

- **Task Management**: Create, edit, and track tasks with priorities, due dates, status, and subtasks
- **Project Organization**: Group related tasks under projects with sidebar navigation
- **Tagging System**: Categorize tasks with flexible tags
- **Time Tracking**: Track time spent on tasks with start/stop timer
- **Search & Filter**: Search tasks by title or tags, filter by view (Today, Upcoming, Projects)
- **Sorting**: Sort tasks by priority, due date, title, status, or creation date
- **Undo Support**: Undo task and project operations with `u` or `Ctrl+Z`
- **Vim-style Navigation**: Fast keyboard-driven interface
- **Multiple Storage Backends**: Save data as JSON, YAML, SQLite, or Markdown
- **Customizable**: Themes, keybindings, and settings via config files

## Installation

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd taskflow

# Build and run
cargo build --release
./target/release/taskflow
```

### Requirements

- Rust 1.70 or later
- A terminal with UTF-8 support

### Shell Completions

TaskFlow supports shell completion for Bash, Zsh, and Fish. Generate and install completions:

**Bash:**
```bash
# Generate and install
taskflow completion bash > ~/.local/share/bash-completion/completions/taskflow

# Or system-wide (requires sudo)
sudo taskflow completion bash > /etc/bash_completion.d/taskflow

# Reload (or restart terminal)
source ~/.local/share/bash-completion/completions/taskflow
```

**Zsh:**
```bash
# Create completions directory if needed
mkdir -p ~/.zsh/completions

# Generate completions
taskflow completion zsh > ~/.zsh/completions/_taskflow

# Add to ~/.zshrc (if not already present):
# fpath=(~/.zsh/completions $fpath)
# autoload -Uz compinit && compinit

# Reload
source ~/.zshrc
```

**Fish:**
```bash
# Generate and install (Fish loads automatically)
taskflow completion fish > ~/.config/fish/completions/taskflow.fish
```

## Quick Start

```bash
# Run TaskFlow with default settings (JSON storage)
cargo run

# Run with a specific backend
cargo run -- --backend yaml --data ~/tasks.yaml
cargo run -- --backend sqlite --data ~/tasks.db
cargo run -- --backend markdown --data ~/tasks/

# Run with demo data
cargo run -- --demo
```

## Usage

### Interface Overview

```
┌─────────────────────────────────────────────────────────────┐
│ TaskFlow - Project Management TUI                           │
├──────────────┬──────────────────────────────────────────────┤
│ All Tasks    │ !!!! [!] Review and fix bugs       [12/10]   │
│ Today        │ !!!  [~] Create TEA architecture   #rust     │
│ Upcoming     │ !!   [ ] Build task list UI                  │
│              │ !!   [ ] Add storage backends                │
│ ── Projects ─│ !    [ ] Implement keybinding config         │
│ Backend      │ !    [ ] Add theme support                   │
│ Frontend     │      [ ] Write documentation                 │
│              │      [x] Set up project structure            │
├──────────────┴──────────────────────────────────────────────┤
│ 7 tasks (2 completed) | hiding completed | Press ? for help │
└─────────────────────────────────────────────────────────────┘
```

### Task Display

Each task shows:
- **Priority indicator** (left): `!!!!` (urgent), `!!!` (high), `!!` (medium), `!` (low), or blank (none)
- **Status symbol**: `[ ]` (todo), `[~]` (in progress), `[!]` (blocked), `[x]` (done), `[-]` (cancelled)
- **Task title**
- **Tags**: Displayed as `#tagname` after the title
- **Due date** (if set): Shown in brackets, colored red if overdue, yellow if due today

### Keyboard Shortcuts

#### Navigation

| Key | Action |
|-----|--------|
| `j` or `↓` | Move down |
| `k` or `↑` | Move up |
| `g` | Go to first task |
| `G` | Go to last task |
| `Ctrl+u` or `Page Up` | Page up (10 items) |
| `Ctrl+d` or `Page Down` | Page down (10 items) |
| `h` or `←` | Focus sidebar |
| `l` or `→` | Focus task list |
| `Enter` | Select sidebar item |

#### Task Actions

| Key | Action |
|-----|--------|
| `a` | Add new task |
| `A` | Add subtask under selected task |
| `e` | Edit task title |
| `d` | Delete task (with confirmation) |
| `x` or `Space` | Toggle task completion |
| `p` | Cycle priority (None → Low → Medium → High → Urgent) |
| `D` | Edit due date (YYYY-MM-DD format) |
| `T` | Edit tags (comma-separated) |
| `n` | Edit description/notes |
| `m` | Move task to project |
| `t` | Toggle time tracking |

#### Project Actions

| Key | Action |
|-----|--------|
| `P` | Create new project |

#### Search & Filter

| Key | Action |
|-----|--------|
| `/` | Search tasks (by title or tags) |
| `Ctrl+l` | Clear search |
| `#` | Filter by tag (comma-separated) |
| `Ctrl+t` | Clear tag filter |
| `s` | Cycle sort field (Created → Priority → Due Date → Title → Status) |
| `S` | Toggle sort order (Ascending/Descending) |

#### View Controls

| Key | Action |
|-----|--------|
| `b` | Toggle sidebar |
| `c` | Toggle showing completed tasks |
| `?` | Show/hide help popup |

#### Multi-Select (Bulk Operations)

| Key | Action |
|-----|--------|
| `v` | Toggle multi-select mode |
| `V` | Select all visible tasks |
| `Space` | Toggle selection of current task (in multi-select mode) |
| `Ctrl+v` | Clear selection and exit multi-select |

When in multi-select mode, selected tasks show a `●` indicator. After selecting tasks, you can delete them with `d` or perform other operations.

#### Dependencies & Recurrence

| Key | Action |
|-----|--------|
| `B` | Edit dependencies (set which tasks block this one) |
| `R` | Set task recurrence pattern |

Tasks with dependencies show a `[B]` indicator. Recurring tasks show a `↻` indicator.

**Recurrence patterns:**
- `d` - Daily (repeats every day)
- `w` - Weekly (repeats on same day of week)
- `m` - Monthly (repeats on same day of month)
- `y` - Yearly (repeats on same date each year)
- `0` - Clear recurrence (make non-recurring)

When you complete a recurring task, a new task is automatically created with the next due date.

#### Calendar View

| Key | Action |
|-----|--------|
| `←`/`→` or `h`/`l` | Navigate days |
| `↑`/`↓` or `j`/`k` | Navigate weeks |
| `<`/`>` | Previous/Next month |

The Calendar view shows a monthly grid with tasks displayed for each day. Days with tasks show a dot indicator, and the selected day's tasks are listed in a panel on the right.

#### Export

| Key | Action |
|-----|--------|
| `Ctrl+e` | Export tasks to CSV |
| `Ctrl+i` | Export tasks to ICS (iCalendar) |

#### Macros

| Key | Action |
|-----|--------|
| `Ctrl+q` | Start/stop macro recording (then press 0-9 for slot) |
| `@0`-`@9` | Play macro from slot 0-9 |

Record a sequence of actions and replay them later:
1. Press `Ctrl+q` to start recording, then press a digit (0-9) to select the slot
2. Perform any actions you want to record
3. Press `Ctrl+q` again, then the same digit to save the macro
4. Press `@` followed by the digit to replay the macro

The footer shows `[REC]` when recording is active.

#### Pomodoro Timer

| Key | Action |
|-----|--------|
| `F5` | Start Pomodoro session (4 cycles) |
| `F6` | Pause/Resume timer |
| `F7` | Skip current phase (work/break) |
| `F8` | Stop Pomodoro session |

The Pomodoro timer appears in the status bar when active, showing:
- Phase icon: 🍅 (work), ☕ (short break), 🌴 (long break)
- Remaining time (MM:SS)
- Cycle progress [completed/goal]
- Pause indicator (⏸) when paused

#### Templates

| Key | Action |
|-----|--------|
| `Ctrl+n` | Show template picker |
| `0`-`9` | Select template by number (in picker) |
| `j`/`k` | Navigate template list (in picker) |
| `Enter` | Create task from selected template |
| `Esc` | Cancel template picker |

Templates allow quickly creating common task types with preset fields:
- **Bug Fix**: High priority with #bug tag and structured description
- **Feature**: Medium priority with #feature tag
- **Review**: Medium priority, due tomorrow
- **Meeting Notes**: Low priority with attendee/agenda template
- **Daily Task**: Low priority, due today
- **Weekly Task**: Low priority, due in 7 days
- **Urgent**: Urgent priority, due today
- **Research**: Low priority with research template

After selecting a template, the task is created and you can edit the title.

#### General

| Key | Action |
|-----|--------|
| `u` or `Ctrl+z` | Undo last action |
| `U` or `Ctrl+r` | Redo last action |
| `Ctrl+s` | Save |
| `q` or `Esc` | Quit TaskFlow |

### Task Priorities

Tasks can have one of five priority levels:

| Priority | Symbol | Color |
|----------|--------|-------|
| Urgent | `!!!!` | Red |
| High | `!!!` | Light Red |
| Medium | `!!` | Yellow |
| Low | `!` | Green |
| None | (blank) | - |

### Task Statuses

| Status | Symbol | Description |
|--------|--------|-------------|
| Todo | `[ ]` | Not started |
| In Progress | `[~]` | Currently being worked on |
| Blocked | `[!]` | Waiting on something |
| Done | `[x]` | Completed |
| Cancelled | `[-]` | No longer needed |

### Views

| View | Description |
|------|-------------|
| All Tasks | Shows all tasks (default) |
| Today | Tasks due today |
| Upcoming | Tasks with future due dates |
| Overdue | Tasks past their due date |
| Calendar | Monthly calendar view with task indicators |
| Dashboard | Statistics overview with completion rates and time tracking |
| Projects | Tasks assigned to a project |

### Dashboard View

The Dashboard provides an overview of your task statistics:

- **Completion Panel**: Overall completion rate, overdue count, and completion by priority
- **Time Tracking Panel**: Total time tracked, average time per task, current tracking status
- **Projects Panel**: Per-project completion percentages
- **Status Distribution**: Bar chart showing task counts by status (Todo, In Progress, Blocked, Done, Cancelled)
- **This Week Panel**: Tasks created and completed this week, active tasks count

Access the Dashboard by selecting it from the sidebar navigation.

### Due Dates

Due dates are displayed with color coding:
- **Red**: Task is overdue
- **Yellow**: Task is due today
- **Gray**: Task is due in the future

## Configuration

TaskFlow stores configuration in `~/.config/taskflow/`:

```
~/.config/taskflow/
├── config.toml        # General settings
├── keybindings.toml   # Custom key mappings
└── themes/
    └── default.toml   # Color themes
```

### Settings (config.toml)

```toml
# Storage backend: json, yaml, sqlite, markdown
backend = "json"

# Data file path (relative to config dir or absolute)
data_path = "tasks.json"

# UI defaults
show_sidebar = true
show_completed = false
default_priority = "none"

# Auto-save interval in seconds (0 to disable)
auto_save_interval = 300

# Theme name
theme = "default"
```

### Keybindings (keybindings.toml)

```toml
[bindings]
j = "move_down"
k = "move_up"
q = "quit"
# ... customize as needed
```

## Architecture

TaskFlow uses **The Elm Architecture (TEA)** pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                         Event Loop                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────┐    ┌──────────┐    ┌─────────┐              │
│   │  Model  │───▶│  Update  │───▶│  View   │              │
│   │ (State) │    │(Messages)│    │  (UI)   │              │
│   └─────────┘    └──────────┘    └─────────┘              │
│        ▲                              │                    │
│        │                              │                    │
│        └──────────────────────────────┘                    │
│                   User Input                               │
└─────────────────────────────────────────────────────────────┘
```

- **Model**: Central application state (tasks, projects, UI state)
- **Message**: Events that can change state (navigation, task actions, etc.)
- **Update**: Pure function that takes state + message and produces new state
- **View**: Renders the UI based on current state

### Component Interaction

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              main.rs                                     │
│                         (Event Loop + Terminal)                          │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│    config/    │      │     app/      │      │      ui/      │
│  ─────────────│      │  ─────────────│      │  ─────────────│
│  settings.rs  │◀────▶│   model.rs    │─────▶│   view.rs     │
│  keybindings  │      │  message.rs   │      │  components/  │
│   theme.rs    │      │  update.rs    │      │   sidebar     │
└───────────────┘      │   undo.rs     │      │   task_list   │
                       └───────┬───────┘      │   dialogs     │
                               │              └───────────────┘
                               ▼
                       ┌───────────────┐
                       │   storage/    │
                       │  ─────────────│
                       │  backends/    │
                       │  ├─ json.rs   │
                       │  ├─ yaml.rs   │
                       │  ├─ sqlite.rs │
                       │  └─ markdown  │
                       │  import.rs    │
                       │  export.rs    │
                       └───────────────┘
                               │
                       ┌───────┴───────┐
                       ▼               ▼
               ┌───────────┐   ┌───────────┐
               │  domain/  │   │  File     │
               │  ─────────│   │  System   │
               │  task.rs  │   └───────────┘
               │ project.rs│
               │  tag.rs   │
               │filter.rs  │
               └───────────┘
```

### Data Flow

1. **User Input** → Terminal captures key events
2. **Key Mapping** → Keybindings translate keys to actions
3. **Message Dispatch** → Actions become typed messages
4. **Update** → Model is updated based on message
5. **Side Effects** → Storage operations, undo stack updates
6. **View Render** → UI components read model state
7. **Terminal Draw** → Ratatui renders to terminal

## Project Structure

```
taskflow/
├── src/
│   ├── main.rs           # Entry point and event loop
│   ├── lib.rs            # Library exports
│   ├── domain/           # Core entities
│   │   ├── task.rs       # Task model
│   │   ├── project.rs    # Project model
│   │   ├── tag.rs        # Tag model
│   │   ├── time_entry.rs # Time tracking
│   │   └── filter.rs     # Query filters
│   ├── app/              # TEA architecture
│   │   ├── model.rs      # Application state
│   │   ├── message.rs    # Event types
│   │   ├── update.rs     # State transitions
│   │   └── undo.rs       # Undo stack
│   ├── config/           # Configuration
│   │   ├── settings.rs   # App settings
│   │   ├── keybindings.rs# Key mappings
│   │   └── theme.rs      # Color themes
│   ├── storage/          # Persistence
│   │   └── backends/     # JSON, YAML, SQLite, Markdown
│   └── ui/               # User interface
│       ├── view.rs       # Main renderer
│       └── components/   # UI widgets
├── tests/
│   └── integration.rs    # Integration tests
├── Cargo.toml
└── README.md
```

## Storage Backends

TaskFlow supports multiple storage backends:

| Backend | File Format | Best For |
|---------|-------------|----------|
| JSON | `.json` | Default, fast, compact |
| YAML | `.yaml` | Human-readable, easy to edit |
| SQLite | `.db` | Large datasets, queries |
| Markdown | directory | Integration with other tools |

### Markdown Backend

The Markdown backend stores each task as a separate `.md` file with YAML frontmatter:

```markdown
---
id: "550e8400-e29b-41d4-a716-446655440000"
title: "Implement feature X"
status: todo
priority: high
due_date: 2025-01-15
tags:
  - rust
  - backend
---

Task description and notes go here...
```

## Export Formats

TaskFlow can export tasks to external formats for use with other tools:

### CSV Export (`Ctrl+e`)

Exports all tasks to a CSV file with columns:
- ID, Title, Status, Priority, Due Date, Tags, Project ID, Description, Created, Completed

CSV files can be opened in spreadsheet applications like Excel, Google Sheets, or LibreOffice Calc.

### ICS Export (`Ctrl+i`)

Exports tasks as iCalendar (ICS) format VTODO items, compatible with:
- Apple Calendar/Reminders
- Google Calendar
- Microsoft Outlook
- Any calendar application supporting the iCalendar standard

Each task becomes a VTODO component with:
- Summary (title)
- Description
- Due date
- Priority (mapped to iCalendar priority 1-9)
- Status (NEEDS-ACTION, IN-PROCESS, COMPLETED, CANCELLED)
- Categories (tags)

## Library Usage

TaskFlow can also be used as a library in your Rust projects:

### Add to Cargo.toml

```toml
[dependencies]
taskflow = { path = "path/to/taskflow" }
```

### Creating Tasks

```rust
use taskflow::domain::{Task, Priority, TaskStatus};
use chrono::Utc;

// Create a simple task
let task = Task::new("Write documentation");

// Create a task with builder pattern
let today = Utc::now().date_naive();
let task = Task::new("Fix critical bug")
    .with_priority(Priority::Urgent)
    .with_due_date(today)
    .with_tags(vec!["bug".into(), "critical".into()])
    .with_description("Users can't login via SSO".to_string());

// Toggle completion
let mut task = task;
task.toggle_complete();
assert_eq!(task.status, TaskStatus::Done);
```

### Working with Projects

```rust
use taskflow::domain::{Project, Task};

// Create a project
let project = Project::new("Backend API")
    .with_color("#3498db");

// Assign tasks to the project
let task = Task::new("Implement REST endpoints")
    .with_project(project.id.clone());
```

### Time Tracking

```rust
use taskflow::domain::{Task, TimeEntry};

let task = Task::new("Code review");

// Start tracking
let mut entry = TimeEntry::start(task.id.clone());
assert!(entry.is_running());

// ... do some work ...

// Stop and get duration
entry.stop();
println!("Time spent: {}", entry.formatted_duration());
```

### Using Storage Backends

```rust
use taskflow::storage::{create_backend, BackendType, StorageBackend};
use taskflow::domain::Task;
use std::path::Path;

// Create a backend
let mut backend = create_backend(
    BackendType::Json,
    Path::new("tasks.json")
)?;

// Save a task
let task = Task::new("My task");
backend.create_task(&task)?;

// Load all tasks
let tasks = backend.list_tasks()?;
```

### Exporting Tasks

```rust
use taskflow::storage::{export_to_string, ExportFormat};
use taskflow::domain::{Task, Priority};

let tasks = vec![
    Task::new("Task 1").with_priority(Priority::High),
    Task::new("Task 2").with_priority(Priority::Low),
];

// Export to CSV
let csv = export_to_string(&tasks, ExportFormat::Csv);

// Export to iCalendar
let ics = export_to_string(&tasks, ExportFormat::Ics);
```

### Using Themes

```rust
use taskflow::config::{Theme, ColorSpec};
use ratatui::style::Color;

// Load default theme
let theme = Theme::default();

// Access colors
let accent = theme.colors.accent.to_color();
let urgent = theme.priority.urgent.to_color();

// Create custom colors
let custom = ColorSpec::Hex("#ff5500".to_string());
let rgb = ColorSpec::Rgb { r: 100, g: 150, b: 200 };
```

For more examples, see the [documentation](https://docs.rs/taskflow) or the `tests/` directory.

## Testing

TaskFlow includes unit tests, integration tests, and stress tests.

### Running Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run tests with output
cargo test -- --nocapture
```

### Stress Tests

Stress tests verify performance at multiple scale levels. Located in `tests/stress.rs`:

| Level | Tasks | Description |
|-------|-------|-------------|
| 1 | 100 | Quick sanity check |
| 2 | 1,000 | Standard stress test |
| 3 | 10,000 | Heavy load |
| 4 | 100,000 | Extreme load (ignored by default) |
| 5 | 1,000,000 | Maximum stress (ignored by default) |

```bash
# Run all non-ignored stress tests (levels 1-3)
cargo test --test stress

# Run with timing output
cargo test --test stress -- --nocapture

# Run specific level
cargo test --test stress level_1
cargo test --test stress level_2
cargo test --test stress level_3

# Run ignored slow tests (levels 4-5)
cargo test --test stress level_4 -- --ignored
cargo test --test stress level_5 -- --ignored

# Run all levels including slow tests
cargo test --test stress -- --ignored
```

**Base performance thresholds (for 1,000 tasks):**

| Operation | Threshold |
|-----------|-----------|
| Refresh visible tasks | <200ms |
| Filter by priority | <100ms |
| Sort by due date | <100ms |
| Sort by priority | <100ms |
| Search tasks | <150ms |
| Filter by tags | <100ms |
| Combined operations | <200ms |

Thresholds scale with O(n log n) complexity for larger task counts.

### Code Quality

```bash
# Run clippy lints
cargo clippy --all-targets

# Check formatting
cargo fmt --check

# Build documentation
cargo doc --no-deps
```

## Performance Tuning

TaskFlow is designed for responsive performance up to 10,000+ tasks. Here are tips for optimal performance:

### Storage Backend Selection

| Backend | Read Speed | Write Speed | Best For |
|---------|------------|-------------|----------|
| JSON | Fast | Fast | Default, small-medium datasets |
| YAML | Medium | Medium | Human editing, external tools |
| SQLite | Very Fast | Fast | Large datasets (1000+ tasks) |
| Markdown | Slow | Medium | Git integration, external editing |

**Recommendation**: For 1000+ tasks, use SQLite for best filter/sort performance.

### Configuration Tips

1. **Disable auto-save for large datasets**: Set `auto_save_interval = 0` and save manually with `Ctrl+s`

2. **Hide completed tasks**: Keep `show_completed = false` to reduce rendering overhead

3. **Use project filters**: Filter to specific projects rather than viewing all tasks

### Memory Usage

TaskFlow keeps all tasks in memory. Approximate memory usage:
- ~1KB per task (base overhead)
- +~100 bytes per tag
- +~500 bytes per time entry
- +~2KB for tasks with subtasks

For 10,000 tasks: expect ~15-20MB memory usage.

### Known Limitations

- **Markdown backend**: Not recommended for 500+ tasks (file I/O overhead)
- **Tag search in SQLite**: Uses string matching (no dedicated index)
- **Undo stack**: Keeps last 100 actions in memory

## Known Issues

### Current Limitations

1. **Large update.rs file**: The main update logic is in a single 5500-line file. While functional, this makes navigation difficult. A future refactor may split this into modules.

2. **SQLite tag queries**: Tag filtering in SQLite uses JSON string matching rather than a normalized table, which is slower for large datasets.

3. **No external cache invalidation**: The Markdown backend caches parsed files but doesn't detect external modifications until restart.

### Workarounds

- **Slow tag filtering with SQLite**: Use project-based organization instead of heavy tag usage
- **External Markdown edits**: Restart TaskFlow after editing files externally

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

MIT License
