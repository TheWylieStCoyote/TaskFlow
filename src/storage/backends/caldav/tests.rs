//! CalDAV backend tests that never contact a server.
//!
//! Every test here works on the in-memory store and the local sidecar only.
//! `flush` is safe to call because no task is ever created, so `dirty_ids`
//! and `deleted_ids` stay empty and no VTODO push is attempted; `initialize`
//! is only exercised when the `caldav-sync` feature is off (the HTTP stubs
//! return an error immediately) so the feature-enabled build cannot reach out
//! to `config.url`.

use std::path::Path;

use tempfile::TempDir;

use super::{CalDavBackend, CalDavConfig, SIDECAR_EXTENSION};
use crate::domain::{
    AuditAction, AuditLogEntry, FieldChange, Filter, Project, SavedFilter, SortSpec, TaskId,
    TimeEntry, WorkLogEntry,
};
use crate::storage::{
    AuditLogRepository, ExportData, ProjectRepository, SavedFilterRepository, StorageBackend,
    StorageError, TimeEntryRepository, WorkLogRepository,
};

fn test_config() -> CalDavConfig {
    CalDavConfig {
        // Loopback with nothing listening; never used by these tests anyway.
        url: "http://127.0.0.1:1".to_string(),
        username: "alice".to_string(),
        password: "secret".to_string(),
        collection_path: "/dav/tasks/".to_string(),
    }
}

fn backend_with_sidecar(dir: &TempDir) -> CalDavBackend {
    CalDavBackend::from_config(test_config()).with_sidecar(dir.path().join("caldav.local.json"))
}

fn sample_filter(name: &str) -> SavedFilter {
    SavedFilter::new(name, Filter::default(), SortSpec::default())
}

// ── sidecar path ─────────────────────────────────────────────────────────────

#[test]
fn sidecar_path_sits_next_to_config_file() {
    let sidecar = CalDavBackend::sidecar_path_for(Path::new("/home/alice/.taskflow/caldav.toml"));
    assert_eq!(
        sidecar,
        Path::new("/home/alice/.taskflow/caldav.local.json")
    );
    assert_eq!(SIDECAR_EXTENSION, "local.json");
}

#[test]
fn new_from_config_file_enables_sidecar() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("caldav.toml");
    std::fs::write(
        &config_path,
        "url = \"http://127.0.0.1:1\"\nusername = \"a\"\npassword = \"b\"\ncollection_path = \"/c/\"\n",
    )
    .unwrap();

    let backend = CalDavBackend::new(&config_path).unwrap();

    assert_eq!(
        backend.sidecar_path(),
        Some(dir.path().join("caldav.local.json").as_path())
    );
}

#[test]
fn from_config_has_no_sidecar_until_requested() {
    let backend = CalDavBackend::from_config(test_config());
    assert!(backend.sidecar_path().is_none());
    // Saving with no sidecar is a silent no-op.
    backend.save_sidecar().unwrap();
}

// ── sidecar round trip ───────────────────────────────────────────────────────

#[test]
fn flush_writes_local_only_data_to_sidecar_and_reload_restores_it() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);

    let project = Project::new("Local project");
    backend.create_project(&project).unwrap();

    let task_id = TaskId::new();
    let mut entry = TimeEntry::start(task_id);
    entry.stop();
    backend.create_time_entry(&entry).unwrap();

    let log = WorkLogEntry::new(task_id, "Did a thing");
    backend.create_work_log(&log).unwrap();

    let filter = sample_filter("Urgent");
    backend.create_saved_filter(&filter).unwrap();

    let audit = AuditLogEntry::new(
        task_id,
        AuditAction::Modified,
        vec![FieldChange {
            field: "title".into(),
            old_value: "a".into(),
            new_value: "b".into(),
        }],
    );
    backend.append_audit_entry(&audit).unwrap();

    backend
        .set_pomodoro_stats(&crate::domain::PomodoroStats::default())
        .unwrap();

    assert!(backend.mem_dirty);
    backend.flush().unwrap();
    assert!(!backend.mem_dirty);

    let sidecar = dir.path().join("caldav.local.json");
    assert!(sidecar.exists(), "flush must write the sidecar");
    assert!(
        !dir.path().join("caldav.local.json.tmp").exists(),
        "atomic_write must not leave its temp file behind"
    );

    // Reconstruct from scratch and load the sidecar.
    let mut reloaded = backend_with_sidecar(&dir);
    assert!(reloaded.load_sidecar().unwrap());

    assert_eq!(reloaded.list_projects().unwrap().len(), 1);
    assert_eq!(reloaded.list_projects().unwrap()[0].name, "Local project");
    assert_eq!(
        reloaded.get_time_entry(&entry.id).unwrap().map(|e| e.id),
        Some(entry.id)
    );
    assert_eq!(
        reloaded.get_work_log(&log.id).unwrap().map(|l| l.content),
        Some("Did a thing".to_string())
    );
    let filters = reloaded.list_saved_filters().unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].id, filter.id);
    assert_eq!(filters[0].name, "Urgent");
    let audits = reloaded.list_audit_entries().unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].id, audit.id);
    assert_eq!(audits[0].changes.len(), 1);
    assert!(reloaded.mem.pomodoro_stats.is_some());
    assert!(!reloaded.mem_dirty, "a fresh load is not dirty");
}

#[test]
fn sidecar_never_contains_tasks() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    // Put a task straight into memory (bypassing the repo so nothing is queued
    // for a push) and a project through the repo.
    let task = crate::domain::Task::new("Server task");
    backend.mem.tasks.insert(task.id, task);
    backend.create_project(&Project::new("P")).unwrap();
    backend.flush().unwrap();

    let raw = std::fs::read_to_string(dir.path().join("caldav.local.json")).unwrap();
    let on_disk: ExportData = serde_json::from_str(&raw).unwrap();
    assert!(on_disk.tasks.is_empty(), "tasks belong to the server");
    assert_eq!(on_disk.projects.len(), 1);
}

#[test]
fn load_sidecar_keeps_tasks_already_in_memory() {
    let dir = TempDir::new().unwrap();
    let mut writer = backend_with_sidecar(&dir);
    writer.create_project(&Project::new("P")).unwrap();
    writer.flush().unwrap();

    let mut reader = backend_with_sidecar(&dir);
    let task = crate::domain::Task::new("From server");
    reader.mem.tasks.insert(task.id, task.clone());
    assert!(reader.load_sidecar().unwrap());

    assert_eq!(reader.mem.tasks.len(), 1);
    assert_eq!(reader.mem.tasks[&task.id].title, "From server");
    assert_eq!(reader.list_projects().unwrap().len(), 1);
}

#[test]
fn load_sidecar_without_file_is_a_noop() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    assert!(!backend.load_sidecar().unwrap());
    assert!(backend.mem.projects.is_empty());
}

#[test]
fn load_sidecar_rejects_corrupt_json() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("caldav.local.json"), "{ nope").unwrap();
    let mut backend = backend_with_sidecar(&dir);
    let err = backend.load_sidecar().unwrap_err();
    assert!(
        matches!(err, StorageError::Deserialization { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn flush_without_changes_does_not_touch_sidecar() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    backend.flush().unwrap();
    assert!(!dir.path().join("caldav.local.json").exists());
}

#[test]
fn second_flush_replaces_sidecar_contents() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    let project = Project::new("First");
    backend.create_project(&project).unwrap();
    backend.flush().unwrap();

    backend.delete_project(&project.id).unwrap();
    backend
        .create_saved_filter(&sample_filter("Only filter"))
        .unwrap();
    backend.flush().unwrap();

    let mut reloaded = backend_with_sidecar(&dir);
    reloaded.load_sidecar().unwrap();
    assert!(reloaded.list_projects().unwrap().is_empty());
    assert_eq!(reloaded.list_saved_filters().unwrap().len(), 1);
}

#[test]
fn pomodoro_setters_mark_memory_dirty() {
    let mut backend = CalDavBackend::from_config(test_config());
    assert!(!backend.mem_dirty);
    backend
        .set_pomodoro_config(&crate::domain::PomodoroConfig::default())
        .unwrap();
    assert!(backend.mem_dirty);
    backend.mem_dirty = false;
    backend.set_pomodoro_session(None).unwrap();
    assert!(backend.mem_dirty);
}

/// `initialize` must read the sidecar before syncing. The HTTP client is a
/// stub without the `caldav-sync` feature, so this never reaches the network;
/// with the feature it would try `config.url`, hence the gate.
#[cfg(not(feature = "caldav-sync"))]
#[test]
fn initialize_loads_sidecar_before_server_sync() {
    let dir = TempDir::new().unwrap();
    let mut writer = backend_with_sidecar(&dir);
    writer
        .create_saved_filter(&sample_filter("Persisted"))
        .unwrap();
    writer.flush().unwrap();

    let mut backend = backend_with_sidecar(&dir);
    backend.initialize().unwrap();
    assert_eq!(backend.list_saved_filters().unwrap().len(), 1);
}

// ── repository semantics ─────────────────────────────────────────────────────

#[test]
fn saved_filter_repository_round_trip() {
    let mut backend = CalDavBackend::from_config(test_config());
    let mut filter = sample_filter("Work");
    backend.create_saved_filter(&filter).unwrap();
    assert!(matches!(
        backend.create_saved_filter(&filter),
        Err(StorageError::AlreadyExists { .. })
    ));

    filter.name = "Work (renamed)".into();
    backend.update_saved_filter(&filter).unwrap();
    assert_eq!(
        backend.get_saved_filter(&filter.id).unwrap().unwrap().name,
        "Work (renamed)"
    );

    backend.delete_saved_filter(&filter.id).unwrap();
    assert!(backend.list_saved_filters().unwrap().is_empty());
    assert!(matches!(
        backend.delete_saved_filter(&filter.id),
        Err(StorageError::NotFound { .. })
    ));
    assert!(matches!(
        backend.update_saved_filter(&filter),
        Err(StorageError::NotFound { .. })
    ));
}

#[test]
fn audit_log_repository_appends_and_filters_by_task() {
    let mut backend = CalDavBackend::from_config(test_config());
    let a = TaskId::new();
    let b = TaskId::new();
    let first = AuditLogEntry::new(a, AuditAction::Created, vec![]);
    let second = AuditLogEntry::new(b, AuditAction::Created, vec![]);
    let third = AuditLogEntry::new(a, AuditAction::Completed, vec![]);
    backend.append_audit_entry(&first).unwrap();
    backend.append_audit_entry(&second).unwrap();
    backend.append_audit_entry(&third).unwrap();
    // Re-appending the same entry replaces rather than duplicates.
    backend.append_audit_entry(&third).unwrap();

    assert_eq!(backend.list_audit_entries().unwrap().len(), 3);
    let for_a = backend.get_audit_entries_for_task(&a).unwrap();
    assert_eq!(for_a.len(), 2);
    assert!(for_a[0].changed_at >= for_a[1].changed_at, "newest first");
}
