# TaskFlow Roadmap

A collection of proposed features, enhancements, and integrations for future development. Organized by category with complexity estimates and the primary areas of the codebase each would touch.

**Status as of 2026-10-23.** Legend: ✅ done · 🔶 partially shipped · 🚧 implemented on an unmerged branch · ⬜ not started.

| Status | Items |
|--------|-------|
| ✅ Done | 2.2, 3.9 |
| 🔶 Partial | 1.3, 1.9, 1.10, 2.4, 2.7, 2.8, 2.10, 3.10 |
| 🚧 On an unmerged branch | 1.2, 1.8 (plus the missing halves of 1.3, 1.9, 3.10) |
| ⬜ Not started | 1.1, 1.4–1.7, 2.1, 2.3, 2.5, 2.6, 2.9, 3.1–3.8 |

Three branches from March 2026 hold finished work that was never merged or pushed (they exist only in the local checkout): `feature/fuzzy_search` (1.8), `feature/desktop_notifications` (3.10) and `feature/timeblocking` (1.2, calendar half of 1.9, collapse half of 1.3). Each needs the same rebase-and-lint pass the Neovim branch got in PR #27.

---

## Category 1: UI / UX Enhancements

### 1.1 Persistent Side Panel (Task Detail)
Show task detail in a persistent right-hand split pane instead of only as a popup overlay. The panel auto-updates as the cursor moves through the task list — no keypress needed to open it.
- **Affected:** `src/ui/components/task_detail/`, layout logic in `src/ui/view/`
- **Complexity:** Medium — layout split already exists in Calendar view; reuse that pattern
- **Status:** ⬜ Not started — task detail is still a popup overlay (`i`/Enter).

### 1.2 Inline Task Editing
Edit task title directly in the task list row (press `e` or `F2`) without opening a separate input dialog.
- **Affected:** `src/ui/components/task_list/`, `src/bin/taskflow/input/mod.rs`
- **Complexity:** Medium
- **Status:** 🚧 Implemented on the unmerged branch `feature/timeblocking` (Mar 2026); needs rebasing onto master.

### 1.3 Subtask Tree View
Render subtasks as indented rows in the task list (collapsible with `za`). Subtasks exist in the domain but the list is currently flat.
- **Affected:** `src/app/model/view_queries.rs`, `src/ui/components/task_list/`
- **Complexity:** Medium-High
- **Status:** 🔶 Partial — subtasks render as indented rows (`nesting_depth` in `task_list`); collapse/expand is on the unmerged branch `feature/timeblocking`.

### 1.4 Configurable Dashboard Widgets
Let users choose which panels appear on the Dashboard and in what layout via `config.toml`. Currently the dashboard layout is hardcoded.
- **Affected:** `src/ui/components/dashboard/`, `src/config/settings.rs`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 1.5 Configurable Task List Columns
Multi-column task list where the user picks which fields appear as columns (priority, due date, project, estimate, tags) and their widths — similar to TaskWarrior's `rc.report` settings.
- **Affected:** `src/config/settings.rs`, `src/ui/components/task_list/`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 1.6 Project / Tag Progress Bars in Sidebar
Show inline completion percentages or mini progress bars next to project names in the sidebar.
- **Affected:** `src/ui/components/sidebar/`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 1.7 Task Aging Indicator
Visually dim or color-shift tasks that haven't been touched in N days, making stale work immediately visible without a report.
- **Affected:** `src/ui/components/task_list/`, `src/config/settings.rs`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 1.8 Fuzzy Quick-Open (Telescope-style)
A single keypress (e.g., `<Space><Space>`) opens a floating fuzzy finder over all tasks — type to filter, arrow to select, Enter to jump. Supplements the existing `/` search.
- **Affected:** New component `src/ui/components/quick_open/`, `src/bin/taskflow/input/mod.rs`
- **Complexity:** Medium
- **Status:** 🚧 Implemented on the unmerged branch `feature/fuzzy_search` (Mar 2026); needs rebasing onto master.

### 1.9 Time-Blocking in Calendar View
Assign tasks to specific time slots within a day in the Calendar view. The `scheduled_time` field already exists in the domain; this surfaces it visually.
- **Affected:** `src/ui/components/calendar/`, `src/app/update/navigation/calendar.rs`
- **Complexity:** High
- **Status:** 🔶 Partial — `scheduled_start_time`/`scheduled_end_time`, quick-add `time:9:00-11:00`, the `Alt+s` editor and ICS/CSV export shipped; the calendar rendering is on the unmerged branch `feature/timeblocking`.

### 1.10 Custom Sidebar Views via Config
Let users define named views in `config.toml` using the existing filter DSL — they appear as extra entries in the sidebar navigation.
- **Affected:** `src/config/settings.rs`, `src/app/model/mod.rs`, `src/ui/components/sidebar/`
- **Complexity:** Medium
- **Status:** 🔶 Partial — saved filters (filter + sort + icon) exist in the TUI and the pipe `saved_filter` entity, but cannot yet be declared in `config.toml`.

---

## Category 2: Power User Features

### 2.1 Scripting Hooks (on-event shell commands)
Run user-defined shell commands when task events occur: `on_complete`, `on_create`, `on_overdue`, `on_snooze_expire`. Configured in `config.toml`.
- **Affected:** `src/config/settings.rs`, `src/app/update/task.rs`
- **Complexity:** Low-Medium
- **Status:** ⬜ Not started.

### 2.2 Natural Language Date Parsing
Parse "next monday", "in 3 weeks", "end of month" in the due date input field. `chrono` already handles dates; just needs a thin parsing layer.
- **Affected:** `src/app/update/task.rs`, due date input handling
- **Complexity:** Low — no new dependencies needed
- **Status:** ✅ Done — the due-date prompt and quick-add share `app::quick_add::date` (`today`, `tomorrow`, weekdays, `next monday`, `this friday`, `eow`/`eom`/`eoy`, `+3d` offsets). The `in 3 weeks` phrasing is not recognised.

### 2.3 Workspaces / Context Profiles
Named profiles (e.g., "work", "personal") that switch the active project filter, theme, and keybindings in one step. Toggled via the command palette.
- **Affected:** `src/config/settings.rs`, `src/app/model/mod.rs`, `src/app/update/`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 2.4 Task Scoring / Prioritization Formula
User-configurable urgency score (e.g., RICE, or simple age + priority weighting). Adds a virtual "score" column and sort-by-score option.
- **Affected:** `src/domain/task/`, `src/config/settings.rs`, `src/app/model/view_queries.rs`
- **Complexity:** Medium
- **Status:** 🔶 Partial — `taskflow next` ranks by a fixed urgency formula; no user-configurable weights and no score column or sort.

### 2.5 Recurring Task Preview
Show a "next N occurrences" preview when editing recurrence settings so users can verify their pattern before saving.
- **Affected:** `src/domain/task/recurrence.rs`, new modal component
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 2.6 Watch Mode (Auto-reload)
Monitor the data file(s) with `inotify`/`kqueue` and automatically reload when changed externally (e.g., by a sync agent). Currently requires manual `R` refresh.
- **Affected:** `src/bin/taskflow/tui.rs`, event loop
- **Complexity:** Low — `notify` crate is a natural fit
- **Status:** ⬜ Not started — manual reload via the `RefreshStorage` action only.

### 2.7 Dependency Editor UI
The task dependency domain model exists but the UI is minimal. Add a dedicated panel for viewing and editing "blocks / blocked by" relationships inline.
- **Affected:** `src/ui/components/task_detail/`, `src/bin/taskflow/input/`
- **Complexity:** Medium
- **Status:** 🔶 Partial — task detail shows a *Blocked By* section and `EditDependencies` opens a numbered prompt; no dedicated panel.

### 2.8 Bulk Edit via Filter DSL
Apply field changes to all tasks matching a DSL expression from the command palette. Extends the existing bulk-delete/move actions.
- **Example:** `:bulk priority:high due:overdue → set status:blocked`
- **Affected:** `src/app/update/task.rs`, `src/ui/components/command_palette/`
- **Complexity:** Medium
- **Status:** 🔶 Partial — multi-select bulk delete / move / set status / set due date exist; no DSL-driven bulk edit.

### 2.9 Daily Note / Journal Entry
Attach a free-text daily note to any day in the Calendar view. Stored in the existing data backend alongside tasks.
- **Affected:** New domain type `DailyNote`, storage layer, `src/ui/components/calendar/`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 2.10 Focus Queue Panel
The focus queue (`AddToFocusQueue`) already exists as an action. Add a visual panel showing queued tasks in order with auto-advance to the next task on completion.
- **Affected:** `src/app/model/mod.rs`, new queue panel component
- **Complexity:** Medium
- **Status:** 🔶 Partial — `AddToFocusQueue`, `AdvanceFocusQueue` and `ClearFocusQueue` actions exist; the queue is not rendered anywhere.

---

## Category 3: Integrations

### 3.1 GitHub Issues Sync
Pull GitHub issues into TaskFlow as tasks via the GitHub REST API (token stored in config). Two-way: marking done in TaskFlow can close the issue via API. Uses `reqwest` (already an optional dependency).
- **Affected:** New `src/integrations/github/`, `src/config/settings.rs`
- **Complexity:** High
- **Status:** ⬜ Not started — design notes in `docs/GITHUB_GITLAB_SYNC.md`.

### 3.2 Webhook / HTTP Push Events
POST a JSON payload to a user-configured URL when task events fire (complete, create, overdue). A simple bridge to Zapier, n8n, Make, Home Assistant, etc.
- **Affected:** New `src/integrations/webhooks/`, `src/config/settings.rs`
- **Complexity:** Low-Medium
- **Status:** ⬜ Not started.

### 3.3 Local HTTP / JSON API Server
Optional `taskflow serve` mode exposing a local REST API. Enables integration from browser extensions, Raycast/Alfred, or other tools without spawning a CLI process each time.
- **Affected:** New `src/api/` module, new binary subcommand
- **Complexity:** High
- **Status:** ⬜ Not started — the editor socket (3.9) handlers are a natural base for it.

### 3.4 Obsidian Vault Integration
Scan an Obsidian vault for `- [ ]` checkboxes and import them as tasks, with optional backlinks to the source note. Complements the existing Markdown storage backend.
- **Affected:** New importer in `src/storage/import/`, `src/config/settings.rs`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 3.5 Taskwarrior Migration
One-shot import: `taskflow import taskwarrior` reads `~/.task/` export JSON and maps fields (projects, tags, priority, due, recurrence) to TaskFlow equivalents.
- **Affected:** New `src/storage/import/taskwarrior.rs`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 3.6 org-mode Export
Export tasks to Emacs org-mode format (`.org` files with `TODO`/`DONE` states, `SCHEDULED:`, `DEADLINE:`, tags). Complements existing CSV / ICS / Markdown / HTML exporters.
- **Affected:** New `src/storage/export/org.rs`
- **Complexity:** Low-Medium
- **Status:** ⬜ Not started.

### 3.7 Jira / Linear Import
Import issues from Jira (REST API) or Linear (GraphQL API) as tasks with project/tag mapping. Read-only sync initially.
- **Affected:** New `src/integrations/jira/` and `src/integrations/linear/`, `src/config/settings.rs`
- **Complexity:** High
- **Status:** ⬜ Not started.

### 3.8 Cloud Sync via Git
Auto-commit and push the data file to a user-configured git remote on every save; pull on startup. Provides free multi-machine sync using the existing git toolchain.
- **Affected:** New `src/integrations/git_sync.rs`, `src/config/settings.rs`
- **Complexity:** Medium (shells out to git)
- **Status:** ⬜ Not started — `taskflow git sync` links tasks to branches and is unrelated to data-file sync.

### 3.9 Neovim / Editor Plugin Interface
Expose a socket/pipe interface so a Neovim plugin (or any editor) can add tasks from code comments, show task status in the statusline, or open the TUI in a floating terminal.
- **Affected:** `src/bin/taskflow/` (socket listener), documentation
- **Complexity:** Medium
- **Status:** ✅ Done — PR #27 (Oct 2026): `src/socket/` Unix domain socket server in the TUI, `neovim/` Lua plugin, `[editor]` settings, plenary specs in CI.

### 3.10 Desktop Notification Enhancements
Fully activate the existing `desktop-notifications` optional Cargo feature: due-soon alerts (configurable lead time), snooze expiry notices, Pomodoro phase transitions. Ship enabled by default on supported platforms.
- **Affected:** `src/bin/taskflow/tui.rs`, `Cargo.toml` feature flags
- **Complexity:** Low (feature stub exists, needs wiring and config knobs)
- **Status:** 🔶 Partial — the `desktop-notifications` feature sends overdue, due-today, Pomodoro-phase and recurring-spawn notices; lead-time config and default-on are on the unmerged branch `feature/desktop_notifications`.

---

## Suggested Priority Tiers

| Tier | Items | Rationale |
|------|-------|-----------|
| **Quick wins** | 1.6, 1.7, 2.2, 2.5, 2.6, 3.6, 3.10 | Low complexity, immediately visible value |
| **Core UX** | 1.1, 1.2, 1.3, 1.5, 1.8, 2.7, 2.8 | Reduces daily friction |
| **Power** | 1.4, 1.10, 2.1, 2.3, 2.4, 2.9, 2.10 | Differentiates from simpler tools |
| **Integrations** | 3.1–3.9 | Expands ecosystem reach |
