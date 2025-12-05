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
| Projects | Tasks assigned to a project |

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

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

MIT License
