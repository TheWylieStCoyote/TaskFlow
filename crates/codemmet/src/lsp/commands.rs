//! `workspace/executeCommand`: `codemmet.expand` and `codemmet.wrap`.
//!
//! Both commands *return* `{ range, snippet, isSnippet }` instead of sending
//! `workspace/applyEdit`, because applied edits cannot carry tabstops; the
//! client deletes the range and hands the snippet to its own expander.

use codemmet_core::context::{ContextProvider, DocSnapshot, ExpansionContext, IndentUnit};
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

    let ctx = context_for(
        server.provider.as_mut(),
        &doc,
        offset,
        args.formatting_options,
    );
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

/// Run `codemmet.wrap`: wrap the selected text in an expanded abbreviation.
pub fn wrap(server: &mut Server, args: &WrapArgs) -> Result<serde_json::Value, CommandError> {
    let doc = server.snapshot(&args.uri)?;
    let index = LineIndex::new(&doc.text);
    let start = index.offset(&doc.text, args.range.start, server.encoding);
    let end = index
        .offset(&doc.text, args.range.end, server.encoding)
        .max(start);
    let selection = &doc.text[start..end];

    let ctx = context_for(
        server.provider.as_mut(),
        &doc,
        start,
        args.formatting_options,
    );
    let snippet = crate::wrap::wrap(
        &server.engine,
        &doc.language,
        &args.abbreviation,
        selection,
        &ctx,
    )
    .map_err(|err| format!("cannot wrap with `{}`: {err}", args.abbreviation))?;

    Ok(result(args.range, &snippet))
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
