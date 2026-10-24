//! Types for the editor/Neovim Unix Domain Socket interface.

use std::sync::mpsc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::domain::{Priority, TaskStatus};

// Re-import pipe types – these live in the binary crate, so we define our own
// lightweight versions here to avoid a circular dependency.
//
// The binary's pipe::handlers::dispatch() returns HandlerResult which is
// re-wrapped into PipeResponse by the TUI loop.

/// Serialisable error for the pipe/socket protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketPipeError {
    pub code: String,
    pub message: String,
}

/// Thin response envelope used by the write channel.
#[derive(Debug, Clone, Serialize)]
pub struct SocketResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SocketPipeError>,
}

impl SocketResponse {
    #[must_use]
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    #[must_use]
    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(SocketPipeError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

/// Requests that arrive over the Unix Domain Socket from editors.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditorRequest {
    /// Health check – returns pong + protocol version
    Ping,
    /// Current TUI state summary
    Status,
    /// Navigate TUI to a specific task
    FocusTask { task_id: String },
    /// Wrap an existing pipe-protocol request
    Pipe(PipeEnvelope),
}

/// A pipe request envelope mirroring the binary's PipeRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeEnvelope {
    pub operation: String,
    pub entity: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub filters: Option<serde_json::Value>,
}

/// A write operation sent from the socket thread to the TUI event loop.
///
/// If `focus_task_id` is `Some`, the TUI loop dispatches
/// `SystemMessage::FocusTaskById` instead of running the pipe handler.
pub struct EditorWriteRequest {
    pub pipe_envelope: PipeEnvelope,
    pub respond_to: mpsc::SyncSender<SocketResponse>,
    /// When set, navigate TUI to this task instead of running a pipe operation.
    pub focus_task_id: Option<String>,
}

/// Lightweight summary of a single task, used in snapshots.
#[derive(Debug, Clone, Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub priority: Priority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    /// Tag names, sorted; omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// A point-in-time snapshot of the Model, safe to read from the socket thread.
#[derive(Debug, Clone)]
pub struct ModelSnapshot {
    pub tasks: Vec<TaskSummary>,
    pub focus_task_id: Option<String>,
    pub incomplete_count: usize,
    pub protocol_version: &'static str,
    pub snapshot_at: SystemTime,
}

impl Default for ModelSnapshot {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            focus_task_id: None,
            incomplete_count: 0,
            protocol_version: "1",
            snapshot_at: SystemTime::now(),
        }
    }
}
