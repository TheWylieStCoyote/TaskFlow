# Integration Features - Analysis

## Overview

Analysis of external integration features for TaskFlow, including imports, notifications, calendar sync, and note-taking plugins.

**Note**: GitHub/GitLab sync is documented separately in [GITHUB_GITLAB_SYNC.md](GITHUB_GITLAB_SYNC.md).

---

## Current Capabilities

### Import Formats
| Format | Location | Capabilities |
|--------|----------|--------------|
| CSV | `src/storage/import/csv.rs` | Tasks with all fields |
| ICS | `src/storage/import/ics.rs` | VTODO and VEVENT parsing |

### Export Formats
| Format | Location | Capabilities |
|--------|----------|--------------|
| CSV | `src/storage/export/csv.rs` | Full task export |
| ICS | `src/storage/export/ics.rs` | VTODO with time blocks |
| DOT | `src/storage/export/dot.rs` | Graphviz dependency graphs |
| Mermaid | `src/storage/export/mermaid.rs` | Markdown flowcharts |
| HTML/MD | `src/storage/export/report/` | Analytics reports |

### Pipe Interface
- Location: `src/bin/taskflow/commands/pipe/`
- JSON stdin/stdout protocol
- Operations: list, get, create, update, delete, export, import
- Entities: task, project, time_entry, work_log, habit, goal, tag

---

## Feature 1: Todoist/Notion Import

**Goal**: One-click migration from popular task managers

### What Needs to Be Added

```rust
// New module: src/storage/import/todoist.rs
pub struct TodoistImporter {
    api_token: String,
    client: reqwest::Client,
}

impl TodoistImporter {
    pub async fn import_all(&self) -> Result<ImportResult> {
        let projects = self.fetch_projects().await?;
        let tasks = self.fetch_tasks().await?;
        let labels = self.fetch_labels().await?;

        // Map to TaskFlow entities
        // Handle relationships
        // Return import result
    }
}
```

### Data Mapping

| Todoist | TaskFlow |
|---------|----------|
| Sections | Projects |
| Labels | Tags |
| Priority (1-4) | Priority enum |
| Due dates + recurring | Recurrence |
| Subtasks | parent_task_id |

### New Dependencies
```toml
reqwest = { version = "0.11", features = ["json"] }
oauth2 = "4.4"
tokio = { version = "1", features = ["full"] }
```

### Complexity: High (8-12 weeks)
- API client: 2 weeks
- Data mapping: 2 weeks
- Conflict resolution: 1-2 weeks
- Testing & UI: 2-3 weeks

---

## Feature 2: Desktop Notifications

**Goal**: System alerts for due tasks, overdue items, Pomodoro milestones

### What Exists
- Config flags exist but not implemented: `show_overdue_alert`, `show_due_today`
- Pomodoro state tracking

### What Needs to Be Added

```rust
// New module: src/notifications/mod.rs
pub struct NotificationManager {
    config: NotificationConfig,
    last_check: DateTime<Utc>,
}

pub enum NotificationType {
    DueToday(Vec<Task>),
    Overdue(Vec<Task>),
    PomodoroPhaseEnd(PomodoroPhase),
    HabitCheckIn(Habit),
    GoalDeadline(Goal),
}

impl NotificationManager {
    pub fn check_and_notify(&mut self, model: &Model) {
        // Query tasks due today
        // Query overdue tasks
        // Check Pomodoro state
        // Send notifications
    }
}
```

### Configuration
```toml
[notifications]
enabled = true
show_overdue_alert = true
show_due_today = true
show_pomodoro_notifications = true
notification_time = "09:00"
overdue_check_interval = 3600  # seconds
```

### New Dependencies
```toml
notify-rust = "4.10"  # Cross-platform notifications
```

### Complexity: Medium (3-4 weeks)
- Library integration: 1 week
- Time-based trigger logic: 1 week
- Config extensions: 3-4 days
- Testing: 1 week

---

## Feature 3: CalDAV/Google Calendar Sync

**Goal**: Two-way sync with calendar apps for time-blocking visibility

### What Exists
- ICS export/import works
- Time-blocked tasks have DTSTART/DTEND

### What Needs to Be Added

```rust
// New module: src/storage/sync/caldav.rs
pub struct CalDAVClient {
    url: String,
    username: String,
    password: String,
}

pub enum SyncStrategy {
    OneWayExport,    // TaskFlow → Calendar
    OneWayImport,    // Calendar → TaskFlow
    TwoWay,          // Bidirectional
}

pub struct CalDAVConfig {
    url: String,
    credentials: Credentials,
    sync_strategy: SyncStrategy,
    last_sync: DateTime<Utc>,
    conflict_resolution: ConflictResolution,
}

impl CalDAVClient {
    pub async fn push_events(&self, tasks: &[Task]) -> Result<()>;
    pub async fn pull_events(&self) -> Result<Vec<CalendarEvent>>;
    pub async fn sync(&mut self, model: &mut Model) -> Result<SyncResult>;
}
```

### Google Calendar API
```rust
pub struct GoogleCalendarClient {
    oauth_token: String,
    calendar_id: String,
}
```

### Complexity: Very High (10-16 weeks)
- CalDAV client: 3-4 weeks
- Sync state management: 2-3 weeks
- Google Calendar: 2-3 weeks
- Conflict resolution: 2 weeks
- Testing: 2 weeks

### Risk: Critical
- Network failures
- Auth token expiry
- Sync conflicts and data loss potential

---

## Feature 4: Obsidian/Logseq Plugin

**Goal**: Embed TaskFlow tasks in markdown notes with bidirectional sync

### Architecture Options

**Option A: Pipe Interface Only (Simpler)**
- Obsidian plugin calls `taskflow pipe` via subprocess
- No extra Rust code needed
- **Effort**: 3-4 weeks (just plugin)

**Option B: REST API Server (Better UX)**
```rust
// New module: src/api/server.rs
pub async fn start_server(model: Arc<Mutex<Model>>, port: u16) {
    let app = Router::new()
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:id", get(get_task).put(update_task));

    axum::Server::bind(&addr).serve(app.into_make_service()).await;
}
```

**Effort**: 6-8 weeks total

**Option C: File Watcher (Markdown-Native)**
- TaskFlow monitors markdown files for `<!-- taskflow -->` markers
- Auto-updates embedded task lists
- **Effort**: 8-10 weeks

### Obsidian Plugin Structure (TypeScript)
```
obsidian-taskflow-plugin/
├── main.ts              # Plugin entry point
├── taskflow-client.ts   # Pipe/HTTP client
├── inline-renderer.ts   # Render tasks in notes
├── command.ts           # "Insert task" command
└── settings.ts          # Plugin configuration
```

### Embed Format
```markdown
# My Project

<!-- taskflow-start: project:Work,priority:high -->
- [ ] Task 1
- [ ] Task 2
<!-- taskflow-end -->
```

### Complexity: Medium-High (4-12 weeks depending on approach)

---

## Dependencies Summary

| Feature | New Crates | Async Required |
|---------|-----------|----------------|
| Todoist Import | `reqwest`, `oauth2` | Yes |
| Desktop Notifications | `notify-rust` | No |
| CalDAV Sync | `reqwest`, `ical` | Yes |
| Google Calendar | `reqwest`, OAuth2 | Yes |
| REST API Server | `axum`, `tower`, `tokio` | Yes |

---

## Implementation Priority

| Feature | Effort | Value | Risk | Priority |
|---------|--------|-------|------|----------|
| Desktop Notifications | 3-4w | High | Low | 1st |
| REST API Server | 2-3w | High (enables plugins) | Low | 2nd |
| Todoist Import | 8-12w | High | Medium | 3rd |
| Obsidian Plugin | 3-5w | Medium | Low | 4th |
| CalDAV Sync | 10-16w | Medium | Critical | 5th |

**Recommended Start**: Desktop Notifications + REST API Server (5-7 weeks)

---

## Key Files to Create

```
src/notifications/           # Desktop notifications
├── mod.rs
├── provider.rs              # Platform-specific
└── config.rs

src/api/                     # REST API server
├── mod.rs
├── routes.rs
└── middleware.rs

src/storage/import/
├── todoist.rs              # Todoist import
└── notion.rs               # Notion import

src/storage/sync/
├── caldav.rs               # CalDAV sync
└── google_calendar.rs      # Google Calendar
```

---

## See Also

- [GITHUB_GITLAB_SYNC.md](GITHUB_GITLAB_SYNC.md) - GitHub/GitLab issue sync
- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
