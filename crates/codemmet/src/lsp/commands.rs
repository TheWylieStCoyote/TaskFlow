//! `workspace/executeCommand`: `codemmet.expand` and `codemmet.wrap`.
//!
//! Both commands *return* `{ range, snippet, isSnippet }` instead of sending
//! `workspace/applyEdit`, because applied edits cannot carry tabstops; the
//! client deletes the range and hands the snippet to its own expander.

use codemmet_core::context::{
    ContextProvider, DocSnapshot, ExpansionContext, IndentUnit, WrapSelection,
};
use codemmet_core::snippet::render::TabstopMode;
use lsp_types::{Position, Range, Uri};
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
/// Returns `{ range, text }` where `range` covers the abbreviation and `text`
/// is the clean expansion, or JSON `null` when there is no renderable
/// abbreviation there — the client just clears its preview, rather than seeing
/// an error. Unlike [`expand`], the base indent is kept: a preview is
/// display-only (never inserted), so it should show accurately indented code.
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
        return serde_json::Value::Null;
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
