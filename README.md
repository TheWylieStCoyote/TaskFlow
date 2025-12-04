# TaskFlow

A terminal-based project management application built with Rust.

TaskFlow provides a fast, keyboard-driven interface for managing tasks, projects, and time tracking—all from your terminal.

## Features

- **Task Management**: Create, organize, and track tasks with priorities, due dates, and status
- **Project Organization**: Group related tasks under projects
- **Tagging System**: Categorize tasks with flexible tags
- **Time Tracking**: Track time spent on tasks (coming soon)
- **Vim-style Navigation**: Fast keyboard-driven interface
- **Multiple Storage Backends**: Save data as Markdown, YAML, JSON, or SQLite (coming soon)
- **Customizable**: Themes, keybindings, and custom views via config files (coming soon)

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
# Run TaskFlow
cargo run

# Or if installed
taskflow
```

## Usage

### Interface Overview

```
┌─────────────────────────────────────────────────────────────┐
│ TaskFlow - Project Management TUI                           │
├─────────────────────────────────────────────────────────────┤
│ !!!! [!] Review and fix bugs                      [12/10]   │
│ !!!  [~] Create TEA architecture                            │
│ !!   [ ] Build task list UI                                 │
│ !!   [ ] Add storage backends                               │
│ !    [ ] Implement keybinding config                        │
│ !    [ ] Add theme support                                  │
│      [ ] Write documentation                                │
│      [x] Set up project structure                           │
│      [x] Implement domain types                             │
├─────────────────────────────────────────────────────────────┤
│ 7 tasks (2 completed) | hiding completed | Press ? for help │
└─────────────────────────────────────────────────────────────┘
```

### Task Display

Each task shows:
- **Priority indicator** (left): `!!!!` (urgent), `!!!` (high), `!!` (medium), `!` (low), or blank (none)
- **Status symbol**: `[ ]` (todo), `[~]` (in progress), `[!]` (blocked), `[x]` (done), `[-]` (cancelled)
- **Task title**
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

#### Task Actions

| Key | Action |
|-----|--------|
| `x` or `Space` | Toggle task completion |

#### View Controls

| Key | Action |
|-----|--------|
| `c` | Toggle showing completed tasks |
| `?` | Show/hide help popup |

#### General

| Key | Action |
|-----|--------|
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

Tasks are automatically sorted by priority (highest first), then by creation date.

### Task Statuses

| Status | Symbol | Description |
|--------|--------|-------------|
| Todo | `[ ]` | Not started |
| In Progress | `[~]` | Currently being worked on |
| Blocked | `[!]` | Waiting on something |
| Done | `[x]` | Completed |
| Cancelled | `[-]` | No longer needed |

### Due Dates

Due dates are displayed with color coding:
- **Red**: Task is overdue
- **Yellow**: Task is due today
- **Gray**: Task is due in the future

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
│   │   └── update.rs     # State transitions
│   └── ui/               # User interface
│       ├── view.rs       # Main renderer
│       └── components/   # UI widgets
├── Cargo.toml
└── README.md
```

## Roadmap

### Current (v0.1)
- [x] Basic task list display
- [x] Vim-style navigation
- [x] Task completion toggle
- [x] Priority and status indicators
- [x] Due date display
- [x] Help popup

### Planned (v0.2)
- [ ] Task creation and editing
- [ ] Project sidebar
- [ ] Tag filtering
- [ ] Search functionality

### Future
- [ ] Multiple storage backends (YAML, JSON, SQLite, Markdown)
- [ ] Configuration files for themes and keybindings
- [ ] Time tracking with start/stop timer
- [ ] Task dependencies
- [ ] Custom views
- [ ] Undo/redo

## Configuration (Coming Soon)

TaskFlow will support configuration via TOML files:

```
~/.config/taskflow/
├── config.toml        # General settings
├── keybindings.toml   # Custom key mappings
├── themes/
│   └── default.toml   # Color themes
└── views.toml         # Custom filtered views
```

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

MIT License
