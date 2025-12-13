# Changelog

All notable changes to TaskFlow will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **Pipe Interface**: Scripting integration via `taskflow pipe` command for stdin/stdout interaction
  - Full CRUD operations for all entity types (tasks, projects, time entries, work logs, habits, goals, key results, tags)
  - JSON request/response protocol for programmatic access
  - Multiple output formats: JSON (default), YAML, and CSV
  - Filtering, sorting, and pagination for list operations
  - Bulk export/import operations
- **Goal/OKR Tracking**: Full OKR (Objectives and Key Results) management with goals, key results, progress tracking, and quarterly filtering
- **Habit Tracking**: Track daily/weekly habits with streaks, check-ins, and analytics; includes archival and habit view
- **Timeline View**: Gantt-style timeline for visualizing task schedules with zoom controls
- **Advanced Analytics Views**: New Heatmap, Forecast, Network, and Burndown chart views for visualizing task completion patterns, predicting workload, and tracking project progress
- **Quick Capture Mode**: Fast task entry with syntax hints for due dates, priorities, and tags
- **Mouse Support**: Click navigation and view interaction throughout the application
- **Calendar Enhancements**: Press Enter in calendar grid to view day's tasks; Enter in task list opens task details
- **Kanban View Improvements**: Task selection with detail view panel
- **Eisenhower Matrix Improvements**: Task selection with detail view panel
- **User Feedback**: Status messages with auto-dismiss timeouts for better UX
- **Sample Data**: Expanded demo data with 88 tasks across 10 projects for better onboarding
- **Historical Completion Dates**: Tasks now track completion timestamps for accurate heatmap visualization
- **Time Blocking**: Schedule tasks with specific time blocks
  - `scheduled_start_time` and `scheduled_end_time` fields for tasks
  - Quick-add syntax: `time:9:00-11:00` or `time:9am-11am`
  - Keybinding: `Alt+s` for editing time blocks
  - Time block display in timeline and task detail views
  - CSV/ICS export support for time block fields

### Changed

- **Error Messages**: More actionable guidance when operations fail
- **Theme Consistency**: Standardized highlighting and dialog styling across all views
- **Analytics Module**: Split into focused submodules for better maintainability
- **Forecast View**: Safer handling of empty data with filter_map instead of unwrap()

### Fixed

- **Doc Tests**: Converted ignored documentation tests to runnable examples
- **Snapshot Tests**: Replaced environment-sensitive snapshots with assertion-based tests
- **Clippy Warnings**: Resolved all linter warnings for cleaner codebase
- **Git TODOs View**: Fixed selection highlighting (off-by-one error in index calculation)
- **Git TODOs Parsing**: `TODO(user): message` now correctly extracts `message` (was extracting `user): message`)
- **Sidebar Views**: Removed duplicate view entries from navigation array
- **Habits View**: Fixed completion rate display (was showing 5000% instead of 50%)

### Performance

- **Storage Benchmarks**: Added criterion benchmarks for JSON, YAML, SQLite, and Markdown backends
- **Core Operations**: Benchmarks for task filtering, sorting, and search operations

### Documentation

- **Module Documentation**: Added comprehensive docs to 18 core modules
- **Architecture Decision Records**: Documented key design decisions in ADR format
- **View Documentation**: Expanded help text and keyboard shortcut hints
- **CONTRIBUTING Guide**: Updated contribution guidelines
- **Domain Module Docs**: Enhanced documentation for 6 key domain modules:
  - `analytics.rs`: Type overview table, ReportConfig examples, cross-references
  - `work_log.rs`: Usage patterns, TimeEntry comparison, query examples
  - `calendar_event.rs`: ICS import mapping, enum status docs, multi-day events
  - `goal.rs` / `key_result.rs`: OKR workflow, linked tasks, progress tracking
  - `pomodoro.rs`: Session lifecycle, phase transitions, pause/resume
  - `tag.rs`: Naming conventions, filter DSL integration

### Developer Experience

- **Test Coverage**: Added 200+ new tests including:
  - UI component tests for dashboard, task list, timeline, and weekly review
  - Snapshot tests for visual regression testing
  - Property-based tests for edge cases
  - CLI integration tests
  - Time tracking and Pomodoro handler tests
  - Domain type unit tests
- **CI Improvements**: Updated workflow configuration for better reliability
