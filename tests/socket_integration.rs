//! End-to-end integration tests for the Unix Domain Socket editor interface.
//!
//! These tests spin up the socket server against a real (in-memory) Model and
//! verify the full request → response cycle.

#[cfg(unix)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    use tempfile::tempdir;

    use taskflow::app::Model;
    use taskflow::config::Settings;
    use taskflow::socket;
    use taskflow::socket::types::{EditorWriteRequest, ModelSnapshot, SocketResponse};

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn make_settings_with_socket(path: &std::path::Path) -> Settings {
        let mut settings = Settings::default();
        settings.editor.enabled = true;
        settings.editor.socket_path = path.to_string_lossy().to_string();
        settings.editor.socket_permissions = 0o600;
        settings.editor.query_timeout_ms = 2000;
        settings
    }

    fn start_server(
        socket_path: &std::path::Path,
        model: &Model,
    ) -> (
        socket::EditorServer,
        mpsc::Receiver<EditorWriteRequest>,
        Arc<Mutex<ModelSnapshot>>,
    ) {
        let snap = Arc::new(Mutex::new(ModelSnapshot::from_model(model)));
        let (write_tx, write_rx) = mpsc::sync_channel(64);
        let settings = make_settings_with_socket(socket_path);
        let server = socket::start(&settings, write_tx, Arc::clone(&snap));
        (server, write_rx, snap)
    }

    /// Connect to the socket (with retries), send a JSON request, read one
    /// newline-terminated response.
    fn send_request(socket_path: &std::path::Path, request_json: &str) -> serde_json::Value {
        let stream = {
            let mut s = None;
            for _ in 0..30 {
                match UnixStream::connect(socket_path) {
                    Ok(stream) => {
                        s = Some(stream);
                        break;
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
            s.expect("socket did not become ready within 300ms")
        };

        let mut payload = request_json.to_string();
        payload.push('\n');
        (&stream).write_all(payload.as_bytes()).unwrap();

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        serde_json::from_str(line.trim()).expect("invalid JSON response")
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_ping_roundtrip() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("ping.sock");
        let model = Model::new();
        let (_server, _rx, _snap) = start_server(&sock, &model);

        let resp = send_request(&sock, r#"{"type":"ping"}"#);
        assert_eq!(resp["success"], true);
        assert_eq!(resp["data"]["pong"], true);
        assert_eq!(resp["data"]["protocol_version"], "1");
    }

    #[test]
    fn test_status_roundtrip() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("status.sock");
        let model = Model::new();
        let (_server, _rx, _snap) = start_server(&sock, &model);

        let resp = send_request(&sock, r#"{"type":"status"}"#);
        assert_eq!(resp["success"], true);
        assert_eq!(resp["data"]["running"], true);
    }

    #[test]
    fn test_list_tasks_from_snapshot() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("list.sock");

        let mut model = Model::new();
        let task = taskflow::domain::Task::new("Snapshot task");
        model.tasks.insert(task.id, task);

        let (_server, _rx, _snap) = start_server(&sock, &model);

        let resp = send_request(
            &sock,
            r#"{"type":"pipe","operation":"list","entity":"task"}"#,
        );
        assert_eq!(resp["success"], true);
        let data = resp["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["title"], "Snapshot task");
    }

    #[test]
    fn test_create_task_via_socket() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("create.sock");
        let model = Model::new();
        let (_server, write_rx, _snap) = start_server(&sock, &model);

        // Send create in a background thread
        let sock2 = sock.clone();
        let handle = std::thread::spawn(move || {
            send_request(
                &sock2,
                r#"{"type":"pipe","operation":"create","entity":"task","data":{"title":"Socket task"}}"#,
            )
        });

        // Simulate TUI loop draining the write channel
        let req = write_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let _ = req.respond_to.send(SocketResponse::ok(
            serde_json::json!({"id": "abc123", "title": "Socket task"}),
        ));

        let resp = handle.join().unwrap();
        assert_eq!(resp["success"], true);
    }

    #[test]
    fn test_focus_task_not_found() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("focus.sock");
        let model = Model::new();
        let (_server, _rx, _snap) = start_server(&sock, &model);

        let resp = send_request(
            &sock,
            r#"{"type":"focus_task","task_id":"00000000-0000-0000-0000-000000000000"}"#,
        );
        assert_eq!(resp["success"], false);
        assert_eq!(resp["error"]["code"], "NOT_FOUND");
    }

    #[test]
    fn test_concurrent_reads() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("concurrent.sock");

        let mut model = Model::new();
        for i in 0..3usize {
            let task = taskflow::domain::Task::new(format!("Task {i}"));
            model.tasks.insert(task.id, task);
        }

        let (_server, _rx, _snap) = start_server(&sock, &model);

        // Four concurrent readers — all should succeed without deadlock
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let sp = sock.clone();
                std::thread::spawn(move || {
                    send_request(&sp, r#"{"type":"pipe","operation":"list","entity":"task"}"#)
                })
            })
            .collect();

        for h in handles {
            let resp = h.join().unwrap();
            assert_eq!(resp["success"], true, "concurrent read should succeed");
        }
    }

    #[test]
    fn test_socket_cleanup_on_drop() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("cleanup.sock");
        let model = Model::new();

        {
            let (_server, _rx, _snap) = start_server(&sock, &model);
            // Wait for socket file to appear
            for _ in 0..30 {
                if sock.exists() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(
                sock.exists(),
                "socket file should exist while server is live"
            );
        } // EditorServer dropped → SocketGuard::drop removes the file

        std::thread::sleep(Duration::from_millis(20));
        assert!(
            !sock.exists(),
            "socket file should be removed after server drop"
        );
    }

    #[test]
    fn test_malformed_request() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("malformed.sock");
        let model = Model::new();
        let (_server, _rx, _snap) = start_server(&sock, &model);

        let resp = send_request(&sock, r#"{not valid json"#);
        assert_eq!(resp["success"], false);
        assert_eq!(resp["error"]["code"], "PARSE_ERROR");
    }

    // ---------------------------------------------------------------------------
    // Unit tests (no socket needed)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_model_snapshot_from_model() {
        let mut model = Model::new();
        let task = taskflow::domain::Task::new("Test task");
        let tid = task.id;
        model.tasks.insert(tid, task);
        model.refresh_visible_tasks();

        let snap = ModelSnapshot::from_model(&model);
        assert_eq!(snap.tasks.len(), 1);
        assert_eq!(snap.tasks[0].title, "Test task");
        assert_eq!(snap.incomplete_count, 1);
    }

    #[test]
    fn test_editor_request_deser_ping() {
        use taskflow::socket::types::EditorRequest;
        let req: EditorRequest = serde_json::from_str(r#"{"type":"ping"}"#).unwrap();
        assert!(matches!(req, EditorRequest::Ping));
    }

    #[test]
    fn test_editor_request_deser_pipe_create() {
        use taskflow::socket::types::EditorRequest;
        let json = r#"{"type":"pipe","operation":"create","entity":"task","data":{"title":"hi"}}"#;
        let req: EditorRequest = serde_json::from_str(json).unwrap();
        match req {
            EditorRequest::Pipe(env) => {
                assert_eq!(env.operation, "create");
                assert_eq!(env.entity, "task");
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn test_settings_editor_defaults() {
        let settings = Settings::default();
        assert!(settings.editor.enabled);
        assert_eq!(settings.editor.socket_path, "");
        assert_eq!(settings.editor.socket_permissions, 0o600);
        assert_eq!(settings.editor.query_timeout_ms, 5000);
        assert!(!settings.editor.auto_open_tui);
    }

    #[test]
    fn test_settings_socket_path_resolution() {
        let settings = Settings::default();
        let path = settings.socket_path();
        assert!(
            path.to_string_lossy().contains("taskflow.sock"),
            "socket path should contain 'taskflow.sock', got: {}",
            path.display()
        );
    }
}
