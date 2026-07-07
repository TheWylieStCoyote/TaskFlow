//! `workspace/executeCommand`: `codemmet.expand` and `codemmet.wrap`.
//!
//! Both commands *return* `{ range, snippet, isSnippet }` instead of sending
//! `workspace/applyEdit`, because applied edits cannot carry tabstops; the
//! client deletes the range and hands the snippet to its own expander.

use codemmet_core::config::writer::{SnippetEntry, upsert_snippet};
use codemmet_core::context::{
    ContextProvider, DocSnapshot, ExpansionContext, IndentUnit, WrapSelection,
};
use codemmet_core::lang::registry::resolve_capture_dir;
use codemmet_core::snippet::capture::{CaptureOptions, capture as capture_code};
use codemmet_core::snippet::render::TabstopMode;
use lsp_types::{Position, Range, Uri};
use serde::Deserialize;

use super::line_index::LineIndex;
use super::{Server, extract, file_uri_to_path};

/// A command failure: the human-readable message for a JSON-RPC error.
#[derive(Debug)]
pub struct CommandError(pub String);

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self(message)
    }
}

/// Editor formatting preferences passed alongside a command.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FormattingOptions {
    /// Spaces per indent level when `insert_spaces` is set.
    #[serde(alias = "tabSize")]
    pub tab_size: u32,
    /// Indent with spaces rather than tabs.
    #[serde(alias = "insertSpaces")]
    pub insert_spaces: bool,
}

impl FormattingOptions {
    /// The indent unit these options describe.
    fn indent_unit(self) -> IndentUnit {
        if self.insert_spaces {
            IndentUnit::Spaces(u8::try_from(self.tab_size).unwrap_or(u8::MAX))
        } else {
            IndentUnit::Tab
        }
    }
}

/// Arguments of `codemmet.expand`.
#[derive(Debug, Deserialize)]
pub struct ExpandArgs {
    /// The document to expand in.
    pub uri: Uri,
    /// The cursor position; the abbreviation ends here.
    pub position: Position,
    /// Editor indent preferences; detected from the line when absent.
    #[serde(default, alias = "formattingOptions")]
    pub formatting_options: Option<FormattingOptions>,
}

/// Arguments of `codemmet.wrap`.
#[derive(Debug, Deserialize)]
pub struct WrapArgs {
    /// The document the selection lives in.
    pub uri: Uri,
    /// The selection to wrap.
    pub range: Range,
    /// The abbreviation to wrap it with.
    pub abbreviation: String,
    /// Editor indent preferences; detected from the line when absent.
    #[serde(default, alias = "formattingOptions")]
    pub formatting_options: Option<FormattingOptions>,
}

/// Arguments of `codemmet.capture`.
#[derive(Debug, Deserialize)]
pub struct CaptureArgs {
    /// The document the selection lives in.
    pub uri: Uri,
    /// The selection to generalize into a snippet.
    pub range: Range,
    /// The trigger name to register the captured snippet under.
    pub name: String,
    /// Optional human-readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// Identifiers to force into `${param}`s even if they occur once.
    #[serde(default)]
    pub params: Vec<String>,
    /// Identifiers to keep literal even if they repeat.
    #[serde(default)]
    pub literals: Vec<String>,
    /// Overwrite an existing trigger of the same name.
    #[serde(default)]
    pub force: bool,
}

/// Run `codemmet.expand`: expand the abbreviation left of `position`.
pub fn expand(server: &mut Server, args: &ExpandArgs) -> Result<serde_json::Value, CommandError> {
    let doc = server.snapshot(&args.uri)?;
    let index = LineIndex::new(&doc.text);
    let offset = index.offset(&doc.text, args.position, server.encoding);
    let (start, candidate) = extract::candidate_at(&doc.text, offset).ok_or_else(|| {
        format!(
            "no abbreviation at {}:{}",
            args.position.line, args.position.character
        )
    })?;

    let mut ctx = context_for(
        server.provider.as_mut(),
        &doc,
        offset,
        args.formatting_options,
    );
    // LSP snippet clients (VS Code, Neovim's `vim.snippet`) re-indent snippet
    // continuation lines from the insertion position, so a server-side base
    // indent would be applied twice. Render with an empty base indent;
    // nesting depth still uses the provider's indent unit.
    ctx.base_indent = String::new();
    let expansion = server
        .engine
        .expand(candidate, &doc.language, &ctx, TabstopMode::Keep)
        .map_err(|err| format!("cannot expand `{candidate}`: {err}"))?;

    let range = Range {
        start: index.position(&doc.text, start, server.encoding),
        end: index.position(&doc.text, offset, server.encoding),
    };
    Ok(result(range, &expansion.body))
}

/// Run `codemmet.preview`: the strip-mode expansion of the abbreviation left
/// of `position`, for a non-committing live preview (e.g. ghost virtual text).
///
/// Three outcomes, so the client can avoid flicker while you type through
/// half-finished abbreviations:
///
/// - `{ range, text }` — a renderable abbreviation; show this.
/// - `{}` — an abbreviation is present but not renderable *yet* (a dangling
///   operator like `fori10>`, an empty argument like `fn:`, an unresolved
///   child): the client should KEEP its current preview rather than clear.
/// - `null` — no abbreviation before the cursor at all; the client clears.
///
/// Unlike [`expand`], the base indent is kept: a preview is display-only
/// (never inserted), so it should show accurately indented code.
pub fn preview(server: &mut Server, args: &ExpandArgs) -> serde_json::Value {
    let Ok(doc) = server.snapshot(&args.uri) else {
        return serde_json::Value::Null;
    };
    let index = LineIndex::new(&doc.text);
    let offset = index.offset(&doc.text, args.position, server.encoding);
    let Some((start, candidate)) = extract::candidate_at(&doc.text, offset) else {
        return serde_json::Value::Null;
    };
    let ctx = context_for(
        server.provider.as_mut(),
        &doc,
        offset,
        args.formatting_options,
    );
    let Ok(expansion) = server
        .engine
        .expand(candidate, &doc.language, &ctx, TabstopMode::Strip)
    else {
        // A candidate is present but not renderable mid-typing: signal "keep".
        return serde_json::json!({});
    };
    let range = Range {
        start: index.position(&doc.text, start, server.encoding),
        end: index.position(&doc.text, offset, server.encoding),
    };
    serde_json::json!({ "range": range, "text": expansion.body })
}

/// Run `codemmet.wrap`: wrap the selected text in an expanded abbreviation.
///
/// The provider validates the selection first: it may snap the range
/// outward to statement/line boundaries (the *snapped* range is wrapped)
/// or reject it (cross-scope selections), which surfaces as a JSON-RPC
/// error. The returned range starts at the snapped region's first
/// non-whitespace byte, so the client deletes only from the indent onward:
/// the original leading whitespace survives in the buffer and the snippet
/// client re-indents continuation lines from that position.
pub fn wrap(server: &mut Server, args: &WrapArgs) -> Result<serde_json::Value, CommandError> {
    let doc = server.snapshot(&args.uri)?;
    let index = LineIndex::new(&doc.text);
    let start = index.offset(&doc.text, args.range.start, server.encoding);
    let end = index
        .offset(&doc.text, args.range.end, server.encoding)
        .max(start);
    let snapped = match server.provider.validate_wrap(&doc, start..end) {
        WrapSelection::Valid { range } => range,
        WrapSelection::Invalid { reason } => {
            return Err(CommandError(format!("cannot wrap selection: {reason}")));
        }
    };
    // The full snapped text, leading whitespace included: the renderer
    // dedents the selection's common indent itself.
    let selection = &doc.text[snapped.clone()];

    let mut ctx = context_for(
        server.provider.as_mut(),
        &doc,
        snapped.start,
        args.formatting_options,
    );
    // LSP snippet clients (VS Code, Neovim's `vim.snippet`) re-indent snippet
    // continuation lines from the insertion position, so a server-side base
    // indent would be applied twice. Render with an empty base indent;
    // nesting depth still uses the provider's indent unit.
    ctx.base_indent = String::new();
    let expansion = server
        .engine
        .expand_wrap(
            &args.abbreviation,
            &doc.language,
            &ctx,
            TabstopMode::Keep,
            selection,
        )
        .map_err(|err| format!("cannot wrap with `{}`: {err}", args.abbreviation))?;

    // Advance past the first line's leading whitespace so the replacement
    // starts where the wrapped construct should sit.
    let content_start = snapped.start
        + doc.text[snapped.clone()]
            .bytes()
            .take_while(|&b| b == b' ' || b == b'\t')
            .count();
    let range = Range {
        start: index.position(&doc.text, content_start, server.encoding),
        end: index.position(&doc.text, snapped.end, server.encoding),
    };
    Ok(result(range, &expansion.body))
}

/// Run `codemmet.capture`: generalize the selected code into a reusable
/// snippet and write it into the project pack, then reload the registry so the
/// new trigger is immediately expandable in the same session.
///
/// Unlike [`expand`]/[`wrap`], this command *writes* — but only the file the
/// user explicitly asked for, and only inert snippet text. The target is the
/// `.codemmet/snippets` pack at or above the document's directory (created if
/// absent), resolved via
/// [`resolve_capture_dir`](codemmet_core::lang::registry::resolve_capture_dir).
///
/// Returns `{ path, trigger, params }` for the client to report.
pub fn capture(server: &mut Server, args: &CaptureArgs) -> Result<serde_json::Value, CommandError> {
    let doc = server.snapshot(&args.uri)?;
    let index = LineIndex::new(&doc.text);
    let start = index.offset(&doc.text, args.range.start, server.encoding);
    let end = index
        .offset(&doc.text, args.range.end, server.encoding)
        .max(start);
    let selection = &doc.text[start..end];
    if selection.trim().is_empty() {
        return Err(CommandError(
            "nothing to capture: the selection is empty".to_owned(),
        ));
    }

    // The target directory is derived from the document's own path.
    let file_path = file_uri_to_path(args.uri.as_str()).ok_or_else(|| {
        CommandError(format!(
            "cannot capture into a non-file URI: {}",
            args.uri.as_str()
        ))
    })?;
    let dir = resolve_capture_dir(file_path.parent().unwrap_or(&file_path));

    let options = CaptureOptions {
        force_params: args.params.clone(),
        force_literals: args.literals.clone(),
    };
    let captured = capture_code(&doc.language, selection, &options);
    let entry = SnippetEntry {
        trigger: args.name.clone(),
        description: args.description.clone(),
        params: captured.params,
        body: captured.body,
    };
    let param_names: Vec<String> = entry.params.iter().map(|p| p.name.clone()).collect();

    let path = upsert_snippet(&dir, &doc.language, &entry, args.force)
        .map_err(|err| CommandError(format!("cannot capture `{}`: {err}", args.name)))?;

    // Make the captured trigger usable now: adopt the pack dir as the project
    // override and rebuild the registry.
    server.project_dir = Some(dir);
    server
        .reload_registry()
        .map_err(|err| CommandError(format!("captured, but reloading snippets failed: {err}")))?;

    Ok(serde_json::json!({
        "path": path.display().to_string(),
        "trigger": args.name,
        "params": param_names,
    }))
}

/// Ask the provider for the context at `offset`; the editor's explicit
/// `formatting_options` override the detected indent unit (scopes and base
/// indent stay provider-derived).
fn context_for(
    provider: &mut dyn ContextProvider,
    doc: &DocSnapshot,
    offset: usize,
    formatting_options: Option<FormattingOptions>,
) -> ExpansionContext {
    let mut ctx = provider.context_at(doc, offset);
    if let Some(options) = formatting_options {
        ctx.indent_unit = options.indent_unit();
    }
    ctx
}

/// The command result shape both commands share.
fn result(range: Range, snippet: &str) -> serde_json::Value {
    serde_json::json!({
        "range": range,
        "snippet": snippet,
        "isSnippet": true,
    })
}
