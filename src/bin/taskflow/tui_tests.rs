//! Tests for `drain_editor_writes`: how editor socket writes reach the model,
//! the view lists and the disk.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, SyncSender};

use tempfile::tempdir;

use taskflow::app::Model;
use taskflow::socket::types::{EditorWriteRequest, PipeEnvelope, SocketResponse};
use taskflow::storage::BackendType;

use crate::tui::drain_editor_writes;

fn json_model(path: &Path) -> Model {
    Model::new()
        .with_storage(BackendType::Json, path.to_path_buf())
        .expect("JSON storage in a tempdir")
}

/// Queue one request on `tx` and return the receiver for its response.
fn push(
    tx: &SyncSender<EditorWriteRequest>,
    envelope: PipeEnvelope,
    focus_task_id: Option<String>,
) -> Receiver<SocketResponse> {
    let (respond_to, response_rx) = mpsc::sync_channel(1);
    tx.send(EditorWriteRequest {
        pipe_envelope: envelope,
        respond_to,
        focus_task_id,
    })
    .unwrap();
    response_rx
}

fn create_task_envelope(title: &str) -> PipeEnvelope {
    PipeEnvelope {
        operation: "create".to_string(),
        entity: "task".to_string(),
        id: None,
        data: Some(serde_json::json!({ "title": title })),
        filters: None,
    }
}

#[test]
fn drain_without_requests_applies_nothing() {
    let mut model = Model::new();
    let (_tx, rx) = mpsc::sync_channel::<EditorWriteRequest>(4);

    assert!(!drain_editor_writes(&mut model, &rx));
}

#[test]
fn drain_saves_editor_created_task_to_disk_immediately() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("data.json");
    let mut model = json_model(&path);
    let (tx, rx) = mpsc::sync_channel(4);
    let response_rx = push(&tx, create_task_envelope("from the editor"), None);

    assert!(drain_editor_writes(&mut model, &rx));

    let response = response_rx.try_recv().expect("a response was sent");
    assert!(response.success, "create failed: {:?}", response.error);
    let on_disk = std::fs::read_to_string(&path).expect("data file written by the drain");
    assert!(on_disk.contains("from the editor"));
    assert!(!model.storage.dirty);
    assert!(model.alerts.error_message.is_none());
}

#[test]
fn drain_refreshes_visible_tasks() {
    let dir = tempdir().unwrap();
    let mut model = json_model(&dir.path().join("data.json"));
    model.refresh_visible_tasks();
    assert!(model.visible_tasks.is_empty());
    let (tx, rx) = mpsc::sync_channel(4);
    let _response_rx = push(&tx, create_task_envelope("visible now"), None);

    drain_editor_writes(&mut model, &rx);

    assert_eq!(model.visible_tasks.len(), 1);
    let task = &model.tasks[&model.visible_tasks[0]];
    assert_eq!(task.title, "visible now");
}

#[test]
fn drain_applies_every_queued_request() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("data.json");
    let mut model = json_model(&path);
    let (tx, rx) = mpsc::sync_channel(4);
    let first = push(&tx, create_task_envelope("first"), None);
    let second = push(&tx, create_task_envelope("second"), None);

    drain_editor_writes(&mut model, &rx);

    assert!(first.try_recv().unwrap().success);
    assert!(second.try_recv().unwrap().success);
    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert!(on_disk.contains("first") && on_disk.contains("second"));
}

#[test]
fn drain_answers_focus_requests_with_null() {
    let mut model = Model::new();
    let (tx, rx) = mpsc::sync_channel(4);
    let envelope = PipeEnvelope {
        operation: "get".to_string(),
        entity: "task".to_string(),
        id: None,
        data: None,
        filters: None,
    };
    let response_rx = push(&tx, envelope, Some("no-such-task".to_string()));

    assert!(drain_editor_writes(&mut model, &rx));

    let response = response_rx.try_recv().unwrap();
    assert!(response.success);
    assert_eq!(response.data, Some(serde_json::Value::Null));
}

#[test]
fn drain_rejects_unknown_operation_as_invalid_request() {
    let mut model = Model::new();
    let (tx, rx) = mpsc::sync_channel(4);
    let envelope = PipeEnvelope {
        operation: "explode".to_string(),
        ..create_task_envelope("never created")
    };
    let response_rx = push(&tx, envelope, None);

    drain_editor_writes(&mut model, &rx);

    let response = response_rx.try_recv().unwrap();
    assert!(!response.success);
    assert_eq!(response.error.unwrap().code, "INVALID_REQUEST");
    assert!(model.tasks.is_empty());
}

#[test]
fn drain_does_not_save_sample_data() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("data.json");
    let mut model = json_model(&path).with_sample_data();
    assert!(model.storage.sample_data_mode);
    let (tx, rx) = mpsc::sync_channel(4);
    let _response_rx = push(&tx, create_task_envelope("sample only"), None);

    drain_editor_writes(&mut model, &rx);

    assert!(!path.exists());
}

#[test]
fn drain_reports_a_failed_save() {
    let dir = tempdir().unwrap();
    let blocked = dir.path().join("blocked");
    let mut model = json_model(&blocked.join("data.json"));
    // A plain file where the data directory should be makes the save fail.
    std::fs::write(&blocked, b"not a directory").unwrap();
    let (tx, rx) = mpsc::sync_channel(4);
    let response_rx = push(&tx, create_task_envelope("unsaved"), None);

    drain_editor_writes(&mut model, &rx);

    assert!(response_rx.try_recv().unwrap().success);
    let alert = model.alerts.error_message.expect("save error surfaced");
    assert!(
        alert.starts_with("Save after editor write failed"),
        "{alert}"
    );
    assert!(model.storage.dirty);
}
