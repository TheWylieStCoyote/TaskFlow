//! The LSP server: a synchronous, single-threaded loop over stdio.
//!
//! Architecture per `PLAN.md`: no async runtime — expansion is pure CPU
//! work, so one `for msg in &connection.receiver` loop suffices. Documents
//! use full-text sync; positions cross the wire in UTF-8 when the client
//! offers it (see [`line_index`]), UTF-16 otherwise.

mod commands;
mod completion;
mod extract;
mod line_index;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use codemmet_core::Engine;
use codemmet_core::lang::registry::{LanguageSnippets, Registry};
use lsp_server::{Connection, ErrorCode, Message, Request, Response};
use lsp_types::{
    CompletionOptions, CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, ExecuteCommandOptions, ExecuteCommandParams, InitializeParams,
    PositionEncodingKind, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    Uri,
};
use serde::Deserialize;

use commands::CommandError;
use line_index::{LineIndex, PositionEncoding};

/// Arguments for `codemmet lsp`.
#[derive(Debug, clap::Args)]
pub struct LspArgs {
    /// Accepted for client compatibility; stdio is the only transport.
    #[arg(long)]
    stdio: bool,
}

/// When completion items are offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionMode {
    /// On every keystroke with a candidate (the default).
    Always,
    /// Only once the candidate contains a composition operator.
    OperatorsOnly,
    /// Never.
    Off,
}

/// `initializationOptions` the client may pass.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct InitOptions {
    /// Snippet override directory (defaults to `<config>/codemmet/snippets`
    /// or the CLI's `--config`).
    snippets_dir: Option<PathBuf>,
    /// `"always"` | `"operators-only"` | `"off"`.
    completion: Option<String>,
}

/// One open document, synced with full-text replacement.
#[derive(Debug)]
struct Document {
    /// Full document text.
    text: String,
    /// The client's `languageId` for the document.
    language_id: String,
    /// Version of the last applied change.
    version: i32,
}

/// All server state.
struct Server {
    /// The expansion engine (registry + renderer).
    engine: Engine,
    /// Open documents by URI.
    docs: HashMap<Uri, Document>,
    /// Completion gating from `initializationOptions`.
    completion_mode: CompletionMode,
    /// The negotiated position encoding.
    encoding: PositionEncoding,
}

/// Run `codemmet lsp` over stdio until the client disconnects or exits.
pub fn run(user_dir: Option<&Path>, _args: &LspArgs) -> ExitCode {
    match serve(user_dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The full server lifecycle: handshake, main loop, teardown.
fn serve(default_user_dir: Option<&Path>) -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();

    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(initialize_params)?;
    let options: InitOptions = params
        .initialization_options
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    let encoding = negotiate_encoding(&params.capabilities);

    connection.initialize_finish(
        initialize_id,
        serde_json::json!({
            "capabilities": capabilities(encoding),
            "serverInfo": { "name": "codemmet", "version": env!("CARGO_PKG_VERSION") },
        }),
    )?;

    let user_dir = options.snippets_dir.as_deref().or(default_user_dir);
    let registry = match Registry::with_user_dir(user_dir) {
        Ok(registry) => registry,
        Err(err) => {
            // User overrides are broken; stay usable on builtins alone.
            eprintln!("codemmet: ignoring user snippets: {err}");
            Registry::with_builtins()?
        }
    };
    let mut server = Server {
        engine: Engine::with_registry(registry),
        docs: HashMap::new(),
        completion_mode: options.completion_mode(),
        encoding,
    };

    for msg in &connection.receiver {
        match msg {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                let response = server.handle_request(request);
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notification) => {
                server.handle_notification(&notification.method, notification.params);
            }
            Message::Response(_) => {}
        }
    }

    drop(connection);
    io_threads.join()?;
    Ok(())
}

impl InitOptions {
    /// The completion gate the options select; unknown values mean "always".
    fn completion_mode(&self) -> CompletionMode {
        match self.completion.as_deref() {
            Some("off") => CompletionMode::Off,
            Some("operators-only") => CompletionMode::OperatorsOnly,
            _ => CompletionMode::Always,
        }
    }
}

/// UTF-8 when the client advertises support for it, UTF-16 otherwise.
fn negotiate_encoding(capabilities: &lsp_types::ClientCapabilities) -> PositionEncoding {
    let offers_utf8 = capabilities
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_ref())
        .is_some_and(|encodings| encodings.contains(&PositionEncodingKind::UTF8));
    if offers_utf8 {
        PositionEncoding::Utf8
    } else {
        PositionEncoding::Utf16
    }
}

/// The server capabilities advertised at initialize time.
fn capabilities(encoding: PositionEncoding) -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(match encoding {
            PositionEncoding::Utf8 => PositionEncodingKind::UTF8,
            PositionEncoding::Utf16 => PositionEncodingKind::UTF16,
        }),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(
                [">", "+", ":", ","]
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            ),
            ..CompletionOptions::default()
        }),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec!["codemmet.expand".to_owned(), "codemmet.wrap".to_owned()],
            ..ExecuteCommandOptions::default()
        }),
        ..ServerCapabilities::default()
    }
}

impl Server {
    /// Dispatch one client request to its handler.
    fn handle_request(&self, request: Request) -> Response {
        match request.method.as_str() {
            "textDocument/completion" => self.handle_completion(request),
            "workspace/executeCommand" => self.handle_execute_command(request),
            _ => Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                format!("unhandled method `{}`", request.method),
            ),
        }
    }

    /// `textDocument/completion`: items for the abbreviation at the cursor.
    fn handle_completion(&self, request: Request) -> Response {
        let params: CompletionParams = match serde_json::from_value(request.params) {
            Ok(params) => params,
            Err(err) => return invalid_params(request.id, &err),
        };
        let position = params.text_document_position;
        let Some(doc) = self.docs.get(&position.text_document.uri) else {
            // An unopened document is not an error; there is just nothing
            // to complete.
            return Response::new_ok(request.id, serde_json::Value::Null);
        };
        let index = LineIndex::new(&doc.text);
        let offset = index.offset(&doc.text, position.position, self.encoding);
        let items = completion::compute(
            &self.engine,
            &doc.text,
            &doc.language_id,
            offset,
            self.completion_mode,
            self.encoding,
        );
        match serde_json::to_value(items) {
            Ok(value) => Response::new_ok(request.id, value),
            Err(err) => internal_error(request.id, &err),
        }
    }

    /// `workspace/executeCommand`: `codemmet.expand` / `codemmet.wrap`.
    fn handle_execute_command(&self, request: Request) -> Response {
        let params: ExecuteCommandParams = match serde_json::from_value(request.params) {
            Ok(params) => params,
            Err(err) => return invalid_params(request.id, &err),
        };
        let result = match params.command.as_str() {
            "codemmet.expand" => {
                command_args(params.arguments).and_then(|args| commands::expand(self, &args))
            }
            "codemmet.wrap" => {
                command_args(params.arguments).and_then(|args| commands::wrap(self, &args))
            }
            other => Err(CommandError(format!("unknown command `{other}`"))),
        };
        match result {
            Ok(value) => Response::new_ok(request.id, value),
            Err(CommandError(message)) => {
                Response::new_err(request.id, ErrorCode::RequestFailed as i32, message)
            }
        }
    }

    /// Apply one client notification to the document store.
    fn handle_notification(&mut self, method: &str, params: serde_json::Value) {
        match method {
            "textDocument/didOpen" => {
                let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) else {
                    return;
                };
                let doc = params.text_document;
                self.docs.insert(
                    doc.uri,
                    Document {
                        text: doc.text,
                        language_id: doc.language_id,
                        version: doc.version,
                    },
                );
            }
            "textDocument/didChange" => {
                let Ok(mut params) = serde_json::from_value::<DidChangeTextDocumentParams>(params)
                else {
                    return;
                };
                let Some(doc) = self.docs.get_mut(&params.text_document.uri) else {
                    return;
                };
                // Full sync: the last change carries the whole new text.
                // Stale (out-of-order) versions are dropped.
                if params.text_document.version >= doc.version
                    && let Some(change) = params.content_changes.pop()
                {
                    doc.text = change.text;
                    doc.version = params.text_document.version;
                }
            }
            "textDocument/didClose" => {
                if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
                    self.docs.remove(&params.text_document.uri);
                }
            }
            _ => {}
        }
    }

    /// The open document at `uri`, or a command error naming it.
    fn document(&self, uri: &Uri) -> Result<&Document, CommandError> {
        self.docs
            .get(uri)
            .ok_or_else(|| CommandError(format!("document not open: {}", uri.as_str())))
    }

    /// The language for a document's `languageId`, or a command error.
    fn language(&self, language_id: &str) -> Result<&LanguageSnippets, CommandError> {
        self.engine
            .registry()
            .language(language_id)
            .ok_or_else(|| CommandError(format!("unsupported language `{language_id}`")))
    }
}

/// Deserialize the single object argument LSP commands carry.
fn command_args<T: serde::de::DeserializeOwned>(
    arguments: Vec<serde_json::Value>,
) -> Result<T, CommandError> {
    let mut arguments = arguments;
    if arguments.len() != 1 {
        return Err(CommandError(format!(
            "expected exactly one argument object, got {}",
            arguments.len()
        )));
    }
    let argument = arguments.remove(0);
    serde_json::from_value(argument)
        .map_err(|err| CommandError(format!("invalid command arguments: {err}")))
}

/// An `InvalidParams` error response.
fn invalid_params(id: lsp_server::RequestId, err: &dyn std::fmt::Display) -> Response {
    Response::new_err(id, ErrorCode::InvalidParams as i32, err.to_string())
}

/// An `InternalError` error response.
fn internal_error(id: lsp_server::RequestId, err: &dyn std::fmt::Display) -> Response {
    Response::new_err(id, ErrorCode::InternalError as i32, err.to_string())
}
