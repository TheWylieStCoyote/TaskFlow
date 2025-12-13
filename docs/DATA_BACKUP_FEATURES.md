# Data & Backup Features - Analysis

## Overview

Analysis of data and backup features for TaskFlow, including audit logs, encrypted backup, task genealogy, cloud sync, and historical analysis.

---

## Current Infrastructure

| Component | Location | Description |
|-----------|----------|-------------|
| Undo/Redo System | `src/app/undo/` | Bounded history stack (max 50 items) |
| Storage Backends | `src/storage/backends/` | JSON, YAML, SQLite, Markdown |
| Export System | `src/storage/export/` | CSV, ICS, DOT, Mermaid, HTML, Markdown |
| Import System | `src/storage/import/` | CSV, ICS with duplicate detection |
| Analytics Engine | `src/app/analytics/` | Trends, velocity, time tracking |

### Existing Timestamps
- Task: `created_at`, `updated_at`, `due_date`, `scheduled_date`, `completed_at`
- TimeEntry: `started_at`, `ended_at`, `duration_minutes`
- WorkLogEntry: `created_at`, `updated_at`

---

## Feature 1: Audit Log / History

**Goal**: Persistent history of all changes for accountability and recovery

### What Exists
- **UndoStack**: Bounded history with 11 action types:
  - Task CRUD (Created, Deleted, Modified with before/after snapshots)
  - Project CRUD operations
  - Time entry management
  - Work log entries
  - TimerSwitched (atomic multi-field operation)
- Undo actions store full before/after states via `Box<Task>`, `Box<TimeEntry>`
- Inverse operations calculated via macros

### What's Missing
- **Persistent audit trail**: Undo stack is in-memory, clears on app exit
- **Change attribution**: No user/session tracking
- **Audit queries**: No "all changes to task X" or "changes in date range Y"
- **Revert-to-timestamp**: Can't restore past state at arbitrary time

### What Needs to Be Added

```rust
pub struct AuditEntry {
    pub id: AuditEntryId,
    pub entity_type: EntityType,    // Task, Project, TimeEntry, etc.
    pub entity_id: String,
    pub operation: AuditOperation,  // Create, Update, Delete
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<String>,
    pub source: OperationSource,    // Keyboard, Pipe, AutoRecurrence
}

pub enum AuditOperation {
    Create,
    Update,
    Delete,
    StatusChange(TaskStatus, TaskStatus),
    Restore,
}
```

### New Modules
```
src/domain/audit/
├── audit_entry.rs       # AuditEntry struct
├── audit_log.rs         # Persistent audit trail interface
└── change_set.rs        # Grouped atomic changes

src/storage/audit/
├── mod.rs               # AuditRepository trait
├── audit_store.rs       # Persistent storage
└── query.rs             # Time-range and entity queries
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/storage/repository.rs` | Add AuditRepository trait |
| `src/storage/backends/sqlite/schema.rs` | Add audit_log table with indices |
| `src/app/model/storage.rs` | Emit audit entry on every change |
| `src/app/undo/action.rs` | Extend with session_id, timestamp |

### Complexity: Medium (25-35 hours)
- Schema and queries: 10-12 hours
- Integration with update handlers: 8-10 hours
- Query interface: 6-8 hours

---

## Feature 2: Encrypted Backup

**Goal**: Secure, encrypted backups of all TaskFlow data

### What Exists
- 4 serialization backends (JSON, YAML, SQLite, Markdown)
- All entities implement Serialize/Deserialize via serde
- ExportData struct consolidates all entity types
- export_all() / import_all() for bulk operations

### What's Missing
- **No encryption layer**: No crypto dependencies
- **No key management**: No password hashing or key derivation
- **No backup versioning**: No snapshots with timestamps
- **No integrity checking**: No checksums or signatures

### What Needs to Be Added

```rust
pub struct EncryptedBackup {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub salt: [u8; 16],           // Argon2 salt
    pub nonce: [u8; 12],          // AES-GCM nonce
    pub encrypted_data: Vec<u8>,  // Encrypted ExportData
    pub hmac: [u8; 32],           // Integrity verification
}

pub struct BackupConfig {
    pub encryption_enabled: bool,
    pub compression_enabled: bool,  // Optional gzip
    pub backup_directory: PathBuf,
    pub max_backups: usize,         // Rotation
}
```

### New Dependencies
```toml
aes-gcm = "0.10"     # AES-GCM encryption
argon2 = "0.5"       # Password-based key derivation
rand = "0.8"         # Secure random for salt/nonce
sha2 = "0.10"        # HMAC for integrity
flate2 = "1.0"       # Optional compression
```

### New Modules
```
src/crypto/
├── cipher.rs           # AES-GCM wrappers
├── key_management.rs   # Password → key derivation
└── integrity.rs        # HMAC-SHA256 verification

src/storage/backup/
├── mod.rs              # BackupFormat enum
├── encrypted_backup.rs # EncryptedBackup struct
├── compressor.rs       # Optional gzip
└── versioning.rs       # Backup snapshots
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/storage/mod.rs` | Add EncryptedBackend wrapper |
| `src/app/model/storage.rs` | Encryption/decryption steps |
| `src/app/message.rs` | EncryptionMessage for password UI |
| `Cargo.toml` | Add crypto dependencies |

### Complexity: Medium-High (40-50 hours)
- Crypto implementation: 12-15 hours
- Key derivation and password UI: 10-12 hours
- Backup versioning: 8-10 hours
- Integration and testing: 12-15 hours

---

## Feature 3: Task Genealogy / Lineage

**Goal**: Track and visualize task relationship history

### What Exists
- Task relationships:
  - `parent_task_id: Option<TaskId>` - Subtask hierarchy
  - `dependencies: Vec<TaskId>` - Blocking tasks
  - `next_task_id: Option<TaskId>` - Task chains
- Hierarchy support in `src/app/model/hierarchy.rs`
- Export to Graphviz and Mermaid for visualization

### What's Missing
- **No genealogy history**: Relationships are current state only
- **No conflict detection**: No circular dependency prevention
- **No impact analysis**: Can't see "what breaks if task X deleted"
- **No interactive visualization**: Static exports only

### What Needs to Be Added

```rust
pub struct RelationshipHistory {
    pub id: RelationshipHistoryId,
    pub task_id: TaskId,
    pub relationship_type: RelationshipType,
    pub related_task_id: TaskId,
    pub action: RelationshipAction,  // Added, Removed
    pub timestamp: DateTime<Utc>,
}

pub struct ImpactAnalysis {
    pub task_id: TaskId,
    pub blocked_tasks: Vec<TaskId>,      // Direct dependents
    pub cascade_affected: Vec<TaskId>,   // Transitive dependents
    pub subtasks: Vec<TaskId>,
    pub total_impact_score: u32,
}

pub fn find_critical_path(tasks: &[Task]) -> Vec<TaskId>;
pub fn detect_circular_dependency(from: TaskId, to: TaskId) -> bool;
```

### New Modules
```
src/domain/genealogy/
├── relationship_history.rs  # RelationshipChange tracking
├── constraint_validator.rs  # Circular deps, orphan detection
├── impact_analysis.rs       # What breaks if task X changes?
└── critical_path.rs         # Longest dependency chain

src/ui/components/genealogy/
├── tree_view.rs             # Visual task tree in TUI
└── impact_renderer.rs       # Shows cascade of changes
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/domain/task/mod.rs` | Add RelationshipHistory tracking |
| `src/app/model/mod.rs` | Add genealogy calculation cache |
| `src/ui/components/task_detail/` | Enhanced relationship display |
| `src/storage/repository.rs` | Query relationship history |

### Complexity: Medium (30-40 hours)
- Graph algorithms (DFS/BFS): 8-10 hours
- History storage: 8-10 hours
- Impact analysis: 8-10 hours
- UI rendering: 8-10 hours

---

## Feature 4: Cloud Sync

**Goal**: Synchronize TaskFlow data across devices

### What Exists
- StorageBackend trait is format-agnostic
- export_all() / import_all() for bulk operations
- MergeStrategy enum: Skip, Overwrite, CreateNew
- DuplicateDetector with fuzzy matching

### What's Missing
- **No remote connection**: No HTTP client
- **No conflict resolution**: No concurrent edit handling
- **No sync protocol**: No change log or version tracking
- **No offline support**: No queuing for offline changes

### What Needs to Be Added

```rust
pub struct SyncEngine {
    pub provider: Box<dyn CloudProvider>,
    pub state: SyncState,
    pub change_log: ChangeLog,
    pub conflict_resolver: ConflictResolver,
}

pub struct SyncState {
    pub last_sync: Option<DateTime<Utc>>,
    pub local_version: u64,
    pub remote_version: u64,
    pub pending_changes: Vec<ChangeEntry>,
}

pub trait CloudProvider {
    async fn push(&self, data: &ExportData) -> Result<()>;
    async fn pull(&self) -> Result<ExportData>;
    async fn get_version(&self) -> Result<u64>;
}

pub enum ConflictResolution {
    LocalWins,
    RemoteWins,
    LastWriteWins,
    ThreeWayMerge,
    PromptUser,
}
```

### New Dependencies
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### New Modules
```
src/sync/
├── mod.rs               # SyncEngine orchestrator
├── cloud_provider.rs    # CloudProvider trait
├── change_log.rs        # ChangeLog tracking
├── conflict_resolver.rs # Conflict detection/resolution
├── state.rs             # SyncState management
└── providers/
    ├── dropbox.rs       # Dropbox integration
    ├── s3.rs            # AWS S3 integration
    └── webdav.rs        # Generic WebDAV support
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/storage/repository.rs` | Add SyncState tracking |
| `src/app/model/mod.rs` | Sync state, queue, conflicts |
| `src/app/message.rs` | SyncMessage variants |
| `src/app/update/` | Sync handlers |
| `Cargo.toml` | Add reqwest, tokio |

### Complexity: High (80-120 hours)
- Sync engine: 20-25 hours
- Conflict resolution: 20-25 hours
- Cloud provider integration: 20-25 hours
- Offline queue and retry: 15-20 hours
- Testing: 15-20 hours

---

## Feature 5: Historical Analysis

**Goal**: Analyze productivity trends over time

### What Exists
- **Comprehensive analytics engine** (`src/app/analytics/`):
  - Completion trends over time
  - Velocity metrics (weekly/monthly)
  - Time analytics by project, day of week, hour
  - `ProductivityInsights`: streaks, avg_tasks_per_day, best_day, peak_hour
- Habit tracking with historical check-ins
- Goal progress history: `progress_history: Vec<(DateTime, u32)>`
- Report generation to HTML/Markdown

### What's Missing
- **No immutable event log**: Analytics computed on-the-fly
- **No time-series database**: O(n) scans for queries
- **No forecasting**: No velocity-based ETA or trend extrapolation
- **No retention policies**: No archival of old data

### What Needs to Be Added

```rust
pub struct DomainEvent {
    pub id: EventId,
    pub event_type: EventType,
    pub entity_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

pub struct TimeSeries {
    pub metric: MetricType,
    pub bucket: TimeBucket,     // Day, Week, Month
    pub values: Vec<(DateTime<Utc>, f64)>,
}

pub struct BurndownPrediction {
    pub predicted_completion_date: Option<NaiveDate>,
    pub confidence_level: f64,
    pub scenarios: Vec<Scenario>,  // Optimistic, Realistic, Pessimistic
}

pub struct AnomalyDetector {
    fn detect(&self, series: &TimeSeries) -> Vec<Anomaly>;
}
```

### New Modules
```
src/domain/events/
├── domain_event.rs     # DomainEvent struct
├── event_store.rs      # EventStore trait
└── snapshot.rs         # Periodic snapshots

src/analytics/history/
├── time_series.rs      # TimeSeries types
├── aggregation.rs      # Pre-aggregate by day/week/month
├── forecaster.rs       # Velocity-based ETA
└── anomaly_detector.rs # Identify unusual patterns
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/storage/backends/sqlite/schema.rs` | Add events table, time-series buckets |
| `src/app/model/storage.rs` | Emit domain events on changes |
| `src/app/analytics/mod.rs` | Accept EventStore for historical queries |
| `src/domain/task/mod.rs` | Add deleted_at for soft deletes |

### Complexity: Medium-High (50-70 hours)
- Event sourcing: 15-20 hours
- Time-series aggregation: 10-12 hours
- Forecasting algorithms: 12-15 hours
- Anomaly detection: 8-10 hours
- Integration: 10-12 hours

---

## Implementation Priority

| Feature | Effort | Value | Risk | Priority |
|---------|--------|-------|------|----------|
| Audit Log | 25-35h | High | Low | 1st (foundation) |
| Encrypted Backup | 40-50h | High | Medium | 2nd (security) |
| Historical Analysis | 50-70h | Medium | Medium | 3rd (builds on audit) |
| Task Genealogy | 30-40h | Medium | Low | 4th (enhancement) |
| Cloud Sync | 80-120h | High | High | 5th (complex) |

**Total Estimated Effort**: 225-315 hours

---

## Recommended Implementation Order

### Phase 1: Audit Log (Foundation)
- Enables other features to track changes
- Relatively self-contained
- Provides immediate user value
- **Essential for compliance/accountability**

### Phase 2: Encrypted Backup
- Builds on existing serialization
- Essential for data security
- No dependencies on other phases
- **Critical for sensitive data**

### Phase 3: Historical Analysis
- Depends on audit log for complete picture
- Time-series infrastructure useful for Phase 4
- High value with good foundation
- **Leverages existing analytics engine**

### Phase 4: Task Genealogy (Enhancement)
- Standalone feature
- Leverages existing relationship fields
- Polish vs core functionality
- **Graph algorithms are straightforward**

### Phase 5: Cloud Sync (Ambitious)
- Most complex
- Depends on audit log + encrypted backup
- Consider: start with single cloud provider (Dropbox)
- **Requires careful conflict resolution design**

---

## Key Architectural Strengths

1. **Storage Abstraction**: Multiple backends already pluggable
2. **Serialization**: Serde infrastructure mature across all entities
3. **Timestamps**: All entities have created_at/updated_at
4. **Repository Pattern**: Clear CRUD abstraction for new entities
5. **Message-based Updates**: Easy to emit events from update handlers
6. **Undo System**: Foundation for tracking changes exists

---

## Key Files Reference

**Undo System:**
- `src/app/undo/mod.rs` - UndoStack implementation
- `src/app/undo/action.rs` - UndoAction enum (11 types)

**Storage Layer:**
- `src/storage/repository.rs` - Repository traits
- `src/storage/backends/` - 4 backend implementations
- `src/storage/export/` - Export capabilities

**Analytics:**
- `src/app/analytics/` - Trends, velocity, time analytics
- `src/domain/analytics.rs` - ProductivityInsights struct

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
- [WORKFLOW_AUTOMATION_FEATURES.md](WORKFLOW_AUTOMATION_FEATURES.md) - Related: Audit log for tracking
