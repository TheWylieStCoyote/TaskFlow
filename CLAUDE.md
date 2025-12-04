# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

- `cargo build` - Build the project
- `cargo run` - Build and run the TUI application
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run a single test
- `cargo clippy` - Run linter
- `cargo fmt` - Format code

## Project Overview

TaskFlow is a TUI project management application built with Rust using the Elm Architecture (TEA) pattern.

### Architecture

- **TEA Pattern**: Model → Update → View cycle with message passing
- **Ratatui + Crossterm**: Terminal UI framework
- **Configuration-based extensibility**: Themes, keybindings, custom views via TOML files
- **Multiple storage backends**: Designed to support Markdown, YAML, JSON, SQLite

### Module Structure

- `src/domain/` - Core entities: Task, Project, Tag, TimeEntry, Filter
- `src/app/` - TEA architecture: Model (state), Message (events), Update (state transitions)
- `src/ui/` - View rendering and UI components
- `src/storage/` - Storage abstraction and backends (to be implemented)
- `src/config/` - Configuration loading and parsing (to be implemented)

### Key Keybindings (in the TUI)

- `j/k` or arrows - Navigate up/down
- `x` or Space - Toggle task complete
- `c` - Toggle show completed tasks
- `?` - Show help
- `q` or Esc - Quit
