//! The LSP server: a synchronous, single-threaded loop over stdio.
//!
//! Architecture per `PLAN.md`: no async runtime — expansion is pure CPU
//! work, so one `for msg in &connection.receiver` loop suffices. Documents
//! use incremental sync; positions cross the wire in UTF-8 when the client
//! offers it (see [`line_index`]), UTF-16 otherwise. Syntax awareness comes
//! from a [`ContextProvider`]: tree-sitter-backed under the default
//! `tree-sitter` feature, the scope-blind null provider otherwise.

mod commands;
mod completion;
mod extract;
mod line_index;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use codemmet_core::Engine;
use codemmet_core::context::{ContextProvider, DocSnapshot};
use codemmet_core::lang::registry::{LanguageSnippets, Registry, find_project_snippets_dir};
#[cfg(feature = "tree-sitter")]
use codemmet_tsctx::TreeSitterProvider;
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
    /// Opt in to project-local packs (`.codemmet/snippets` under the workspace
    /// root). Off by default: the editor may open untrusted repositories, and a
    /// project pack can silently redefine what abbreviations expand to.
    project_snippets: Option<bool>,
}

/// One open document, kept current via incremental sync.
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
    /// Syntax awareness for expansion points (`context_at` needs `&mut`
    /// for its tree cache, so the server owns the provider mutably).
    provider: Box<dyn ContextProvider>,
}

/// Convert a `file:` URI string to a filesystem path.
///
/// Handles the common `file:///path` (empty authority) and `file://host/path`
/// forms and percent-decodes the path. Returns `None` for non-`file` URIs.
fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // Everything from the first '/' is the path; anything before it is an
    // authority (host) we don't need for a local path.
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(start) => rest.get(start..)?,
        None => return None,
    };
    Some(PathBuf::from(percent_decode(path)))
}

/// Percent-decode a URI path component (`%20` → space, etc.).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'%'
            && let (Some(hi), Some(lo)) = (
                bytes.get(i + 1).copied().and_then(hex_nibble),
                bytes.get(i + 2).copied().and_then(hex_nibble),
            )
        {
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(byte);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A hex digit's value, or `None` if `byte` is not `[0-9a-fA-F]`.
const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// The best context provider this build carries: tree-sitter under the
/// default `tree-sitter` feature, the scope-blind null provider otherwise.
fn default_provider() -> Box<dyn ContextProvider> {
    #[cfg(feature = "tree-sitter")]
    {
        Box::new(TreeSitterProvider::new())
    }
    #[cfg(not(feature = "tree-sitter"))]
    {
        Box::new(codemmet_core::context::NullContextProvider)
    }
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
    // Project packs are opt-in: only when `project_snippets` is set do we look
    // for `.codemmet/snippets` under the workspace root the client reported
    // (a workspace folder, else the deprecated `rootUri`).
    // `root_uri` is deprecated in favour of `workspace_folders`, but many
    // clients still send only the former, so we accept it as a fallback.
    #[allow(deprecated)]
    let project_dir = if options.project_snippets.unwrap_or(false) {
        params
            .workspace_folders
            .as_deref()
            .and_then(|folders| folders.first())
            .map(|folder| folder.uri.as_str())
            .or_else(|| params.root_uri.as_ref().map(|uri| uri.as_str()))
            .and_then(file_uri_to_path)
            .and_then(|root| find_project_snippets_dir(&root))
    } else {
        None
    };
    let registry = match Registry::with_dirs(user_dir, project_dir.as_deref()) {
        Ok(registry) => registry,
        Err(err) => {
            // Overrides are broken; stay usable on builtins alone.
            eprintln!("codemmet: ignoring user snippets: {err}");
            Registry::with_builtins()?
        }
    };
    let mut server = Server {
        engine: Engine::with_registry(registry),
        docs: HashMap::new(),
        completion_mode: options.completion_mode(),
        encoding,
        provider: default_provider(),
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
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
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
            commands: vec![
                "codemmet.expand".to_owned(),
                "codemmet.wrap".to_owned(),
                "codemmet.preview".to_owned(),
            ],
            ..ExecuteCommandOptions::default()
        }),
        ..ServerCapabilities::default()
    }
}

impl Server {
    /// Dispatch one client request to its handler.
    fn handle_request(&mut self, request: Request) -> Response {
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
    fn handle_completion(&mut self, request: Request) -> Response {
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
            self.provider.as_mut(),
            &index,
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
    fn handle_execute_command(&mut self, request: Request) -> Response {
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
            "codemmet.preview" => {
                command_args(params.arguments).map(|args| commands::preview(self, &args))
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
                let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params)
                else {
                    return;
                };
                let Some(doc) = self.docs.get_mut(&params.text_document.uri) else {
                    return;
                };
                // Stale (out-of-order) versions are dropped.
                if params.text_document.version < doc.version {
                    return;
                }
                for change in params.content_changes {
                    if let Some(range) = change.range {
                        // Each range refers to the document as already
                        // modified by the preceding changes in this batch,
                        // so re-index before every splice.
                        let index = LineIndex::new(&doc.text);
                        let start = index.offset(&doc.text, range.start, self.encoding);
                        let end = index.offset(&doc.text, range.end, self.encoding).max(start);
                        doc.text.replace_range(start..end, &change.text);
                    } else {
                        // A rangeless change replaces the whole document.
                        doc.text = change.text;
                    }
                }
                doc.version = params.text_document.version;
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

    /// An owned snapshot of the document at `uri` for the context provider,
    /// carrying the canonical profile id (not the raw client `languageId`).
    fn snapshot(&self, uri: &Uri) -> Result<DocSnapshot, CommandError> {
        let doc = self.document(uri)?;
        let lang = self.language(&doc.language_id)?;
        Ok(DocSnapshot {
            language: lang.profile.id.to_owned(),
            text: doc.text.clone(),
        })
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
