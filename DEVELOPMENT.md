# TaskFlow Development Summary

## Overview

TaskFlow is a CLI TUI project management application built with Rust using:
- **Ratatui** + **Crossterm** for the terminal UI
- **The Elm Architecture (TEA)** for state management
- **Trait-based storage abstraction** supporting multiple backends
- **Configuration-based extensibility** (themes, keybindings, custom views)

## Implementation Phases

### Phase 1: Foundation (Complete)
- Project structure with Cargo.toml and dependencies
- Core domain types: Task, Project, Tag, TimeEntry, Filter
- TEA architecture: Model, Message, Update pattern
- Basic UI with task list and help popup

### Phase 2: Storage Abstraction (Complete)
- Storage error types and repository traits
- JSON, YAML, SQLite, and Markdown backends
- Full integration with the app Model

**Files:**
- `src/storage/error.rs` - StorageError enum
- `src/storage/repository.rs` - Trait definitions
- `src/storage/mod.rs` - BackendType and factory
- `src/storage/backends/json.rs` - JSON file backend
- `src/storage/backends/yaml.rs` - YAML file backend
- `src/storage/backends/sqlite.rs` - SQLite database backend
- `src/storage/backends/markdown.rs` - Markdown files with YAML frontmatter

### Phase 3: Core UI Enhancements (Complete)
- Sidebar component showing navigation views and projects
- Input dialog component for task creation
- Confirmation dialog for task deletion
- New keybindings for task management

**Files:**
- `src/ui/components/sidebar.rs` - Navigation sidebar
- `src/ui/components/input.rs` - Input and confirmation dialogs

### Phase 4: Configuration System (Complete)
- Settings module - loads from `~/.config/taskflow/config.toml`
- Keybindings module - customizable key mappings
- Theme module - color scheme configuration with named colors, hex, and RGB support
- Settings integrate with CLI args (CLI overrides config file)

**Files:**
- `src/config/mod.rs` - Config module root
- `src/config/settings.rs` - Application settings
- `src/config/keybindings.rs` - Key binding configuration
- `src/config/theme.rs` - Theme and color definitions

### Phase 5: Advanced Features (Complete)
- Time tracking with start/stop timer
- Time entries stored per task with duration tracking
- Visual indicators in task list (red dot for active tracking, cyan duration display)
- Automatic timer stop on quit

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to first |
| `G` | Go to last |
| `Ctrl+u` / `PageUp` | Page up |
| `Ctrl+d` / `PageDown` | Page down |
| `x` / `Space` | Toggle complete |
| `a` | Add new task |
| `D` | Delete task |
| `t` | Toggle time tracking |
| `c` | Toggle show completed |
| `b` | Toggle sidebar |
| `?` | Show help |
| `Ctrl+s` | Save |
| `q` / `Esc` | Quit |

## Running the Application

```bash
# Demo mode with sample data
cargo run -- --demo

# Normal mode (loads/saves from ~/.local/share/taskflow/)
cargo run

# Specify storage backend
cargo run -- --backend yaml
cargo run -- --backend json
cargo run -- --backend sqlite
cargo run -- --backend markdown

# Custom data path
cargo run -- --data /path/to/tasks.json
```

## Project Structure

```
src/
├── main.rs                 # Entry point, CLI parsing, event loop
├── lib.rs                  # Library root
│
├── app/
│   ├── mod.rs
│   ├── model.rs            # Application state (TEA Model)
│   ├── message.rs          # Message enum definitions
│   └── update.rs           # State update logic
│
├── ui/
│   ├── mod.rs
│   ├── view.rs             # Main view function
│   └── components/
│       ├── mod.rs
│       ├── task_list.rs    # Task list widget
│       ├── sidebar.rs      # Navigation sidebar
│       ├── input.rs        # Input/confirmation dialogs
│       └── help.rs         # Help popup
│
├── domain/
│   ├── mod.rs
│   ├── task.rs             # Task entity
│   ├── project.rs          # Project entity
│   ├── tag.rs              # Tag entity
│   ├── time_entry.rs       # Time tracking entry
│   └── filter.rs           # Query/filter types
│
├── storage/
│   ├── mod.rs              # Storage traits and factory
│   ├── repository.rs       # Repository trait definitions
│   ├── error.rs            # Storage error types
│   └── backends/
│       ├── mod.rs
│       ├── json.rs         # JSON backend
│       ├── yaml.rs         # YAML backend
│       ├── sqlite.rs       # SQLite backend
│       └── markdown.rs     # Markdown backend
│
└── config/
    ├── mod.rs
    ├── settings.rs         # Main settings
    ├── keybindings.rs      # Key mappings
    └── theme.rs            # Theme configuration
```

## Configuration Files

Configuration files are stored in `~/.config/taskflow/`:

- `config.toml` - Main settings
- `keybindings.toml` - Custom key mappings
- `themes/*.toml` - Custom themes

### Example config.toml

```toml
backend = "json"
theme = "default"
show_sidebar = true
show_completed = false
auto_save_interval = 300
default_priority = "none"
```

## Dependencies

- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation
- `serde` / `serde_json` / `serde_yaml` - Serialization
- `toml` - Configuration parsing
- `chrono` - Date/time handling
- `uuid` - Unique identifiers
- `rusqlite` - SQLite database
- `clap` - CLI argument parsing
- `directories` - Platform-specific paths
- `thiserror` / `anyhow` - Error handling
