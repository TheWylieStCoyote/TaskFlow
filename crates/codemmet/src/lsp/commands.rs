//! `workspace/executeCommand`: `codemmet.expand` and `codemmet.wrap`.
//!
//! Both commands *return* `{ range, snippet, isSnippet }` instead of sending
//! `workspace/applyEdit`, because applied edits cannot carry tabstops; the
//! client deletes the range and hands the snippet to its own expander. When
//! the expansion has missing required imports the result also carries
//! `extraEdits`: plain text edits (computed against the pre-apply document)
//! the client applies *after* the snippet — see `lua/codemmet/lsp.lua`.

use codemmet_core::config::loader::scope_name;
use codemmet_core::context::{
    ContextProvider, DocSnapshot, ExpansionContext, IndentUnit, Scope, WrapSelection,
};
use codemmet_core::snippet::render::{TabstopMode, active_variant};
use lsp_types::{Position, Range, TextEdit, Uri};
use serde::Deserialize;

use super::line_index::LineIndex;
use super::{Server, extract};

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

/// Arguments of `codemmet.list` and `codemmet.context`: a document position.
#[derive(Debug, Deserialize)]
pub struct AtArgs {
    /// The document to inspect.
    pub uri: Uri,
    /// The cursor position the context is computed at.
    pub position: Position,
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

    let engine = server.engine_for(&args.uri);
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
    let expansion = engine
        .expand(candidate, &doc.language, &ctx, TabstopMode::Keep)
        .map_err(|err| format!("cannot expand `{candidate}`: {err}"))?;

    let range = Range {
        start: index.position(&doc.text, start, server.encoding),
        end: index.position(&doc.text, offset, server.encoding),
    };
    let extra_edits = import_edits(server, &doc, &index, &expansion.requires, range.start);
    Ok(result(range, &expansion.body, &extra_edits))
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
    let engine = server.engine_for(&args.uri);
    let ctx = context_for(
        server.provider.as_mut(),
        &doc,
        offset,
        args.formatting_options,
    );
    let Ok(expansion) = engine.expand(candidate, &doc.language, &ctx, TabstopMode::Strip) else {
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

    let engine = server.engine_for(&args.uri);
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
    let expansion = engine
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
    let extra_edits = import_edits(server, &doc, &index, &expansion.requires, range.start);
    Ok(result(range, &expansion.body, &extra_edits))
}

/// Run `codemmet.list`: every trigger registered for the document's language,
/// resolved through the per-document engine (so path-mapped and project pack
/// tiers apply to the file being edited), annotated with the context at
/// `position`.
///
/// Each entry carries the keep-mode `body` the bare trigger renders at the
/// cursor — insertable directly by a snippet client — a one-line strip-mode
/// `hint` for menus, and `activeContext`: the variant context the renderer
/// would pick at the cursor, or null for the default template. A trigger that
/// cannot render bare (an argument-less data binding failure) carries null
/// `body`/`hint` but is still listed for discovery.
pub fn list(server: &mut Server, args: &AtArgs) -> Result<serde_json::Value, CommandError> {
    let doc = server.snapshot(&args.uri)?;
    let index = LineIndex::new(&doc.text);
    let offset = index.offset(&doc.text, args.position, server.encoding);
    let engine = server.engine_for(&args.uri);
    let mut ctx = server.provider.context_at(&doc, offset);
    // Bodies go to snippet clients that re-indent continuation lines from the
    // insertion position (see `expand`), so render with an empty base indent.
    ctx.base_indent = String::new();
    let lang = engine
        .registry()
        .language(&doc.language)
        .ok_or_else(|| CommandError(format!("unsupported language `{}`", doc.language)))?;
    // The renderer matches variants against the innermost scope.
    let scope = ctx.scopes.last().copied().unwrap_or(Scope::Root);

    let snippets: Vec<serde_json::Value> = lang
        .set
        .iter()
        .map(|snippet| {
            let body = engine
                .expand(&snippet.trigger, &doc.language, &ctx, TabstopMode::Keep)
                .ok()
                .map(|expansion| expansion.body);
            let hint = engine
                .expand(&snippet.trigger, &doc.language, &ctx, TabstopMode::Strip)
                .ok()
                .map(|expansion| one_line_hint(&expansion.body));
            serde_json::json!({
                "trigger": snippet.trigger,
                "description": snippet.description,
                "composite": snippet.composite.is_some(),
                "activeContext": active_variant(snippet, scope, &ctx.raw_node_kinds)
                    .map(|variant| variant.context.as_str()),
                "body": body,
                "hint": hint,
            })
        })
        .collect();
    Ok(serde_json::json!({ "language": doc.language, "snippets": snippets }))
}

/// Run `codemmet.context`: the expansion context active at `position` — the
/// scope stack (outermost first) and the in-scope collections the provider
/// extracted from the buffer.
///
/// Without the tree-sitter provider (a `--no-default-features` build) the
/// null provider answers: a bare `["root"]` stack and no collections.
pub fn context(server: &mut Server, args: &AtArgs) -> Result<serde_json::Value, CommandError> {
    let doc = server.snapshot(&args.uri)?;
    let index = LineIndex::new(&doc.text);
    let offset = index.offset(&doc.text, args.position, server.encoding);
    let ctx = server.provider.context_at(&doc, offset);

    let scopes: Vec<&str> = ctx.scopes.iter().map(|scope| scope_name(*scope)).collect();
    let collections: Vec<serde_json::Value> = ctx
        .collections
        .iter()
        .map(|(name, collection)| {
            let items: Vec<serde_json::Value> = collection
                .items
                .iter()
                .map(|item| serde_json::json!({ "value": item.value, "members": item.members }))
                .collect();
            serde_json::json!({ "name": name, "items": items })
        })
        .collect();
    Ok(serde_json::json!({
        "language": doc.language,
        "scopes": scopes,
        "inStringOrComment": ctx.in_string_or_comment,
        "collections": collections,
    }))
}

/// One display line summarizing a strip-mode expansion: its first line, with
/// an ellipsis when more follow.
fn one_line_hint(body: &str) -> String {
    body.split_once('\n')
        .map_or_else(|| body.to_owned(), |(first, _)| format!("{first} …"))
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

/// The missing-import edits for an expansion's `requires` (empty when the
/// provider has no scan, everything is satisfied, or the insertion would
/// collide with the main edit's line).
fn import_edits(
    server: &mut Server,
    doc: &DocSnapshot,
    index: &LineIndex,
    requires: &[String],
    main_start: Position,
) -> Vec<TextEdit> {
    if requires.is_empty() {
        return Vec::new();
    }
    let Some(scan) = server.provider.import_scan(doc) else {
        return Vec::new();
    };
    super::requires::import_edit(
        &doc.language,
        requires,
        &scan,
        index,
        &doc.text,
        server.encoding,
        main_start,
    )
    .into_iter()
    .collect()
}

/// The command result shape both commands share. `extraEdits` appears only
/// when non-empty, so clients unaware of it are untouched.
fn result(range: Range, snippet: &str, extra_edits: &[TextEdit]) -> serde_json::Value {
    let mut value = serde_json::json!({
        "range": range,
        "snippet": snippet,
        "isSnippet": true,
    });
    if !extra_edits.is_empty()
        && let Ok(edits) = serde_json::to_value(extra_edits)
    {
        value["extraEdits"] = edits;
    }
    value
}
