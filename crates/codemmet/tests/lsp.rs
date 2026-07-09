//! End-to-end tests for `codemmet lsp`: the real binary as a child process,
//! speaking LSP over stdin/stdout with Content-Length framing.

// Expected texts like `${1:i}` are LSP snippet syntax, not format strings.
#![allow(clippy::literal_string_with_formatting_args)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

mod common;
use common::scratch_dir;

/// A running `codemmet lsp` child plus a framed JSON-RPC client for it.
struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl LspClient {
    /// Spawn the server and run the initialize handshake.
    ///
    /// `encodings` fills `general.positionEncodings`; `extra_options` merges
    /// into `initializationOptions` next to the scratch `snippets_dir`.
    fn start(test: &str, encodings: &[&str], extra_options: &Value) -> (Self, Value) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_codemmet"))
            .args(["lsp", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("server should spawn");
        let stdin = child.stdin.take().expect("child stdin should be piped");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout should be piped"));
        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        };

        let scratch = scratch_dir(test);
        let scratch_str = scratch.to_str().expect("scratch path is UTF-8");
        let mut options = json!({ "snippets_dir": scratch_str });
        if let (Some(base), Some(extra)) = (options.as_object_mut(), extra_options.as_object()) {
            base.extend(extra.clone());
        }
        let initialize_result = client.request(
            "initialize",
            &json!({
                "capabilities": { "general": { "positionEncodings": encodings } },
                // Report the scratch dir as the workspace root; only tests that
                // set `project_snippets` act on it.
                "rootUri": format!("file://{scratch_str}"),
                "initializationOptions": options,
            }),
        );
        client.notify("initialized", &json!({}));
        (client, initialize_result)
    }

    /// Write one framed message.
    fn send(&mut self, message: &Value) {
        let body = message.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len())
            .and_then(|()| self.stdin.flush())
            .expect("writing to the server should succeed");
    }

    /// Read one framed message.
    fn read_message(&mut self) -> Value {
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            self.stdout
                .read_line(&mut line)
                .expect("reading a header line should succeed");
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length: ") {
                content_length = Some(value.parse().expect("Content-Length should be a number"));
            }
        }
        let length = content_length.expect("response should carry Content-Length");
        let mut body = vec![0u8; length];
        self.stdout
            .read_exact(&mut body)
            .expect("reading the message body should succeed");
        serde_json::from_slice(&body).expect("message body should be JSON")
    }

    /// Send a request and return its full response object.
    fn request_raw(&mut self, method: &str, params: &Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let message = self.read_message();
            if message.get("id").and_then(Value::as_i64) == Some(id) {
                return message;
            }
        }
    }

    /// Send a request that must succeed; returns its `result`.
    fn request(&mut self, method: &str, params: &Value) -> Value {
        let mut response = self.request_raw(method, params);
        assert!(
            response.get("error").is_none(),
            "unexpected error response: {response}"
        );
        response
            .as_object_mut()
            .and_then(|obj| obj.remove("result"))
            .expect("success response should carry a result")
    }

    /// Send a request that must fail; returns its `error` object.
    fn request_err(&mut self, method: &str, params: &Value) -> Value {
        let mut response = self.request_raw(method, params);
        response
            .as_object_mut()
            .and_then(|obj| obj.remove("error"))
            .expect("response should carry an error")
    }

    /// Send a notification.
    fn notify(&mut self, method: &str, params: &Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// `textDocument/didOpen` with full text.
    fn open(&mut self, uri: &str, language_id: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            &json!({ "textDocument": {
                "uri": uri, "languageId": language_id, "version": 1, "text": text,
            }}),
        );
    }

    /// `textDocument/completion` at (line, character); returns the result.
    fn complete(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.request(
            "textDocument/completion",
            &json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )
    }

    /// `textDocument/codeAction` for a zero-width range at (line, character).
    fn code_action(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.request(
            "textDocument/codeAction",
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": line, "character": character },
                    "end": { "line": line, "character": character },
                },
                "context": { "diagnostics": [] },
            }),
        )
    }

    /// Shut the server down and assert it exits cleanly.
    fn shutdown(mut self) {
        self.request("shutdown", &Value::Null);
        self.notify("exit", &Value::Null);
        let status = self.child.wait().expect("server should be waitable");
        assert_eq!(status.code(), Some(0), "server should exit cleanly");
    }
}

/// Items of a completion result that came back as an array.
fn items(result: &Value) -> &Vec<Value> {
    result
        .as_array()
        .expect("completion result should be an array")
}

/// The completion items with `kind == Snippet` (15).
fn snippet_items(result: &Value) -> Vec<&Value> {
    items(result)
        .iter()
        .filter(|item| item["kind"] == json!(15))
        .collect()
}

const URI: &str = "file:///test/main.rs";
const DOC: &str = "fn main() {\n    fori10\n}\n";

#[test]
fn initialize_advertises_capabilities_and_exits_cleanly() {
    let (client, result) = LspClient::start("init", &["utf-8", "utf-16"], &json!({}));
    let capabilities = &result["capabilities"];
    assert_eq!(capabilities["positionEncoding"], json!("utf-8"));
    // 2 = TextDocumentSyncKind::Incremental.
    assert_eq!(capabilities["textDocumentSync"], json!(2));
    assert_eq!(
        capabilities["completionProvider"]["triggerCharacters"],
        json!([">", "+", ":", ","])
    );
    assert_eq!(
        capabilities["completionProvider"]["resolveProvider"],
        json!(false)
    );
    assert_eq!(
        capabilities["executeCommandProvider"]["commands"],
        json!([
            "codemmet.expand",
            "codemmet.wrap",
            "codemmet.preview",
            "codemmet.list",
            "codemmet.context",
            "codemmet.reloadSnippets"
        ])
    );
    assert_eq!(capabilities["codeActionProvider"], json!(true));
    assert_eq!(result["serverInfo"]["name"], json!("codemmet"));
    client.shutdown();
}

#[test]
fn code_action_fills_match_arms_from_a_buffer_enum() {
    // Cursor inside an empty `match` over a variable → the server offers, per
    // in-scope enum, a plain-text edit inserting one arm per variant.
    let (mut client, _) = LspClient::start("code-action", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "enum Msg { Quit, Move, Write }\nfn f(m: Msg) {\n    match m {}\n}\n",
    );

    // Line 2 is `    match m {}`; the `{}` opens at column 12, so column 13 is
    // between the braces.
    let result = client.code_action(URI, 2, 13);
    let actions = result.as_array().expect("code actions are an array");
    assert_eq!(actions.len(), 1, "actions: {result}");
    assert_eq!(actions[0]["title"], json!("Fill match arms from Msg"));
    let edits = &actions[0]["edit"]["changes"][URI];
    assert_eq!(
        edits[0]["newText"],
        json!(
            "\n        Quit => todo!(),\n        Move => todo!(),\n        Write => todo!(),\n    "
        )
    );
    client.shutdown();
}

#[test]
fn completion_expands_fori10_with_exact_text_edit() {
    let (mut client, _) = LspClient::start("fori10", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);

    let result = client.complete(URI, 1, 10);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    let item = snippets[0];
    assert_eq!(item["label"], json!("fori10"));
    assert_eq!(item["insertTextFormat"], json!(2));
    assert_eq!(item["filterText"], json!("fori10"));
    assert_eq!(item["sortText"], json!("0fori10"));

    // The text edit replaces exactly `fori10` on line 1.
    let edit = &item["textEdit"];
    assert_eq!(
        edit["range"],
        json!({ "start": { "line": 1, "character": 4 }, "end": { "line": 1, "character": 10 } })
    );
    let new_text = edit["newText"]
        .as_str()
        .expect("newText should be a string");
    assert!(
        new_text.contains("for ${1:i} in 0..10"),
        "newText: {new_text}"
    );
    // Continuation lines carry only relative depth: LSP snippet clients
    // re-indent them from the insertion position, so baking the base indent
    // in server-side would double-indent.
    assert_eq!(new_text, "for ${1:i} in 0..10 {\n    $0\n}");

    // Docs are filled eagerly; the label detail is the leading trigger's
    // own human description.
    assert_eq!(
        item["labelDetails"]["description"],
        json!("for loop over integer range")
    );
    let docs = item["documentation"]["value"]
        .as_str()
        .expect("documentation should be markdown");
    assert!(docs.starts_with("```rust\n"), "docs: {docs}");
    client.shutdown();
}

#[test]
fn completion_offers_trigger_prefix_discovery() {
    let (mut client, _) = LspClient::start("prefix", &["utf-8"], &json!({}));
    client.open(URI, "rust", "fo");

    let result = client.complete(URI, 0, 2);
    let labels: Vec<&str> = items(&result)
        .iter()
        .filter(|item| item["kind"] == json!(14))
        .map(|item| item["label"].as_str().expect("label should be a string"))
        .collect();
    assert!(labels.contains(&"fori"), "labels: {labels:?}");
    assert!(labels.contains(&"fore"), "labels: {labels:?}");
    client.shutdown();
}

#[test]
fn completion_uses_utf16_positions_by_default() {
    let (mut client, init) = LspClient::start("utf16", &[], &json!({}));
    assert_eq!(init["capabilities"]["positionEncoding"], json!("utf-16"));

    // `日本` is 2 UTF-16 units but 6 UTF-8 bytes wide.
    client.open(
        URI,
        "rust",
        "fn main() {\n    let s = \"日本\"; fori10\n}\n",
    );
    let result = client.complete(URI, 1, 24);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    assert_eq!(
        snippets[0]["textEdit"]["range"],
        json!({ "start": { "line": 1, "character": 18 }, "end": { "line": 1, "character": 24 } })
    );
    client.shutdown();
}

#[test]
fn execute_command_expand_returns_range_and_snippet() {
    let (mut client, _) = LspClient::start("expand", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.expand",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 10 } }],
        }),
    );
    assert_eq!(
        result["range"],
        json!({ "start": { "line": 1, "character": 4 }, "end": { "line": 1, "character": 10 } })
    );
    assert_eq!(result["isSnippet"], json!(true));
    // Continuation lines carry only relative depth: LSP snippet clients
    // re-indent them from the insertion position, so baking the base indent
    // in server-side would double-indent.
    assert_eq!(result["snippet"], json!("for ${1:i} in 0..10 {\n    $0\n}"));
    client.shutdown();
}

#[test]
fn execute_command_expand_fills_arms_from_a_buffer_enum() {
    // The flagship data-driven case, end to end through the tree-sitter
    // provider: `match:Msg>arm*` reads the variants of the `Msg` enum defined
    // in the same buffer and renders one arm per variant — no `--repeat`.
    let (mut client, _) = LspClient::start("expand-enum", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "enum Msg { Quit, Move, Write }\nfn main() {\n    match:Msg>arm*\n}\n",
    );

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.expand",
            "arguments": [{ "uri": URI, "position": { "line": 2, "character": 18 } }],
        }),
    );
    assert_eq!(
        result["snippet"],
        json!("match Msg {\n    Quit => $1,\n    Move => $2,\n    Write => $0,\n}")
    );
    client.shutdown();
}

#[test]
fn execute_command_expand_arms_bind_payload_variants() {
    // Payload variants become wildcard bindings so the generated match
    // compiles: a tuple variant → `Write(..)`, a struct variant → `Move { .. }`.
    let (mut client, _) = LspClient::start("expand-enum-payload", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "enum Msg { Quit, Write(String), Move { x: i32 } }\nfn main() {\n    match:Msg>arm*\n}\n",
    );

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.expand",
            "arguments": [{ "uri": URI, "position": { "line": 2, "character": 18 } }],
        }),
    );
    assert_eq!(
        result["snippet"],
        json!("match Msg {\n    Quit => $1,\n    Write(..) => $2,\n    Move { .. } => $0,\n}")
    );
    client.shutdown();
}

#[test]
fn execute_command_expand_fills_fields_from_a_buffer_struct() {
    // CX3 end to end: `field*@Point` reads the fields of the `Point` struct in
    // the buffer, rendering each field's name and its declared type via
    // `${value}` / `${value.type}` — with no `--repeat`.
    let (mut client, _) = LspClient::start("expand-struct", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "struct Point { x: i32, y: f64 }\nfn main() {\n    struct:Wrapper>field*@Point\n}\n",
    );

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.expand",
            "arguments": [{ "uri": URI, "position": { "line": 2, "character": 31 } }],
        }),
    );
    assert_eq!(
        result["snippet"],
        json!("struct Wrapper {\n    x: i32,\n    y: f64,\n}$0")
    );
    client.shutdown();
}

#[test]
fn execute_command_expand_without_candidate_is_an_error() {
    let (mut client, _) = LspClient::start("expand-err", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);

    let error = client.request_err(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.expand",
            "arguments": [{ "uri": URI, "position": { "line": 2, "character": 0 } }],
        }),
    );
    let message = error["message"]
        .as_str()
        .expect("error should have a message");
    assert!(message.contains("no abbreviation"), "message: {message}");

    // A render failure surfaces its Display message too.
    client.open(URI, "rust", "fn main() {\n    forj\n}\n");
    let error = client.request_err(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.expand",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 8 } }],
        }),
    );
    let message = error["message"]
        .as_str()
        .expect("error should have a message");
    assert!(message.contains("unknown trigger"), "message: {message}");
    assert!(
        message.contains("did you mean `fore`?"),
        "message: {message}"
    );
    client.shutdown();
}

#[test]
fn execute_command_wrap_splices_selection_into_the_loop() {
    let (mut client, _) = LspClient::start("wrap", &["utf-8"], &json!({}));
    client.open(URI, "rust", "fn main() {\n    println!(\"x\");\n}\n");

    let range =
        json!({ "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 18 } });
    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.wrap",
            "arguments": [{ "uri": URI, "range": range, "abbreviation": "fori10" }],
        }),
    );
    // The returned range starts after the line's indent, so the leading
    // whitespace survives in the buffer and the snippet needs no base prefix.
    assert_eq!(
        result["range"],
        json!({ "start": { "line": 1, "character": 4 }, "end": { "line": 1, "character": 18 } })
    );
    assert_eq!(result["isSnippet"], json!(true));
    // The println line sits one level deeper inside the loop, `$0` after it.
    assert_eq!(
        result["snippet"],
        json!("for ${1:i} in 0..10 {\n    println!(\"x\");$0\n}")
    );
    client.shutdown();
}

/// Wrapping a deeper-indented selection starts the returned range at the
/// indent width (the leading whitespace is not part of the replacement)
/// and the snippet carries no base prefix on any line.
#[test]
fn execute_command_wrap_indented_selection_starts_after_the_indent() {
    let (mut client, _) = LspClient::start("wrap-indented", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "fn main() {\n    if true {\n        println!(\"y\");\n    }\n}\n",
    );

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.wrap",
            "arguments": [{
                "uri": URI,
                "range": { "start": { "line": 2, "character": 0 }, "end": { "line": 2, "character": 22 } },
                "abbreviation": "fori10",
            }],
        }),
    );
    // The wrapped line is indented 8 columns; the range starts there.
    assert_eq!(
        result["range"],
        json!({ "start": { "line": 2, "character": 8 }, "end": { "line": 2, "character": 22 } })
    );
    let snippet = result["snippet"]
        .as_str()
        .expect("snippet should be a string");
    assert_eq!(snippet, "for ${1:i} in 0..10 {\n    println!(\"y\");$0\n}");
    assert!(
        snippet.lines().all(|line| !line.starts_with("        ")),
        "no line should carry the old 8-column base prefix: {snippet}"
    );
    client.shutdown();
}

/// A mid-statement wrap range snaps outward to the whole statement's
/// lines; the returned range covers the snapped region.
#[cfg(feature = "tree-sitter")]
#[test]
fn execute_command_wrap_snaps_mid_statement_range() {
    let (mut client, _) = LspClient::start("wrap-snap", &["utf-8"], &json!({}));
    client.open(URI, "rust", "fn main() {\n    println!(\"x\");\n}\n");

    // Selection covers only `intln!` in the middle of the statement.
    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.wrap",
            "arguments": [{
                "uri": URI,
                "range": { "start": { "line": 1, "character": 6 }, "end": { "line": 1, "character": 12 } },
                "abbreviation": "fori10",
            }],
        }),
    );
    assert_eq!(
        result["range"],
        json!({ "start": { "line": 1, "character": 4 }, "end": { "line": 1, "character": 18 } })
    );
    assert_eq!(
        result["snippet"],
        json!("for ${1:i} in 0..10 {\n    println!(\"x\");$0\n}")
    );
    client.shutdown();
}

/// A fragment of a multi-line statement snaps to all of the statement's
/// lines, and the wrapped snippet carries every line.
#[cfg(feature = "tree-sitter")]
#[test]
fn execute_command_wrap_snaps_multiline_statement() {
    let (mut client, _) = LspClient::start("wrap-multiline", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "fn main() {\n    foo(\n        1,\n        2,\n    );\n}\n",
    );

    // Selection covers only the `1` argument on line 2.
    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.wrap",
            "arguments": [{
                "uri": URI,
                "range": { "start": { "line": 2, "character": 8 }, "end": { "line": 2, "character": 9 } },
                "abbreviation": "fori10",
            }],
        }),
    );
    assert_eq!(
        result["range"],
        json!({ "start": { "line": 1, "character": 4 }, "end": { "line": 4, "character": 6 } })
    );
    assert_eq!(
        result["snippet"],
        json!("for ${1:i} in 0..10 {\n    foo(\n        1,\n        2,\n    );$0\n}")
    );
    client.shutdown();
}

/// A range from one function's body into the next function's signature is
/// rejected with a scope-boundary error.
#[cfg(feature = "tree-sitter")]
#[test]
fn execute_command_wrap_cross_function_range_is_an_error() {
    let (mut client, _) = LspClient::start("wrap-cross", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "fn a() {\n    let x = 1;\n}\nfn b() {\n    let y = 2;\n}\n",
    );

    let error = client.request_err(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.wrap",
            "arguments": [{
                "uri": URI,
                "range": { "start": { "line": 1, "character": 8 }, "end": { "line": 3, "character": 4 } },
                "abbreviation": "fori10",
            }],
        }),
    );
    let message = error["message"]
        .as_str()
        .expect("error should have a message");
    assert!(
        message.contains("crosses scope boundaries"),
        "message: {message}"
    );
    client.shutdown();
}

/// The `newText` of the only snippet item in a completion result.
fn snippet_new_text(result: &Value) -> &str {
    let snippets = snippet_items(result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    snippets[0]["textEdit"]["newText"]
        .as_str()
        .expect("newText should be a string")
}

/// `fn` inside an `impl` body selects the `type-body` variant with `&self`.
#[cfg(feature = "tree-sitter")]
#[test]
fn completion_fn_inside_impl_expands_with_self() {
    let (mut client, _) = LspClient::start("fn-impl", &["utf-8"], &json!({}));
    client.open(URI, "rust", "impl Point {\n    fn\n}\n");

    let result = client.complete(URI, 1, 6);
    let new_text = snippet_new_text(&result);
    assert!(new_text.contains("&self"), "newText: {new_text}");

    // Prefix discovery items still appear alongside the snippet item.
    let keywords: Vec<&str> = items(&result)
        .iter()
        .filter(|item| item["kind"] == json!(14))
        .map(|item| item["label"].as_str().expect("label should be a string"))
        .collect();
    assert!(keywords.contains(&"fn"), "keywords: {keywords:?}");
    client.shutdown();
}

/// The same `fn` at the top level stays on the fallback body (no `&self`).
#[test]
fn completion_fn_at_top_level_has_no_self() {
    let (mut client, _) = LspClient::start("fn-top", &["utf-8"], &json!({}));
    client.open(URI, "rust", "fn\n");

    let result = client.complete(URI, 0, 2);
    let new_text = snippet_new_text(&result);
    assert!(!new_text.contains("&self"), "newText: {new_text}");
    client.shutdown();
}

/// Literal apostrophes earlier on the line (a Rust lifetime) do not open a
/// quoted region, so the candidate after them still extracts and completes.
#[test]
fn completion_works_after_a_lifetime_annotation() {
    let (mut client, _) = LspClient::start("lifetime", &["utf-8"], &json!({}));
    client.open(URI, "rust", "fn main() {\n    let x: &'a str = y; pr\n}\n");

    // Cursor right after `pr` at the end of line 1.
    let result = client.complete(URI, 1, 26);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    assert_eq!(snippets[0]["label"], json!("pr"));
    client.shutdown();
}

/// A cursor inside a string literal completes to a null result.
#[test]
fn completion_inside_string_literal_is_null() {
    let (mut client, _) = LspClient::start("in-string", &["utf-8"], &json!({}));
    client.open(URI, "rust", "fn main() {\n    let s = \"fori10\";\n}\n");

    // Character 19 sits right after `fori10`, inside the quotes.
    assert_eq!(client.complete(URI, 1, 19), Value::Null);
    client.shutdown();
}

/// Python `def` inside a class body selects the `self` variant.
///
/// The class body carries statements around the cursor: tree-sitter-python's
/// error recovery folds a `def` typed directly after `class A:` into the
/// class *header* (no body yet), where the type-body scope correctly does
/// not apply.
#[cfg(feature = "tree-sitter")]
#[test]
fn completion_def_inside_class_expands_with_self() {
    let (mut client, _) = LspClient::start("def-class", &["utf-8"], &json!({}));
    let uri = "file:///test/main.py";
    client.open(uri, "python", "class A:\n    x = 1\n    def\n    y = 2\n");

    let result = client.complete(uri, 2, 7);
    let new_text = snippet_new_text(&result);
    assert!(new_text.contains("(self"), "newText: {new_text}");
    client.shutdown();
}

/// `codemmet.expand` goes through the same provider as completion.
#[cfg(feature = "tree-sitter")]
#[test]
fn execute_command_expand_inside_impl_uses_self_variant() {
    let (mut client, _) = LspClient::start("expand-impl", &["utf-8"], &json!({}));
    client.open(URI, "rust", "impl Point {\n    fn\n}\n");

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.expand",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 6 } }],
        }),
    );
    let snippet = result["snippet"]
        .as_str()
        .expect("snippet should be a string");
    assert!(snippet.contains("&self"), "snippet: {snippet}");
    client.shutdown();
}

#[test]
fn did_change_incremental_applies_a_range_edit() {
    let (mut client, _) = LspClient::start("incremental", &["utf-8"], &json!({}));
    client.open(URI, "rust", "fn main() {\n    for\n}\n");

    // Insert `i10` mid-line after `for`, making the line `    fori10`.
    client.notify(
        "textDocument/didChange",
        &json!({
            "textDocument": { "uri": URI, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 1, "character": 7 },
                    "end": { "line": 1, "character": 7 },
                },
                "text": "i10",
            }],
        }),
    );
    let result = client.complete(URI, 1, 10);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    assert_eq!(snippets[0]["label"], json!("fori10"));
    let new_text = snippet_new_text(&result);
    assert!(
        new_text.contains("for ${1:i} in 0..10"),
        "newText: {new_text}"
    );
    client.shutdown();
}

#[test]
fn did_change_applies_multi_change_batches_in_order() {
    let (mut client, _) = LspClient::start("multi-change", &["utf-8"], &json!({}));
    client.open(URI, "rust", "pr\n");

    // Ranges of later changes refer to the text after the earlier ones:
    // `pr` → `fori` → `fori10`.
    client.notify(
        "textDocument/didChange",
        &json!({
            "textDocument": { "uri": URI, "version": 2 },
            "contentChanges": [
                {
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 2 },
                    },
                    "text": "fori",
                },
                {
                    "range": {
                        "start": { "line": 0, "character": 4 },
                        "end": { "line": 0, "character": 4 },
                    },
                    "text": "10",
                },
            ],
        }),
    );
    let result = client.complete(URI, 0, 6);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    assert_eq!(snippets[0]["label"], json!("fori10"));
    client.shutdown();
}

#[test]
fn did_change_full_sync_replaces_the_document() {
    let (mut client, _) = LspClient::start("didchange", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);
    assert_eq!(snippet_items(&client.complete(URI, 1, 10)).len(), 1);

    client.notify(
        "textDocument/didChange",
        &json!({
            "textDocument": { "uri": URI, "version": 2 },
            "contentChanges": [{ "text": "letq\n" }],
        }),
    );
    let result = client.complete(URI, 0, 4);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    assert_eq!(snippets[0]["label"], json!("letq"));
    client.shutdown();
}

#[test]
fn operators_only_gates_bare_words_but_allows_composition() {
    let (mut client, _) = LspClient::start(
        "operators-only",
        &["utf-8"],
        &json!({ "completion": "operators-only" }),
    );
    client.open(URI, "rust", DOC);
    assert_eq!(client.complete(URI, 1, 10), Value::Null);

    client.notify(
        "textDocument/didChange",
        &json!({
            "textDocument": { "uri": URI, "version": 2 },
            "contentChanges": [{ "text": "fn:main>pr\n" }],
        }),
    );
    let result = client.complete(URI, 0, 10);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    assert_eq!(snippets[0]["label"], json!("fn:main>pr"));
    client.shutdown();
}

#[test]
fn unknown_language_and_unopened_document_complete_to_null() {
    let (mut client, _) = LspClient::start("unknown-lang", &["utf-8"], &json!({}));
    client.open(URI, "cobol", DOC);
    assert_eq!(client.complete(URI, 1, 10), Value::Null);
    assert_eq!(client.complete("file:///not/open.rs", 0, 0), Value::Null);
    client.shutdown();
}

/// The project pack is resolved per document: `<file>`'s `.codemmet/snippets`
/// tree (walking up from the file), not the workspace root.
#[test]
fn project_snippets_opt_in_loads_the_project_pack() {
    let root = scratch_dir("project-opt-in");
    let pack_dir = root.join(".codemmet").join("snippets");
    std::fs::create_dir_all(&pack_dir).expect("project snippets dir should be creatable");
    std::fs::write(
        pack_dir.join("rust.toml"),
        "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nbody = \"PROJECT!($0);\"\n",
    )
    .expect("project pack should be writable");
    let uri = format!(
        "file://{}/main.rs",
        root.to_str().expect("scratch is UTF-8")
    );

    let (mut client, _) = LspClient::start(
        "project-opt-in",
        &["utf-8"],
        &json!({ "project_snippets": true }),
    );
    client.open(&uri, "rust", "fn main() {\n    pr\n}\n");
    let result = client.complete(&uri, 1, 6);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    let new_text = snippets[0]["textEdit"]["newText"]
        .as_str()
        .expect("newText should be a string");
    assert!(
        new_text.contains("PROJECT!"),
        "project pack should override pr; newText: {new_text}"
    );
    client.shutdown();
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn project_snippets_are_off_by_default() {
    // The same pack is present, but without the opt-in it must be ignored.
    let root = scratch_dir("project-default-off");
    let pack_dir = root.join(".codemmet").join("snippets");
    std::fs::create_dir_all(&pack_dir).expect("project snippets dir should be creatable");
    std::fs::write(
        pack_dir.join("rust.toml"),
        "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nbody = \"PROJECT!($0);\"\n",
    )
    .expect("project pack should be writable");
    let uri = format!(
        "file://{}/main.rs",
        root.to_str().expect("scratch is UTF-8")
    );

    let (mut client, _) = LspClient::start("project-default-off", &["utf-8"], &json!({}));
    client.open(&uri, "rust", "fn main() {\n    pr\n}\n");
    let result = client.complete(&uri, 1, 6);
    let snippets = snippet_items(&result);
    assert_eq!(snippets.len(), 1, "items: {result}");
    let new_text = snippets[0]["textEdit"]["newText"]
        .as_str()
        .expect("newText should be a string");
    assert!(
        new_text.contains("println!"),
        "default should ignore the project pack; newText: {new_text}"
    );
    client.shutdown();
    std::fs::remove_dir_all(&root).ok();
}

/// `codemmet.reloadSnippets` re-reads packs from disk: a pack written *after*
/// the doc was first completed (and cached) is only picked up after a reload.
#[test]
fn reload_snippets_picks_up_a_pack_written_after_open() {
    let root = scratch_dir("reload-pickup");
    let pack_dir = root.join(".codemmet").join("snippets");
    std::fs::create_dir_all(&pack_dir).expect("project snippets dir should be creatable");
    let uri = format!(
        "file://{}/main.rs",
        root.to_str().expect("scratch is UTF-8")
    );

    let (mut client, _) = LspClient::start(
        "reload-pickup",
        &["utf-8"],
        &json!({ "project_snippets": true }),
    );
    client.open(&uri, "rust", "fn main() {\n    pr\n}\n");

    // No pack yet → builtin println, and the engine is now cached.
    let before = client.complete(&uri, 1, 6);
    let text = snippet_new_text(&before);
    assert!(text.contains("println!"), "expected builtin; got {text}");

    // Write the project pack; the cached engine still yields the builtin.
    std::fs::write(
        pack_dir.join("rust.toml"),
        "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nbody = \"PROJECT!($0);\"\n",
    )
    .expect("project pack should be writable");
    let cached_result = client.complete(&uri, 1, 6);
    let cached = snippet_new_text(&cached_result);
    assert!(
        cached.contains("println!"),
        "cache should still hold; got {cached}"
    );

    // Reload (read-only) → the next completion re-reads and sees the pack.
    client.request(
        "workspace/executeCommand",
        &json!({ "command": "codemmet.reloadSnippets" }),
    );
    let after_result = client.complete(&uri, 1, 6);
    let after = snippet_new_text(&after_result);
    assert!(
        after.contains("PROJECT!"),
        "reload should pick up the pack; got {after}"
    );

    client.shutdown();
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn execute_command_preview_returns_clean_expansion() {
    let (mut client, _) = LspClient::start("preview", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.preview",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 10 } }],
        }),
    );
    // The range covers the abbreviation, and `text` is clean strip-mode code
    // (no snippet markers) with the line's base indent kept for the preview.
    assert_eq!(
        result["range"],
        json!({ "start": { "line": 1, "character": 4 }, "end": { "line": 1, "character": 10 } })
    );
    let text = result["text"]
        .as_str()
        .expect("preview text should be a string");
    assert!(text.contains("for i in 0..10 {"), "text: {text}");
    assert!(
        !text.contains('$'),
        "preview text should have no tabstop markers: {text}"
    );
    client.shutdown();
}

#[test]
fn execute_command_preview_without_candidate_is_null() {
    let (mut client, _) = LspClient::start("preview-null", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);

    // Line 2 is the closing brace: no abbreviation before the cursor.
    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.preview",
            "arguments": [{ "uri": URI, "position": { "line": 2, "character": 0 } }],
        }),
    );
    assert_eq!(result, Value::Null, "expected null preview, got {result}");
    client.shutdown();
}

#[test]
fn execute_command_list_returns_every_trigger_with_body_and_hint() {
    let (mut client, _) = LspClient::start("list", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.list",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 4 } }],
        }),
    );
    assert_eq!(result["language"], json!("rust"));
    let snippets = result["snippets"]
        .as_array()
        .expect("snippets should be an array");
    assert!(snippets.len() >= 30, "got {} snippets", snippets.len());

    let fori = snippets
        .iter()
        .find(|s| s["trigger"] == json!("fori"))
        .expect("fori should be listed");
    assert_eq!(fori["description"], json!("for loop over integer range"));
    assert_eq!(fori["composite"], json!(false));
    // At the cursor no variant fires, so the default template applies.
    assert_eq!(fori["activeContext"], Value::Null);
    // The keep-mode body is directly insertable by a snippet client (no base
    // indent baked in, same as `codemmet.expand`); the bare trigger leaves
    // its params as defaulted tabstops.
    assert_eq!(fori["body"], json!("for ${1:i} in 0..${2:10} {\n    $0\n}"));
    // The hint is the strip-mode first line, elided when more follow.
    assert_eq!(fori["hint"], json!("for i in 0..10 { …"));

    // Composites are flagged so a client can annotate them.
    let dstruct = snippets
        .iter()
        .find(|s| s["trigger"] == json!("dstruct"))
        .expect("dstruct should be listed");
    assert_eq!(dstruct["composite"], json!(true));
    client.shutdown();
}

/// The per-document engine backs `codemmet.list`, so a project pack trigger
/// shows up (with its override), not just builtins.
#[test]
fn execute_command_list_reflects_the_project_pack() {
    let root = scratch_dir("list-project");
    let pack_dir = root.join(".codemmet").join("snippets");
    std::fs::create_dir_all(&pack_dir).expect("project snippets dir should be creatable");
    std::fs::write(
        pack_dir.join("rust.toml"),
        "[meta]\nlanguage = \"rust\"\n[snippets.pr]\ndescription = \"project print\"\nbody = \"PROJECT!($0);\"\n",
    )
    .expect("project pack should be writable");
    let uri = format!(
        "file://{}/main.rs",
        root.to_str().expect("scratch is UTF-8")
    );

    let (mut client, _) = LspClient::start(
        "list-project",
        &["utf-8"],
        &json!({ "project_snippets": true }),
    );
    client.open(&uri, "rust", "fn main() {\n}\n");
    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.list",
            "arguments": [{ "uri": uri, "position": { "line": 0, "character": 0 } }],
        }),
    );
    let snippets = result["snippets"]
        .as_array()
        .expect("snippets should be an array");
    let pr = snippets
        .iter()
        .find(|s| s["trigger"] == json!("pr"))
        .expect("pr should be listed");
    assert_eq!(pr["description"], json!("project print"));
    assert_eq!(pr["body"], json!("PROJECT!($0);"));
    client.shutdown();
    std::fs::remove_dir_all(&root).ok();
}

/// Inside an `impl` body the `fn` entry names the variant that would fire,
/// and its body is the variant's (`&self`).
#[cfg(feature = "tree-sitter")]
#[test]
fn execute_command_list_annotates_the_variant_firing_at_the_cursor() {
    let (mut client, _) = LspClient::start("list-variant", &["utf-8"], &json!({}));
    client.open(URI, "rust", "impl Point {\n    \n}\n");

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.list",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 4 } }],
        }),
    );
    let snippets = result["snippets"]
        .as_array()
        .expect("snippets should be an array");
    let fn_item = snippets
        .iter()
        .find(|s| s["trigger"] == json!("fn"))
        .expect("fn should be listed");
    assert_eq!(fn_item["activeContext"], json!("type-body"));
    let body = fn_item["body"].as_str().expect("body should be a string");
    assert!(body.contains("&self"), "body: {body}");
    // A trigger with no matching variant stays on its default template.
    let fori = snippets
        .iter()
        .find(|s| s["trigger"] == json!("fori"))
        .expect("fori should be listed");
    assert_eq!(fori["activeContext"], Value::Null);
    client.shutdown();
}

#[test]
fn execute_command_context_reports_the_scope_stack() {
    let (mut client, _) = LspClient::start("context", &["utf-8"], &json!({}));
    client.open(URI, "rust", DOC);

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.context",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 4 } }],
        }),
    );
    assert_eq!(result["language"], json!("rust"));
    assert_eq!(result["inStringOrComment"], json!(false));
    // Outermost first; the null provider (no tree-sitter) reports just root.
    let scopes = result["scopes"].as_array().expect("scopes is an array");
    assert_eq!(scopes.first(), Some(&json!("root")), "scopes: {scopes:?}");
    assert!(result["collections"].is_array());
    client.shutdown();
}

/// Inside `fn main` with a same-file enum, the context carries the fn-body
/// scope stack and the enum as a named collection.
#[cfg(feature = "tree-sitter")]
#[test]
fn execute_command_context_reports_collections_and_scopes() {
    let (mut client, _) = LspClient::start("context-enum", &["utf-8"], &json!({}));
    client.open(
        URI,
        "rust",
        "enum Msg { Quit, Move, Write }\nfn main() {\n    \n}\n",
    );

    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.context",
            "arguments": [{ "uri": URI, "position": { "line": 2, "character": 4 } }],
        }),
    );
    assert_eq!(result["scopes"], json!(["root", "fn-body"]));
    let collections = result["collections"]
        .as_array()
        .expect("collections is an array");
    let msg = collections
        .iter()
        .find(|c| c["name"] == json!("Msg"))
        .expect("Msg should be in scope");
    let values: Vec<&str> = msg["items"]
        .as_array()
        .expect("items is an array")
        .iter()
        .map(|item| item["value"].as_str().expect("value is a string"))
        .collect();
    assert_eq!(values, ["Quit", "Move", "Write"]);
    client.shutdown();
}

#[test]
fn execute_command_list_and_context_error_on_unopened_document() {
    let (mut client, _) = LspClient::start("list-err", &["utf-8"], &json!({}));
    for command in ["codemmet.list", "codemmet.context"] {
        let error = client.request_err(
            "workspace/executeCommand",
            &json!({
                "command": command,
                "arguments": [{ "uri": "file:///not/open.rs", "position": { "line": 0, "character": 0 } }],
            }),
        );
        let message = error["message"]
            .as_str()
            .expect("error should have a message");
        assert!(message.contains("document not open"), "message: {message}");
    }
    client.shutdown();
}

#[test]
fn execute_command_preview_keeps_on_incomplete_abbreviation() {
    let (mut client, _) = LspClient::start("preview-keep", &["utf-8"], &json!({}));
    // `forp` is not a trigger: the abbreviation is present but not renderable,
    // so the server returns an empty object (`{}`) — the client keeps its
    // current preview rather than clearing (avoids flicker while typing).
    client.open(URI, "rust", "fn main() {\n    forp\n}\n");
    let result = client.request(
        "workspace/executeCommand",
        &json!({
            "command": "codemmet.preview",
            "arguments": [{ "uri": URI, "position": { "line": 1, "character": 8 } }],
        }),
    );
    assert_eq!(
        result,
        json!({}),
        "expected the keep signal `{{}}`, got {result}"
    );
    client.shutdown();
}
