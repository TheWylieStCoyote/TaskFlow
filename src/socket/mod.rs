//! Unix Domain Socket server for the editor/Neovim integration.
//!
//! # Quick start
//!
//! ```no_run
//! use std::sync::{Arc, Mutex, mpsc};
//! use taskflow::config::Settings;
//! use taskflow::socket::{start, types::{EditorWriteRequest, ModelSnapshot}};
//! use taskflow::app::Model;
//!
//! let model = Model::new();
//! let snap  = Arc::new(Mutex::new(ModelSnapshot::from_model(&model)));
//! let (write_tx, write_rx) = mpsc::sync_channel::<EditorWriteRequest>(64);
//! let settings = Settings::default();
//! let _server = start(&settings, write_tx, snap);
//! // `_server` keeps the socket alive; drop it to remove the socket file.
//! ```

#[cfg(unix)]
pub mod handlers;
#[cfg(unix)]
pub mod snapshot;
pub mod types;

#[cfg(unix)]
mod server {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::mpsc::SyncSender;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use tracing::{debug, error, info, warn};

    use crate::config::Settings;

    use super::handlers;
    use super::types::{EditorRequest, EditorWriteRequest, ModelSnapshot};

    /// RAII guard that removes the socket file on `Drop`.
    pub struct SocketGuard(pub PathBuf);

    impl Drop for SocketGuard {
        fn drop(&mut self) {
            if self.0.exists() {
                let _ = std::fs::remove_file(&self.0);
                debug!(path = %self.0.display(), "Removed socket file");
            }
        }
    }

    /// Handle returned from [`start`]; keeps the server running.
    pub struct EditorServer {
        #[allow(dead_code)]
        _guard: SocketGuard,
    }

    /// Start the editor socket server.
    pub fn start(
        settings: &Settings,
        write_tx: SyncSender<EditorWriteRequest>,
        snapshot_arc: Arc<Mutex<ModelSnapshot>>,
    ) -> EditorServer {
        let socket_path = settings.socket_path();
        let permissions = settings.editor.socket_permissions;
        let timeout = Duration::from_millis(settings.editor.query_timeout_ms);

        // Remove stale socket file from a previous run
        let _ = std::fs::remove_file(&socket_path);

        // Ensure parent directory exists
        if let Some(parent) = socket_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!(error = %e, path = %parent.display(), "Failed to create socket directory");
            }
        }

        let guard = SocketGuard(socket_path.clone());

        std::thread::Builder::new()
            .name("taskflow-socket".to_string())
            .spawn(move || {
                accept_loop(socket_path, permissions, snapshot_arc, write_tx, timeout);
            })
            .expect("failed to spawn socket thread");

        EditorServer { _guard: guard }
    }

    fn accept_loop(
        socket_path: PathBuf,
        permissions: u32,
        snapshot_arc: Arc<Mutex<ModelSnapshot>>,
        write_tx: SyncSender<EditorWriteRequest>,
        timeout: Duration,
    ) {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                error!(error = %e, path = %socket_path.display(), "Failed to bind Unix socket");
                return;
            }
        };

        if let Err(e) =
            std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(permissions))
        {
            warn!(error = %e, "Failed to set socket permissions");
        }

        info!(path = %socket_path.display(), "Editor socket listening");

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let snap = Arc::clone(&snapshot_arc);
                    let tx = write_tx.clone();
                    std::thread::Builder::new()
                        .name("taskflow-socket-conn".to_string())
                        .spawn(move || handle_connection(stream, snap, tx, timeout))
                        .ok();
                }
                Err(e) => {
                    debug!(error = %e, "Socket accept error");
                    break;
                }
            }
        }
    }

    fn handle_connection(
        stream: std::os::unix::net::UnixStream,
        snapshot_arc: Arc<Mutex<ModelSnapshot>>,
        write_tx: SyncSender<EditorWriteRequest>,
        timeout: Duration,
    ) {
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();

        if let Err(e) = reader.read_line(&mut line) {
            warn!(error = %e, "Failed to read request from socket");
            return;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }

        let response = match serde_json::from_str::<EditorRequest>(trimmed) {
            Ok(req) => {
                debug!(?req, "Received editor request");
                handlers::route(req, &snapshot_arc, &write_tx, timeout)
            }
            Err(e) => serde_json::json!({
                "success": false,
                "error": { "code": "PARSE_ERROR", "message": format!("Invalid request JSON: {e}") }
            }),
        };

        let mut response_line =
            serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false}"#.to_string());
        response_line.push('\n');

        if let Err(e) = (&stream).write_all(response_line.as_bytes()) {
            warn!(error = %e, "Failed to write response to socket");
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tempfile::tempdir;

        #[test]
        fn test_socket_guard_removes_file() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("test.sock");
            std::fs::write(&path, b"").unwrap();
            assert!(path.exists());
            {
                let _g = SocketGuard(path.clone());
            }
            assert!(!path.exists());
        }

        #[test]
        fn test_socket_guard_no_panic_if_missing() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("nonexistent.sock");
            let _g = SocketGuard(path);
        }
    }
}

#[cfg(unix)]
pub use server::{start, EditorServer};
