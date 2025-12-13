# GitHub/GitLab Issue Sync Feature - Analysis

## Overview

This document outlines what would be involved in implementing bidirectional synchronization between TaskFlow and GitHub/GitLab issues.

---

## Current Architecture Strengths

TaskFlow is well-positioned for this feature:

| Component | Why It Helps |
|-----------|--------------|
| `task.custom_fields` | HashMap for storing external metadata (issue ID, URL, sync timestamp) - no schema changes needed |
| Pipe Interface | JSON stdin/stdout protocol already supports Create/Update/Import operations |
| Import System | Merge strategies (Skip/Overwrite/CreateNew) handle sync conflicts |
| Git Integration | `src/domain/git/` provides patterns for external system linking |
| TEA Architecture | Message system ready for async sync operations |

---

## New Dependencies Required

```toml
# Cargo.toml additions
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
octocrab = "0.41"      # GitHub API client
# OR for GitLab:
gitlab = "0.1609"      # GitLab API client

tokio = { version = "1", features = ["rt-multi-thread", "macros"] }  # async runtime
```

---

## Data Mapping

```
GitHub Issue              →  TaskFlow Task
─────────────────────────────────────────
issue.number              →  custom_fields["external_id"]
issue.html_url            →  custom_fields["external_url"]
issue.title               →  task.title
issue.body                →  task.description
issue.state (open/closed) →  task.status (todo/done)
issue.labels[]            →  task.tags[]
issue.milestone           →  task.project_id (optional)
issue.assignees[]         →  custom_fields["assignees"]
issue.created_at          →  task.created_at
issue.updated_at          →  custom_fields["external_updated_at"]
```

---

## Implementation Phases

### Phase 1: Foundation (MVP - Read Only)

**Goal**: Fetch GitHub issues and create tasks

| Task | Files | Description |
|------|-------|-------------|
| 1.1 | `Cargo.toml` | Add reqwest, octocrab, tokio dependencies |
| 1.2 | `src/config/settings.rs` | Add `[sync.github]` config section (token, repos) |
| 1.3 | `src/domain/external_issue.rs` | New module: `ExternalIssue`, `ExternalProvider` types |
| 1.4 | `src/integrations/github.rs` | GitHub API client wrapper |
| 1.5 | `src/storage/import/github.rs` | Import handler: issues → tasks |
| 1.6 | CLI command | `taskflow sync github fetch` |

**Estimated scope**: ~800-1200 lines of code

---

### Phase 2: Bidirectional Sync

**Goal**: Push task completions back to GitHub

| Task | Files | Description |
|------|-------|-------------|
| 2.1 | `src/app/message/sync.rs` | New `SyncMessage` enum |
| 2.2 | `src/app/update/sync.rs` | Sync message handlers |
| 2.3 | `src/integrations/github.rs` | Add `close_issue()`, `update_issue()` |
| 2.4 | Sync logic | Track dirty tasks, push on save |
| 2.5 | Conflict resolution | Last-write-wins or user prompt |

---

### Phase 3: UI Integration

**Goal**: Sync controls in TUI

| Task | Files | Description |
|------|-------|-------------|
| 3.1 | Keybinding | `Alt+S` or similar for manual sync |
| 3.2 | Status bar | Show sync status, last sync time |
| 3.3 | Task detail | Show external issue link, sync status |
| 3.4 | Config view | GitHub token setup UI |

---

### Phase 4: GitLab Support

**Goal**: Same functionality for GitLab

| Task | Files | Description |
|------|-------|-------------|
| 4.1 | `src/integrations/gitlab.rs` | GitLab API client |
| 4.2 | Provider trait | Abstract GitHub/GitLab differences |
| 4.3 | Config | `[sync.gitlab]` section |

---

## Configuration Design

```toml
# ~/.config/taskflow/config.toml

[sync]
auto_sync = false
sync_interval_minutes = 15

[sync.github]
enabled = true
token_env = "GITHUB_TOKEN"           # or token_file path
repositories = ["owner/repo1", "owner/repo2"]
sync_labels = true
sync_closed = false                  # import closed issues?
label_filter = ["bug", "enhancement"] # optional: only these labels

[sync.gitlab]
enabled = false
token_env = "GITLAB_TOKEN"
base_url = "https://gitlab.com"      # for self-hosted
projects = ["owner/project"]
```

---

## Key Files to Modify/Create

### New Files

```
src/
├── integrations/
│   ├── mod.rs
│   ├── github.rs          # GitHub API wrapper
│   └── gitlab.rs          # GitLab API wrapper (Phase 4)
├── domain/
│   └── external_issue.rs  # ExternalIssue, SyncMetadata types
├── storage/
│   └── import/
│       └── github.rs      # Issue → Task conversion
├── app/
│   ├── message/
│   │   └── sync.rs        # SyncMessage enum
│   └── update/
│       └── sync.rs        # Sync handlers
└── bin/taskflow/commands/
    └── sync.rs            # CLI: taskflow sync
```

### Modified Files

```
src/config/settings.rs     # Add sync config
src/app/message/mod.rs     # Add Sync variant
src/app/update/mod.rs      # Route sync messages
src/bin/taskflow/main.rs   # Add sync subcommand
Cargo.toml                 # Add dependencies
```

---

## Complexity Assessment

| Aspect | Complexity | Notes |
|--------|------------|-------|
| GitHub API | Low | octocrab handles auth, pagination |
| Data mapping | Low | Straightforward field mapping |
| Conflict resolution | Medium | Need strategy for concurrent edits |
| Async integration | Medium | Current app is sync, needs tokio |
| UI integration | Medium | New views, status indicators |
| GitLab support | Low | Similar to GitHub once abstracted |

**Overall**: Medium-Large feature (~2-4 weeks for full implementation)

---

## Alternative: External Sync Tool

Instead of building into TaskFlow, could create a standalone sync daemon:

```bash
# Separate binary that uses pipe interface
taskflow-github-sync --config ~/.config/taskflow/github-sync.toml
```

**Pros**: Simpler, no async in main app, can run as cron job
**Cons**: Separate install, less integrated UX

---

## Design Decisions to Make

1. **Scope**: Start with GitHub only, or design for multi-provider from start?
2. **Sync direction**: Read-only first, or bidirectional from start?
3. **Conflict strategy**: Last-write-wins, or prompt user?
4. **Auto-sync**: Background sync, or manual trigger only?
5. **Architecture**: Built-in, or external sync tool using pipe interface?

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Other potential features
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture overview
