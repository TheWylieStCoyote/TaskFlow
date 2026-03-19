//! Route `EditorRequest` variants to their responses.

use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use super::types::{EditorRequest, EditorWriteRequest, ModelSnapshot, PipeEnvelope};

/// Route one `EditorRequest` and return a serialisable JSON value.
pub fn route(
    request: EditorRequest,
    snapshot_arc: &Arc<Mutex<ModelSnapshot>>,
    write_tx: &mpsc::SyncSender<EditorWriteRequest>,
    timeout: Duration,
) -> serde_json::Value {
    match request {
        EditorRequest::Ping => handle_ping(snapshot_arc),
        EditorRequest::Status => handle_status(snapshot_arc),
        EditorRequest::FocusTask { task_id } => {
            handle_focus_task(task_id, snapshot_arc, write_tx, timeout)
        }
        EditorRequest::Pipe(envelope) => handle_pipe(envelope, snapshot_arc, write_tx, timeout),
    }
}

fn snapshot_age_ms(snap: &ModelSnapshot) -> u64 {
    snap.snapshot_at.elapsed().unwrap_or_default().as_millis() as u64
}

fn handle_ping(snapshot_arc: &Arc<Mutex<ModelSnapshot>>) -> serde_json::Value {
    let age_ms = snapshot_arc
        .lock()
        .map(|s| snapshot_age_ms(&s))
        .unwrap_or(0);
    json!({
        "success": true,
        "data": {
            "pong": true,
            "protocol_version": "1",
            "server_pid": std::process::id(),
            "snapshot_age_ms": age_ms
        }
    })
}

fn handle_status(snapshot_arc: &Arc<Mutex<ModelSnapshot>>) -> serde_json::Value {
    let Ok(snap) = snapshot_arc.lock() else {
        return error_resp("INTERNAL", "Failed to acquire snapshot lock");
    };

    let focus_task = snap
        .focus_task_id
        .as_deref()
        .and_then(|fid| snap.tasks.iter().find(|t| t.id == fid))
        .map(
            |t| json!({ "id": t.id, "title": t.title, "priority": t.priority, "status": t.status }),
        );

    let snapshot_at = snap
        .snapshot_at
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    json!({
        "success": true,
        "data": {
            "running": true,
            "focus_task": focus_task,
            "incomplete_count": snap.incomplete_count,
            "snapshot_at": snapshot_at
        }
    })
}

fn handle_focus_task(
    task_id: String,
    snapshot_arc: &Arc<Mutex<ModelSnapshot>>,
    write_tx: &mpsc::SyncSender<EditorWriteRequest>,
    timeout: Duration,
) -> serde_json::Value {
    // Validate the task exists in the snapshot
    {
        let Ok(snap) = snapshot_arc.lock() else {
            return error_resp("INTERNAL", "Failed to acquire snapshot lock");
        };
        if !snap.tasks.iter().any(|t| t.id == task_id) {
            return json!({
                "success": false,
                "error": { "code": "NOT_FOUND", "message": format!("Task '{}' not found", task_id) }
            });
        }
    }

    let (resp_tx, resp_rx) = mpsc::sync_channel(1);
    let envelope = PipeEnvelope {
        operation: "get".to_string(),
        entity: "task".to_string(),
        id: Some(task_id.clone()),
        data: None,
        filters: None,
    };
    let write_req = EditorWriteRequest {
        pipe_envelope: envelope,
        respond_to: resp_tx,
        focus_task_id: Some(task_id.clone()),
    };

    if write_tx.send(write_req).is_err() {
        return error_resp("CHANNEL_CLOSED", "TUI event loop is not running");
    }
    match resp_rx.recv_timeout(timeout) {
        Ok(_) => json!({ "success": true, "data": { "focused": true, "task_id": task_id } }),
        Err(_) => error_resp("TIMEOUT", "TUI did not respond in time"),
    }
}

fn handle_pipe(
    envelope: PipeEnvelope,
    snapshot_arc: &Arc<Mutex<ModelSnapshot>>,
    write_tx: &mpsc::SyncSender<EditorWriteRequest>,
    timeout: Duration,
) -> serde_json::Value {
    match envelope.operation.as_str() {
        "list" | "export" => serve_list_from_snapshot(snapshot_arc),
        "get" => serve_get_from_snapshot(&envelope, snapshot_arc),
        _ => forward_to_tui(envelope, write_tx, timeout),
    }
}

fn serve_list_from_snapshot(snapshot_arc: &Arc<Mutex<ModelSnapshot>>) -> serde_json::Value {
    let Ok(snap) = snapshot_arc.lock() else {
        return error_resp("INTERNAL", "Failed to acquire snapshot lock");
    };
    let tasks: Vec<_> = snap.tasks.iter().collect();
    json!({ "success": true, "data": tasks, "metadata": { "total": tasks.len() } })
}

fn serve_get_from_snapshot(
    envelope: &PipeEnvelope,
    snapshot_arc: &Arc<Mutex<ModelSnapshot>>,
) -> serde_json::Value {
    let Ok(snap) = snapshot_arc.lock() else {
        return error_resp("INTERNAL", "Failed to acquire snapshot lock");
    };
    match &envelope.id {
        None => error_resp("INVALID_REQUEST", "id is required for get"),
        Some(id) => match snap.tasks.iter().find(|t| &t.id == id) {
            Some(t) => json!({ "success": true, "data": t }),
            None => json!({
                "success": false,
                "error": { "code": "NOT_FOUND", "message": format!("Task '{}' not found", id) }
            }),
        },
    }
}

fn forward_to_tui(
    envelope: PipeEnvelope,
    write_tx: &mpsc::SyncSender<EditorWriteRequest>,
    timeout: Duration,
) -> serde_json::Value {
    let (resp_tx, resp_rx) = mpsc::sync_channel(1);
    let write_req = EditorWriteRequest {
        pipe_envelope: envelope,
        respond_to: resp_tx,
        focus_task_id: None,
    };
    if write_tx.send(write_req).is_err() {
        return error_resp("CHANNEL_CLOSED", "TUI event loop is not running");
    }
    match resp_rx.recv_timeout(timeout) {
        Ok(response) => serde_json::to_value(&response)
            .unwrap_or_else(|e| error_resp("SERIALIZATION_ERROR", &e.to_string())),
        Err(_) => error_resp("TIMEOUT", "TUI did not respond in time"),
    }
}

fn error_resp(code: &str, message: &str) -> serde_json::Value {
    json!({ "success": false, "error": { "code": code, "message": message } })
}
