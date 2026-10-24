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
//! let _server = start(&settings, write_tx, snap).expect("socket bound");
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
    ///
    /// The file is removed only while it is still *this* server's socket
    /// (same inode as at bind time). If another process has since replaced
    /// it, that process owns the path and the file is left alone.
    #[derive(Debug)]
    pub struct SocketGuard {
        path: PathBuf,
        inode: Option<u64>,
    }

    impl SocketGuard {
        /// Guard `path`, remembering the inode of the socket currently there.
        pub fn new(path: PathBuf) -> Self {
            let inode = socket_inode(&path);
            Self { path, inode }
        }
    }

    impl Drop for SocketGuard {
        fn drop(&mut self) {
            match (self.inode, socket_inode(&self.path)) {
                (Some(ours), Some(current)) if ours != current => {
                    debug!(path = %self.path.display(), "Socket file replaced by another process; leaving it");
                }
                (_, Some(_)) => {
                    let _ = std::fs::remove_file(&self.path);
                    debug!(path = %self.path.display(), "Removed socket file");
                }
                (_, None) => {}
            }
        }
    }

    fn socket_inode(path: &std::path::Path) -> Option<u64> {
        use std::os::unix::fs::MetadataExt;
        std::fs::symlink_metadata(path).ok().map(|m| m.ino())
    }

    /// Is a server already accepting connections on `path`?
    ///
    /// A leftover file from a crashed run refuses the connection; a live
    /// server accepts it. Anything that is not a socket is treated as stale.
    fn socket_is_live(path: &std::path::Path) -> bool {
        std::os::unix::net::UnixStream::connect(path).is_ok()
    }

    /// Handle returned from [`start`]; keeps the server running.
    #[derive(Debug)]
    pub struct EditorServer {
        #[allow(dead_code)]
        _guard: SocketGuard,
    }

    /// Longest socket path the kernel accepts (`sun_path` is 108 bytes
    /// including the terminating NUL).
    pub const MAX_SOCKET_PATH_BYTES: usize = 107;

    /// Start the editor socket server.
    ///
    /// The socket is bound before this returns, so a failure is reported to
    /// the caller instead of dying quietly on the accept thread. The error
    /// message names the path.
    ///
    /// # Errors
    ///
    /// Returns an error when the path is longer than
    /// [`MAX_SOCKET_PATH_BYTES`], its directory cannot be created, or the
    /// socket cannot be bound.
    pub fn start(
        settings: &Settings,
        write_tx: SyncSender<EditorWriteRequest>,
        snapshot_arc: Arc<Mutex<ModelSnapshot>>,
    ) -> std::io::Result<EditorServer> {
        let socket_path = settings.socket_path();
        let permissions = settings.editor.socket_permissions;
        let timeout = Duration::from_millis(settings.editor.query_timeout_ms);

        let path_len = socket_path.as_os_str().len();
        if path_len > MAX_SOCKET_PATH_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "socket path {} is {path_len} bytes; Unix sockets allow at most {MAX_SOCKET_PATH_BYTES} (set a shorter [editor] socket_path)",
                    socket_path.display()
                ),
            ));
        }

        // A live socket belongs to another running TaskFlow; unlinking it
        // would silently cut that instance off from its editors. Only a
        // leftover from a crashed run (connection refused) is removed.
        if socket_path.exists() {
            if socket_is_live(&socket_path) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    format!(
                        "another TaskFlow instance is already serving {}; quit it first or set a different [editor] socket_path",
                        socket_path.display()
                    ),
                ));
            }
            let _ = std::fs::remove_file(&socket_path);
            debug!(path = %socket_path.display(), "Removed stale socket file");
        }

        // Ensure parent directory exists
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!("cannot create socket directory {}: {e}", parent.display()),
                )
            })?;
        }

        let listener = UnixListener::bind(&socket_path).map_err(|e| {
            error!(error = %e, path = %socket_path.display(), "Failed to bind Unix socket");
            std::io::Error::new(
                e.kind(),
                format!("cannot bind {}: {e}", socket_path.display()),
            )
        })?;
        let guard = SocketGuard::new(socket_path.clone());

        if let Err(e) =
            std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(permissions))
        {
            warn!(error = %e, "Failed to set socket permissions");
        }

        info!(path = %socket_path.display(), "Editor socket listening");

        std::thread::Builder::new()
            .name("taskflow-socket".to_string())
            .spawn(move || {
                accept_loop(listener, snapshot_arc, write_tx, timeout);
            })?;

        Ok(EditorServer { _guard: guard })
    }

    fn accept_loop(
        listener: UnixListener,
        snapshot_arc: Arc<Mutex<ModelSnapshot>>,
        write_tx: SyncSender<EditorWriteRequest>,
        timeout: Duration,
    ) {
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
                let _g = SocketGuard::new(path.clone());
            }
            assert!(!path.exists());
        }

        #[test]
        fn test_socket_guard_no_panic_if_missing() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("nonexistent.sock");
            let _g = SocketGuard::new(path);
        }

        #[test]
        fn test_socket_guard_leaves_replaced_file() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("test.sock");
            std::fs::write(&path, b"").unwrap();
            let guard = SocketGuard::new(path.clone());
            // Another process takes over the path. Create the replacement
            // while the original still exists so it gets a distinct inode
            // (unlink-then-create may hand the same inode number back).
            let replacement = dir.path().join("replacement.sock");
            std::fs::write(&replacement, b"replacement").unwrap();
            std::fs::rename(&replacement, &path).unwrap();
            drop(guard);
            assert!(
                path.exists(),
                "guard must not remove a file it no longer owns"
            );
        }

        fn test_settings(path: &std::path::Path) -> Settings {
            let mut settings = Settings::default();
            settings.editor.socket_path = path.to_string_lossy().into_owned();
            settings
        }

        #[test]
        fn test_second_server_does_not_steal_live_socket() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("live.sock");
            let settings = test_settings(&path);
            let snap = Arc::new(Mutex::new(ModelSnapshot::default()));

            let (tx1, _rx1) = std::sync::mpsc::sync_channel(1);
            let first = start(&settings, tx1, Arc::clone(&snap)).expect("first server binds");
            assert!(socket_is_live(&path));

            let (tx2, _rx2) = std::sync::mpsc::sync_channel(1);
            let err = start(&settings, tx2, snap).expect_err("second server must fail");
            assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);

            // The first server is untouched and still reachable.
            assert!(socket_is_live(&path));
            drop(first);
            assert!(!path.exists(), "owner removes its socket on shutdown");
        }

        #[test]
        fn test_stale_socket_file_is_replaced() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("stale.sock");
            // Bind and drop the listener without removing the file: a crash.
            let listener = UnixListener::bind(&path).unwrap();
            drop(listener);
            assert!(path.exists());
            assert!(!socket_is_live(&path));

            let settings = test_settings(&path);
            let (tx, _rx) = std::sync::mpsc::sync_channel(1);
            let server = start(
                &settings,
                tx,
                Arc::new(Mutex::new(ModelSnapshot::default())),
            )
            .expect("stale file is reclaimed");
            assert!(socket_is_live(&path));
            drop(server);
        }
    }
}

#[cfg(unix)]
pub use server::{start, EditorServer, MAX_SOCKET_PATH_BYTES};
