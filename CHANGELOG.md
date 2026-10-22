# Changelog

All notable changes to TaskFlow will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **CalDAV works offline**: the sidecar caches the server's tasks and queues pushes and deletes that have not reached the server; a task created or edited offline survives a restart and is pushed by the first flush that succeeds (local edits win until pushed, queued deletes are never resurrected)
- **`audit_log` entity in the pipe interface** (`list`/`get`, read-only): filter by `task_id`, action and date, page from the newest entry, CSV output flattens field changes
- **`TASKFLOW_CONFIG_DIR`** overrides where `config.toml` and `keybindings.toml` live
- **Saved filters and the audit log are persisted** on every backend (new `SavedFilterRepository`/`AuditLogRepository`; SQLite gains an `audit_log` table, Markdown an append-only `audit_log.jsonl`); the CalDAV backend keeps time entries, work logs, Pomodoro state and other local-only data in a `caldav.local.json` sidecar
- **Time tracking foundation**: one shared duration formatter (`2h`, `1h 30m`), `TimeAnalytics` breakdowns by project, task, tag, day and local hour, and Pomodoro work phases recorded as time entries on the task (`+`/`-` adjust the cycle goal in focus mode)
- **Reports → Time panel**: selectable 7/30/90-day window (`w`), Today / This Week / Avg per active day / live Tracking stats, time by project with hours and share, daily series (bars or sparkline), local hour-of-day chart, top tasks
- **Reports → Tasks panel**: every task with tracked time in the selected window as a scrollable table (task, project, entries, time, share; `j`/`k`, `g`/`G`, PgUp/PgDn); the exported Markdown/HTML reports list the top 25 tasks the same way
- **Dashboard Time Tracking tile**: Today, This Week, Last 30 days, average per active day and the running timer; **footer** shows a live `⏱ HH:MM:SS` readout for the running task
- **Time log editor**: entries grouped by local day with day and grand totals; `a` opens a real add prompt (`[date] START[-END | +DURATION] [note]`); `n` edits the note, `t` moves an entry to another date, `+`/`-` nudge its length by 5 minutes
- **`taskflow time` (`tt`)**: time report for today, this week, the last N days or a date range, by project, day, task or tag, with ASCII bars or `--json`
- **Exports**: CSV gains `Estimated Minutes` / `Actual Minutes` columns (also read on import); Markdown and HTML reports gain a Time Tracking section with per-project and per-day tables
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

- **Time log lists a day's entries in time order** (days still newest first); the task detail preview keeps showing the five most recent entries
- **Error Messages**: More actionable guidance when operations fail
- **Theme Consistency**: Standardized highlighting and dialog styling across all views
- **Analytics Module**: Split into focused submodules for better maintainability
- **Forecast View**: Safer handling of empty data with filter_map instead of unwrap()

### Fixed

- **Timeline selection stops at the last drawn row** instead of the last visible task, and Enter opens the highlighted row (it could open a different task when the list order differed from the timeline order)
- **Unparsable SQLite rows are reported, not dropped**: every skipped row is logged with table, id and the failing column; a bad timestamp is a row error instead of becoming "now"; single-row lookups on such a row return an error
- **Markdown backend no longer overwrites corrupt data files**: a `*.yaml` aux file that does not parse makes startup fail with `CorruptFile` naming the file, leaving it untouched
- **Kanban, Eisenhower, Timeline and Duplicates scroll from the real viewport height** instead of estimated 5–15 row windows, so the selection stays visible at any terminal size
- **Snooze, clear snooze and bulk snooze are undoable**
- **Redo of a timer stop** no longer leaves the stopped entry marked as the active timer
- **Pomodoro statistics record the minutes actually worked** in each cycle (pauses excluded) instead of the configured length
- **`taskflow list --project <unknown>`** errors and lists the available projects instead of silently listing everything
- **Merged-branch detection** recognises branches checked out in another worktree (`+ ` marker in `git branch --merged`)
- **CSV import follows RFC 4180**: quoted fields may contain commas, newlines and doubled quotes, so exports round-trip
- **Help popup** no longer advertises digit keys for switching views; docs (README, CLAUDE.md, manual, quick reference) now match the code for Esc, the pipe protocol, the CLI and every key table
- **Time totals counted twice**: Reports → Time, Insights and the Markdown/HTML reports added each task's cached total on top of its time entries; focus mode added the live session twice
- **Stale task totals**: `actual_minutes` was only recomputed when a timer stopped; adding, editing or deleting time log entries, undo and the pipe API now recompute it and refresh caches
- **Report windows spanned N+1 days**: "last 7 days" covered eight calendar dates (the 7-day Daily chart showed eight bars); every report window now covers exactly N dates ending today
- **Time log used UTC**: entries are displayed and edited in local time; an end time before the start rolls to the next day
- Evening review counted a running timer as zero; `taskflow stats` ignored manually corrected durations
- **Goal deletion orphaned key results** in the JSON, YAML and CalDAV backends; they now cascade like SQLite and Markdown
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
