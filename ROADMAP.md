# TaskFlow Roadmap

A collection of proposed features, enhancements, and integrations for future development. Organized by category with complexity estimates and the primary areas of the codebase each would touch.

**Status as of 2026-10-23.** Legend: ✅ done · 🔶 partially shipped · 🚧 implemented on an unmerged branch · ⬜ not started.

| Status | Items |
|--------|-------|
| ✅ Done | 2.2, 3.9 |
| 🔶 Partial | 1.3, 1.9, 1.10, 2.4, 2.7, 2.8, 2.10, 3.10, 4.3, 4.10 |
| 🚧 On an unmerged branch | 1.2, 1.8 (plus the missing halves of 1.3, 1.9, 3.10) |
| ⬜ Not started | 1.1, 1.4–1.7, 2.1, 2.3, 2.5, 2.6, 2.9, 3.1–3.8, 4.1, 4.2, 4.4–4.9, 4.11–4.19 |

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

## Category 4: Editor Integration Follow-ups

Findings from wiring the 3.9 socket and `neovim/` plugin into a real LazyVim setup (Oct 2026). Ordered by how much each improves the editor workflow.

### 4.1 Graceful Signal Handling
The crate has no signal handling. Neovim sends SIGHUP on quit, so a TUI hidden in an editor terminal dies without saving dirty state, without removing the socket file and without restoring the terminal. Install SIGHUP/SIGTERM/SIGINT handlers that set the quit flag so the normal shutdown path (save, `SocketGuard` drop, terminal restore) runs.
- **Affected:** `src/bin/taskflow/tui.rs`, `Cargo.toml` (`signal-hook` or `ctrlc`)
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.2 Save Promptly After Editor Writes
Socket writes set `storage.dirty` but persistence waits for `auto_save_interval` (default 300 s); a task created over the socket was lost when the TUI was killed. Save after draining the write channel, or debounce to a second or two.
- **Affected:** `src/bin/taskflow/tui.rs` (write drain loop)
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.3 Socket Ownership: Refuse Live Sockets, Remove Only Our Own File
`start()` unconditionally unlinked any existing socket file, so a second instance silently cut the first off from its editors (`connect: ENOENT`) and then removed the file again on exit. Probe the existing socket: a live one fails with `AddrInUse` (shown as "Editor socket disabled"); only a refused connection is treated as stale. The guard remembers the inode and leaves a file another process has replaced.
- **Affected:** `src/socket/mod.rs` (+ tests)
- **Complexity:** Low
- **Status:** 🔶 Partial — implemented and tested in the working tree; needs commit.

### 4.4 Headless Server Mode (`taskflow serve`)
Editor commands only work while a TUI is open, which forces users to keep a floating terminal alive and makes `q` (quit) versus hide a constant trap. A `serve` subcommand that owns the socket and the data, with the TUI becoming a client of it, removes the problem entirely. Smaller first step: let the plugin spawn the server.
- **Affected:** `src/bin/taskflow/` (new subcommand), `src/socket/`, `neovim/lua/taskflow/`
- **Complexity:** High
- **Status:** ⬜ Not started — overlaps with 3.3.

### 4.5 Time Tracking Over the Socket
The socket exposes no time tracking. Add the active timer (task, elapsed seconds) to `status`, plus `start_tracking` / `stop_tracking` requests, so the editor statusline can show the running timer.
- **Affected:** `src/socket/handlers.rs`, `src/socket/types.rs`, `neovim/lua/taskflow/init.lua`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 4.6 First-Class `focus_task` Write
`focus_task` rides the write channel wrapped in a fake `get` pipe envelope. Give `EditorWriteRequest` a proper variant and dispatch it directly in the TUI loop.
- **Affected:** `src/socket/types.rs`, `src/socket/handlers.rs`, `src/bin/taskflow/tui.rs`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.7 Filters and Limits for Socket `list`
The picker fetches every task and filters client-side. Forward `filters` (view, project, status, tag) and a `limit` through the pipe wrapper so "today" / "next" pickers are cheap on large lists.
- **Affected:** `src/socket/handlers.rs`, `neovim/lua/taskflow/telescope.lua`, `neovim/lua/taskflow/commands.lua`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 4.8 Richer `TaskSummary`
Add project name, scheduled date, tracked total and an `is_focus` flag so pickers and the list float can render them without a second request.
- **Affected:** `src/socket/types.rs`, `src/socket/snapshot.rs`, `neovim/lua/taskflow/ui.lua`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.9 Change-Event Subscription
One request per connection means the statusline polls every 30 s. A long-lived `subscribe` connection that pushes a line per model change lets the plugin refresh the statusline and open pickers instantly.
- **Affected:** `src/socket/mod.rs` (accept loop), `src/socket/handlers.rs`, `neovim/lua/taskflow/socket.lua`
- **Complexity:** Medium
- **Status:** ⬜ Not started — the natural base for 3.2.

### 4.10 Telescope Extension Entry Point and Picker Fallback
`load_extension('taskflow')` failed silently and `:Telescope taskflow` did not exist because `neovim/lua/telescope/_extensions/taskflow.lua` was missing. Also add a `vim.ui.select` (or snacks.picker) fallback for users without Telescope.
- **Affected:** `neovim/lua/telescope/_extensions/taskflow.lua`, `neovim/lua/taskflow/commands.lua`
- **Complexity:** Low
- **Status:** 🔶 Partial — the extension file exists in the working tree (needs commit); fallback not started.

### 4.11 Capture: Source Location, Dedupe, Back-Links
`:TaskFlowCapture` records only the comment text. Store file path and line (and the git branch, pairing with `git-todos`), skip comments already captured, and optionally rewrite the comment to `TODO(tf:<id>)` so the link survives edits.
- **Affected:** `neovim/lua/taskflow/capture.lua`, `neovim/lua/taskflow/commands.lua`, `src/domain/git/`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 4.12 Replace Deprecated Neovim APIs
`ui.lua` uses `nvim_buf_set_option` and `vim.fn.termopen`, both deprecated and slated for removal. Use `vim.bo[buf]` and `vim.fn.jobstart(cmd, { term = true })`.
- **Affected:** `neovim/lua/taskflow/ui.lua`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.13 Asynchronous Statusline Refresh
The timer calls `request_sync`, which spins `vim.wait` for up to 500 ms every 30 s when the server is slow. Use the async `request` and update the cache in the callback; fire a `User TaskFlowStatus` autocmd for other consumers.
- **Affected:** `neovim/lua/taskflow/init.lua`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.14 `:TaskFlowOpen` Reuses a Running Instance
It spawns a new TUI every time; with 4.3 the second instance now reports "Editor socket disabled". Check the socket first and reuse the existing terminal buffer, or hand off to the user's terminal integration.
- **Affected:** `neovim/lua/taskflow/commands.lua`, `neovim/lua/taskflow/ui.lua`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.15 Actions in the Task List Float
`Enter` only focuses the task in the TUI. Add toggle complete, start/stop tracking and delete, and show project and due date per row (needs 4.5, 4.8).
- **Affected:** `neovim/lua/taskflow/ui.lua`, `neovim/lua/taskflow/commands.lua`
- **Complexity:** Medium
- **Status:** ⬜ Not started.

### 4.16 `:checkhealth taskflow`
Report the resolved socket path, whether it answers `ping`, the server's `protocol_version` and `server_pid`, the `taskflow` binary on PATH and its version. Most of the Oct 2026 debugging would have been one command.
- **Affected:** `neovim/lua/taskflow/health.lua` (new)
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.17 Idempotent `setup()`
Calling `setup()` twice creates a second statusline timer and re-registers commands. Guard both.
- **Affected:** `neovim/lua/taskflow/init.lua`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.18 One Install Path
`install.sh` installs to `~/.local/bin` while `cargo install` targets `~/.cargo/bin`; both are commonly on PATH, so a stale copy can shadow the fresh one. Document one path, or have `install.sh` detect and remove the other.
- **Affected:** `install.sh`, `README.md`
- **Complexity:** Low
- **Status:** ⬜ Not started.

### 4.19 Document the Single-Instance Rule and Hide vs. Quit
The README only says the TUI must be running. Spell out that one TaskFlow owns the socket, what "Editor socket disabled" means, that `q` quits while the editor should hide its window, and the auto-save implications of 4.1/4.2.
- **Affected:** `README.md`, `neovim/README.md`, `docs/USER_MANUAL.md`
- **Complexity:** Low
- **Status:** ⬜ Not started.

---

## Suggested Priority Tiers

| Tier | Items | Rationale |
|------|-------|-----------|
| **Quick wins** | 1.6, 1.7, 2.2, 2.5, 2.6, 3.6, 3.10 | Low complexity, immediately visible value |
| **Core UX** | 1.1, 1.2, 1.3, 1.5, 1.8, 2.7, 2.8 | Reduces daily friction |
| **Power** | 1.4, 1.10, 2.1, 2.3, 2.4, 2.9, 2.10 | Differentiates from simpler tools |
| **Integrations** | 3.1–3.9 | Expands ecosystem reach |
| **Editor follow-ups** | 4.1–4.3, 4.12–4.14, 4.17 first (reliability, low effort); then 4.5, 4.7–4.9, 4.11, 4.15, 4.16; 4.4 last | Makes the 3.9 integration dependable before extending it |
