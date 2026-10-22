//! CalDAV backend tests that never reach a server.
//!
//! Every test here works on the in-memory store and the local sidecar only.
//! Tests that queue task pushes or deletes and then `flush` behave as an
//! offline client: without the `caldav-sync` feature the HTTP stubs fail
//! immediately, and with it `config.url` points at a loopback port with
//! nothing listening, so the connection is refused at once. `initialize` is
//! only exercised when the feature is off.

use std::path::Path;

use tempfile::TempDir;

use chrono::{DateTime, TimeZone, Utc};

use super::{CalDavBackend, CalDavConfig, Sidecar, SIDECAR_EXTENSION};
use crate::domain::{
    AuditAction, AuditLogEntry, FieldChange, Filter, Project, SavedFilter, SortSpec, Task, TaskId,
    TimeEntry, WorkLogEntry,
};
use crate::storage::{
    AuditLogRepository, ExportData, ProjectRepository, SavedFilterRepository, StorageBackend,
    StorageError, TaskRepository, TimeEntryRepository, WorkLogRepository,
};

fn fixed_time(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 22, hour, 0, 0).unwrap()
}

fn read_sidecar(dir: &TempDir) -> Sidecar {
    let raw = std::fs::read_to_string(dir.path().join("caldav.local.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

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
fn sidecar_caches_server_tasks_without_queuing_them() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    // A task straight into memory stands for one fetched from the server
    // (nothing queued for a push); the project goes through the repo.
    let task = Task::new("Server task");
    backend.mem.tasks.insert(task.id, task.clone());
    backend.create_project(&Project::new("P")).unwrap();
    backend.flush().unwrap();

    let on_disk = read_sidecar(&dir);
    assert_eq!(on_disk.data.tasks.len(), 1, "the sidecar caches tasks");
    assert_eq!(on_disk.data.tasks[&task.id].title, "Server task");
    assert!(on_disk.pending_push.is_empty());
    assert!(on_disk.pending_delete.is_empty());
    assert_eq!(on_disk.data.projects.len(), 1);
}

#[test]
fn load_sidecar_prefers_server_copy_over_cache_unless_pending() {
    let dir = TempDir::new().unwrap();
    let mut writer = backend_with_sidecar(&dir);
    writer.create_project(&Project::new("P")).unwrap();
    let mut cached = Task::new("Cached title");
    let cached_id = cached.id;
    cached.updated_at = fixed_time(1);
    writer.mem.tasks.insert(cached_id, cached);
    let edited = Task::new("Edited offline");
    writer.create_task(&edited).unwrap();
    writer.flush().unwrap();

    let mut reader = backend_with_sidecar(&dir);
    let mut from_server = Task::new("Server title");
    from_server.id = cached_id;
    reader.mem.tasks.insert(cached_id, from_server);
    let mut stale_edit = Task::new("Server copy of the offline edit");
    stale_edit.id = edited.id;
    reader.mem.tasks.insert(edited.id, stale_edit);
    assert!(reader.load_sidecar().unwrap());

    assert_eq!(reader.mem.tasks.len(), 2);
    assert_eq!(reader.mem.tasks[&cached_id].title, "Server title");
    assert_eq!(
        reader.mem.tasks[&edited.id].title, "Edited offline",
        "a queued local edit wins over the server copy"
    );
    assert!(reader.dirty_ids.contains(&edited.id));
    assert_eq!(reader.list_projects().unwrap().len(), 1);
}

// ── offline task cache ───────────────────────────────────────────────────────

#[test]
fn task_created_offline_is_queued_and_survives_restart() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    let task = Task::new("Written on the train");
    backend.create_task(&task).unwrap();

    // The push fails (no server), so the flush must keep the task queued.
    backend.flush().unwrap();
    assert_eq!(backend.pending_sync_count(), 1);
    assert!(backend.dirty_ids.contains(&task.id));

    let on_disk = read_sidecar(&dir);
    assert_eq!(on_disk.pending_push, vec![task.id]);
    assert_eq!(on_disk.data.tasks[&task.id].title, "Written on the train");

    let mut restarted = backend_with_sidecar(&dir);
    assert!(restarted.load_sidecar().unwrap());
    assert_eq!(
        restarted.get_task(&task.id).unwrap().map(|t| t.title),
        Some("Written on the train".to_string())
    );
    assert_eq!(restarted.pending_sync_count(), 1, "still waiting to push");
    assert!(!restarted.mem_dirty);
}

#[test]
fn flush_retries_a_pending_push_even_without_new_edits() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    backend.create_task(&Task::new("Queued")).unwrap();
    backend.flush().unwrap();

    let mut restarted = backend_with_sidecar(&dir);
    restarted.load_sidecar().unwrap();
    assert!(!restarted.mem_dirty);
    // Nothing changed in memory, but the queue is not empty, so flush must
    // try the server again (and, offline, keep the task queued).
    restarted.flush().unwrap();
    assert_eq!(restarted.pending_sync_count(), 1);
}

#[test]
fn task_deleted_offline_is_queued_for_server_delete() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    let task = Task::new("Cached from server");
    backend.mem.tasks.insert(task.id, task.clone());
    backend.delete_task(&task.id).unwrap();
    backend.flush().unwrap();

    let on_disk = read_sidecar(&dir);
    assert!(on_disk.data.tasks.is_empty());
    assert_eq!(on_disk.pending_delete, vec![task.id]);

    let mut restarted = backend_with_sidecar(&dir);
    restarted.load_sidecar().unwrap();
    assert!(restarted.get_task(&task.id).unwrap().is_none());
    assert!(restarted.deleted_ids.contains(&task.id));
}

#[test]
fn creating_then_deleting_offline_leaves_nothing_to_push() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    let task = Task::new("Short-lived");
    backend.create_task(&task).unwrap();
    backend.delete_task(&task.id).unwrap();
    backend.flush().unwrap();

    assert!(!backend.dirty_ids.contains(&task.id));
    assert!(backend.deleted_ids.contains(&task.id));
    assert!(read_sidecar(&dir).pending_push.is_empty());
}

#[test]
fn merge_remote_tasks_respects_pending_queues() {
    let mut backend = CalDavBackend::from_config(test_config());

    // A: cached, server has a newer copy -> server wins.
    let mut a_local = Task::new("A old");
    a_local.updated_at = fixed_time(1);
    let mut a_remote = a_local.clone();
    a_remote.title = "A new".into();
    a_remote.updated_at = fixed_time(2);

    // B: edited offline (queued) -> local wins even though the server is newer.
    let mut b_local = Task::new("B local edit");
    b_local.updated_at = fixed_time(1);
    let mut b_remote = b_local.clone();
    b_remote.title = "B on server".into();
    b_remote.updated_at = fixed_time(3);

    // C: deleted offline (queued) -> must not come back.
    let c_remote = Task::new("C deleted locally");

    // D: created offline (queued), unknown to the server -> kept.
    let d_local = Task::new("D offline");

    // E: new on the server -> added.
    let e_remote = Task::new("E new on server");

    // F: cached but gone from the server -> dropped.
    let f_local = Task::new("F gone");

    for t in [&a_local, &b_local, &d_local, &f_local] {
        backend.mem.tasks.insert(t.id, t.clone());
    }
    backend.dirty_ids.extend([b_local.id, d_local.id]);
    backend.deleted_ids.insert(c_remote.id);

    let changed = backend.merge_remote_tasks(vec![
        a_remote.clone(),
        b_remote,
        c_remote.clone(),
        e_remote.clone(),
    ]);

    let titles: std::collections::BTreeSet<_> = backend
        .mem
        .tasks
        .values()
        .map(|t| t.title.clone())
        .collect();
    assert_eq!(
        titles,
        ["A new", "B local edit", "D offline", "E new on server"]
            .into_iter()
            .map(String::from)
            .collect()
    );
    assert_eq!(changed, 3, "A updated, E added, F removed");
    assert!(
        backend.deleted_ids.contains(&c_remote.id),
        "delete still owed"
    );
    assert!(backend.dirty_ids.contains(&b_local.id));
    assert!(backend.dirty_ids.contains(&d_local.id));
}

#[test]
fn merge_remote_tasks_forgets_deletes_the_server_already_applied() {
    let mut backend = CalDavBackend::from_config(test_config());
    let gone = TaskId::new();
    backend.deleted_ids.insert(gone);
    backend.merge_remote_tasks(vec![]);
    assert!(backend.deleted_ids.is_empty());
    assert_eq!(backend.pending_sync_count(), 0);
}

#[test]
fn sidecar_without_queues_still_loads() {
    let dir = TempDir::new().unwrap();
    let legacy = ExportData {
        projects: vec![Project::new("Old sidecar")],
        ..ExportData::default()
    };
    std::fs::write(
        dir.path().join("caldav.local.json"),
        serde_json::to_string(&legacy).unwrap(),
    )
    .unwrap();

    let mut backend = backend_with_sidecar(&dir);
    assert!(backend.load_sidecar().unwrap());
    assert_eq!(backend.list_projects().unwrap().len(), 1);
    assert_eq!(backend.pending_sync_count(), 0);
}

/// Offline start: the cached tasks are what the user sees. Feature-gated for
/// the same reason as `initialize_loads_sidecar_before_server_sync`.
#[cfg(not(feature = "caldav-sync"))]
#[test]
fn initialize_without_server_keeps_cached_and_queued_tasks() {
    let dir = TempDir::new().unwrap();
    let mut writer = backend_with_sidecar(&dir);
    let cached = Task::new("Cached");
    writer.mem.tasks.insert(cached.id, cached.clone());
    let offline = Task::new("Offline edit");
    writer.create_task(&offline).unwrap();
    writer.flush().unwrap();

    let mut backend = backend_with_sidecar(&dir);
    backend.initialize().unwrap();
    let mut titles: Vec<_> = backend
        .list_tasks()
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    titles.sort();
    assert_eq!(titles, ["Cached", "Offline edit"]);
    assert_eq!(backend.pending_sync_count(), 1);
}

#[cfg(not(feature = "caldav-sync"))]
#[test]
fn import_all_queues_every_task_when_offline() {
    let dir = TempDir::new().unwrap();
    let mut backend = backend_with_sidecar(&dir);
    let mut data = ExportData::default();
    let t1 = Task::new("One");
    let t2 = Task::new("Two");
    data.tasks.insert(t1.id, t1.clone());
    data.tasks.insert(t2.id, t2.clone());
    data.projects.push(Project::new("P"));

    backend.import_all(&data).unwrap();

    assert_eq!(backend.list_tasks().unwrap().len(), 2);
    assert_eq!(backend.pending_sync_count(), 2);
    assert!(!backend.mem_dirty);
    let on_disk = read_sidecar(&dir);
    assert_eq!(on_disk.pending_push.len(), 2);
    assert_eq!(on_disk.data.projects.len(), 1);
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
