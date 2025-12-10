# ADR-0002: Storage Backend Abstraction

## Status

Accepted

## Context

TaskFlow needs to persist data (tasks, projects, time entries) to disk. Users have different preferences:
- Some want human-readable files they can edit manually (YAML, Markdown)
- Some want compact, fast storage (JSON)
- Some want robust querying capabilities (SQLite)
- Some want git-friendly individual files (Markdown)

We need a flexible storage layer that:
- Supports multiple file formats
- Allows users to choose their preferred backend
- Doesn't couple application logic to storage implementation
- Enables easy addition of new backends

## Decision

We use a trait-based storage abstraction:

```rust
pub trait StorageBackend: Send + Sync {
    // Task operations
    fn create_task(&mut self, task: &Task) -> StorageResult<()>;
    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>>;
    fn update_task(&mut self, task: &Task) -> StorageResult<()>;
    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()>;
    fn list_tasks(&self) -> StorageResult<Vec<Task>>;

    // Similar for projects, time entries, habits, etc.
    // ...

    // Lifecycle
    fn initialize(&mut self) -> StorageResult<()>;
    fn flush(&mut self) -> StorageResult<()>;
}
```

Each backend implements this trait:
- `JsonBackend`: Single JSON file, fast serialization
- `YamlBackend`: Single YAML file, human-readable
- `SqliteBackend`: SQLite database, powerful queries
- `MarkdownBackend`: Individual .md files with YAML frontmatter

Backend selection is done via CLI flag or config:
```bash
taskflow --backend sqlite
```

## Consequences

### Positive

- **Flexibility**: Users choose storage format based on needs
- **Testability**: Can mock backends for testing
- **Extensibility**: New backends (e.g., cloud sync) follow same pattern
- **Separation of concerns**: App logic doesn't know about file formats
- **Migration path**: Can add import/export between backends

### Negative

- **Complexity**: More code than a single format
- **Consistency**: Must ensure all backends behave identically
- **Feature parity**: Some backends may not support all features efficiently
- **Testing burden**: Each backend needs thorough testing

### Implementation Notes

- All backends share common filter utilities (`filter_utils.rs`)
- SQLite uses prepared statements for performance
- Markdown backend maintains in-memory cache for speed
- JSON/YAML backends load entire dataset into memory
