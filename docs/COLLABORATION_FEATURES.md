# Collaboration Features - Analysis

## Overview

Analysis of collaboration features for TaskFlow, including task comments, public sharing, and shared projects.

---

## Current Architecture

| Component | Location | Description |
|-----------|----------|-------------|
| WorkLogEntry | `src/domain/work_log.rs` | Chronological log with content, timestamps |
| Export System | `src/storage/export/` | CSV, ICS, HTML, Markdown, DOT, Mermaid |
| Pipe Interface | `src/bin/taskflow/commands/pipe/` | JSON stdin/stdout protocol |
| Filter DSL | `src/domain/filter_dsl/` | 30+ filterable fields |

**Critical Note**: No user/identity system currently exists. TaskFlow is designed as a single-user TUI application.

---

## Feature 1: Task Comments/Discussions

**Goal**: Add threaded comments to tasks for context and collaboration

### What Exists
- `WorkLogEntry` provides chronological log pattern:
  - `id: WorkLogEntryId` (UUID)
  - `task_id: TaskId` (linked to task)
  - `content: String` (multi-line text)
  - `created_at`, `updated_at` timestamps
  - Serialization across all 4 backends
- Model stores `work_logs: HashMap<WorkLogEntryId, WorkLogEntry>`
- Pipe interface can create/list work logs

### What Needs to Be Added

```rust
// Extend or create alongside WorkLogEntry
pub struct Comment {
    pub id: CommentId,
    pub task_id: TaskId,
    pub content: String,
    pub author_id: Option<UserId>,     // NEW
    pub author_name: String,           // NEW
    pub reply_to: Option<CommentId>,   // Threading
    pub is_resolved: bool,             // Discussion closure
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct User {
    pub user_id: UserId,
    pub name: String,
    pub email: Option<String>,
    pub color: Option<String>,  // For UI rendering
}
```

### New Messages
```rust
TaskMessage::AddComment(TaskId, String)
TaskMessage::ReplyToComment(CommentId, String)
TaskMessage::ResolveThread(CommentId)
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/domain/mod.rs` | Export Comment, User types |
| `src/domain/work_log.rs` | Add Comment alongside or refactor |
| `src/storage/repository.rs` | Add CommentRepository trait |
| `src/storage/backends/*.rs` | Implement in JSON, YAML, SQLite, Markdown |
| `src/app/model/mod.rs` | Add comments, users HashMaps |
| `src/app/message/task.rs` | Add comment operations |
| `src/ui/components/task_detail/` | Render comment thread section |

### Complexity: Medium (15-20 hours)
- WorkLogEntry pattern already established
- No authentication needed (single-user with optional author name)
- Threading is straightforward (parent reference + ordering)

---

## Feature 2: Public Task Lists / Read-Only Sharing

**Goal**: Generate shareable views of task lists

### What Exists
- Robust export system:
  - `export_to_csv()` - Task data to CSV
  - `export_to_ics()` - iCalendar format
  - `export_to_dot()` - Graphviz dependency graphs
  - `export_to_mermaid()` - Mermaid diagrams
  - `export_report_to_html()` - Analytics reports
- Filter DSL for scoping what's visible
- SavedFilter for persistent filter configurations

### What Needs to Be Added

```rust
pub struct ShareLink {
    pub share_id: Uuid,
    pub project_id: Option<ProjectId>,
    pub filter: Option<Filter>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub include_completed: bool,
    pub password_hash: Option<String>,
    pub access_count: u32,
}

pub struct ShareMetadata {
    pub title: String,
    pub description: String,
    pub owner_name: String,
    pub theme: Option<String>,
}
```

### Implementation Options

**Option A: Static HTML Export (Simpler)**
- Generate HTML file from filtered task list
- Include CSS styling from theme
- Shareable via file hosting
- **Effort**: 15-20 hours

**Option B: Web Server (Better UX)**
- Minimal HTTP server (axum)
- Serve share links dynamically
- Password protection support
- **Effort**: 40-60 hours

### Key Files to Create/Modify

| File | Change |
|------|--------|
| `src/domain/share.rs` (NEW) | ShareLink, ShareMetadata types |
| `src/storage/repository.rs` | Add ShareRepository trait |
| `src/storage/export/html.rs` | Enhanced HTML with share metadata |
| `src/bin/taskflow/commands/share.rs` (NEW) | CLI for generating shares |
| `src/config/settings.rs` | Share base URL configuration |

### Complexity: Medium (20-30 hours for static, 50-70 hours for web server)

---

## Feature 3: Shared Projects / Multi-User

**Goal**: Multiple users collaborating on the same project

### Critical Finding: No User Infrastructure

Current state:
- No `UserId`, `User`, or user/owner fields on any entity
- No concept of "current user" in Model
- No permissions or access control
- Single-user TUI architecture throughout

### What Would Need to Be Created

**1. Identity System**
```rust
pub struct User {
    pub user_id: UserId,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

pub enum UserRole {
    Owner,      // All permissions
    Editor,     // Create/modify/delete tasks
    Viewer,     // Read-only
    Commenter,  // Can only add comments
}
```

**2. Permission System**
```rust
pub enum Permission {
    CanViewProject, CanEditProject, CanDeleteProject,
    CanCreateTask, CanEditTask, CanDeleteTask,
    CanAddMember, CanRemoveMember,
    CanShare, CanExport,
}

pub struct AccessControl {
    fn check_permission(user: UserId, resource: ProjectId, action: Permission) -> bool;
}
```

**3. Project Enhancement**
```rust
// Extended Project fields
pub owner_id: UserId,
pub members: HashMap<UserId, UserRole>,
pub visibility: ProjectVisibility,  // Private, Team, Public
pub audit_log: Vec<AuditEntry>,
```

**4. Workspace Concept**
```rust
pub struct Workspace {
    pub workspace_id: Uuid,
    pub name: String,
    pub owner_id: UserId,
    pub members: HashMap<UserId, UserRole>,
    pub projects: Vec<ProjectId>,
}
```

**5. Sync Layer (Required for actual multi-user)**
```rust
// Option A: Local Network
pub struct SyncServer { /* runs on one machine */ }
pub struct SyncClient { /* connects to server */ }

// Option B: Cloud Sync
pub struct CloudSync {
    oauth_token: String,
    conflict_resolver: ConflictResolver,
}
```

### Key Files to Create

```
src/domain/user.rs              # User, UserRole, Permission
src/domain/workspace.rs         # Workspace, TeamInvite
src/storage/permission.rs       # Permission checking layer
src/sync/                       # Sync/conflict resolution
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/domain/task/mod.rs` | Add assigned_to, created_by fields |
| `src/domain/project.rs` | Add owner_id, members, visibility |
| All storage backends | Implement new repository traits |
| `src/app/model/mod.rs` | Add auth_context, permissions |
| `src/config/settings.rs` | Add current_user_id, workspace_id |

### Complexity: Very High (200-400 hours)

**Breakdown:**
- User/Identity system: 40-60 hours
- Permission checking: 30-40 hours
- Project/Task modifications: 20-30 hours
- Repository implementations: 80-120 hours
- Sync/Conflict resolution: 100-200 hours
- UI enhancements: 50-80 hours

---

## Implementation Priority

| Feature | Effort | Value | Risk | Priority |
|---------|--------|-------|------|----------|
| Task Comments | 15-20h | Medium | Low | 1st |
| Public Sharing (Static) | 20-30h | Medium | Low | 2nd |
| Public Sharing (Web Server) | 50-70h | Medium | Medium | 3rd |
| Shared Projects | 200-400h | High | Very High | Defer |

**Recommendation**: Start with Task Comments, which leverages the existing WorkLogEntry pattern and provides immediate value without architectural changes.

---

## Phased Approach

### Phase 1: Task Comments (1-2 weeks)
- Add Comment type alongside WorkLogEntry
- Minimal User type (just name, optional)
- UI in task detail modal
- **Low risk, incremental**

### Phase 2: Public Sharing (1-2 weeks)
- Static HTML export with styling
- Share via file hosting
- **Independent of Phase 1**

### Phase 3: Shared Projects (only if needed)
- Requires Phase 1 infrastructure
- Major architectural refactoring
- Significant testing effort
- **Consider: Is TaskFlow meant to be multi-user?**

---

## Key Files Reference

**Core Domain:**
- `src/domain/work_log.rs` - WorkLogEntry pattern (290 lines)
- `src/domain/project.rs` - Project entity (100+ lines)

**Storage Layer:**
- `src/storage/repository.rs` - Repository traits
- `src/storage/backends/` - 4 backend implementations

**Export Infrastructure:**
- `src/storage/export/` - Existing export capabilities

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
- [INTEGRATION_FEATURES.md](INTEGRATION_FEATURES.md) - Related: REST API for plugins
